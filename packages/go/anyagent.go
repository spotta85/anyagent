// Package anyagent is the anyagent binary as a Go API: spawn `anyagent serve`,
// write command lines, route reply and event lines. Every rule lives in the
// binary; this file is a pipe (ticket 13, W1–W10).
package anyagent

import (
	"bufio"
	"encoding/json"
	"fmt"
	"io"
	"iter"
	"os"
	"os/exec"
	"sync"
	"time"
)

// ---------------------------------------------------------------------------
// PUBLIC TYPES
// ---------------------------------------------------------------------------

// Options for Start.
type Options struct {
	Bin  string   // path to the binary; default ANYAGENT_BIN, then PATH
	Mock string   // a mock script (packages/mock-scripts/*.json): no real agents
	Env  []string // environment for the binary and the agents it spawns; default this process's
}

// OpenOptions is what Open and Generate accept besides the agent: the `open` command's fields.
type OpenOptions struct {
	Dir            string                 `json:"dir"`
	Resume         string                 `json:"resume,omitempty"`
	Fork           string                 `json:"fork,omitempty"`
	ForkAt         string                 `json:"fork_at,omitempty"`
	PermissionMode PermissionMode         `json:"permission_mode,omitempty"`
	McpServers     []McpServer            `json:"mcp_servers,omitempty"`
	Configure      map[string]ConfigValue `json:"configure,omitempty"`
}

// Error is `kind`, `message`, and every extra field from the wire in Data (W8).
type Error struct {
	Kind    string
	Message string
	Data    map[string]any
}

func (e *Error) Error() string { return e.Kind + ": " + e.Message }

func (e *Error) UnmarshalJSON(b []byte) error {
	var body map[string]any
	if err := json.Unmarshal(b, &body); err != nil {
		return err
	}
	e.Kind, _ = body["kind"].(string)
	e.Message, _ = body["message"].(string)
	delete(body, "kind")
	delete(body, "message")
	e.Data = body
	return nil
}

// ---------------------------------------------------------------------------
// RUNTIME: one `anyagent serve` process
// ---------------------------------------------------------------------------

// The wire protocol this package speaks; the binary's hello must match.
const protocol = 1

// Longest stdout line accepted; a tool result carrying a big diff is one line.
const lineLimit = 64 << 20

// One line from the binary, untagged: the fields present say what it is.
type line struct {
	Hello *struct {
		Protocol int `json:"protocol"`
	} `json:"hello"`
	ID      *uint64         `json:"id"`
	OK      json.RawMessage `json:"ok"`
	Error   *Error          `json:"error"`
	Event   *Event          `json:"event"`
	Session string          `json:"session"`
	Closed  string          `json:"closed"`
}

type pending struct {
	ch    chan reply
	opens bool
}

type reply struct {
	ok      json.RawMessage
	session *Session
	err     error
}

// Runtime is one `anyagent serve` process. Build it with Start.
type Runtime struct {
	cmd      *exec.Cmd
	stdin    io.WriteCloser
	wmu      sync.Mutex // one frame at a time on stdin
	mu       sync.Mutex // everything below
	next     uint64
	pending  map[uint64]pending
	sessions map[string]*Session
	dead     error
	helloed  bool
	hello    chan error
	exited   chan struct{} // closed by onExit, after code is set
	code     int
}

// Start spawns the binary; returns after its hello line.
func Start(opts Options) (*Runtime, error) {
	bin, err := resolveBinary(opts.Bin)
	if err != nil {
		return nil, err
	}
	args := []string{"serve"}
	if opts.Mock != "" {
		args = append(args, "--mock", opts.Mock)
	}
	rt := &Runtime{
		cmd: exec.Command(bin, args...), next: 1, pending: map[uint64]pending{}, sessions: map[string]*Session{},
		hello: make(chan error, 1), exited: make(chan struct{}),
	}
	rt.cmd.Env = opts.Env
	rt.cmd.Stderr = os.Stderr
	stdout, err := rt.cmd.StdoutPipe()
	if err != nil {
		return nil, err
	}
	if rt.stdin, err = rt.cmd.StdinPipe(); err != nil {
		return nil, err
	}
	if err := rt.cmd.Start(); err != nil {
		return nil, err
	}
	go rt.read(stdout) // sends the hello, or the exit error (W4)
	if err := <-rt.hello; err != nil {
		return nil, err
	}
	return rt, nil
}

func (rt *Runtime) Discover() (DiscoveryReport, error) {
	var report DiscoveryReport
	return report, rt.call(map[string]any{"cmd": "discover"}, &report)
}

// Probe: agent is a catalog id like "claude", or an AgentRef with Acp set.
func (rt *Runtime) Probe(agent any) (AgentDetails, error) {
	var details AgentDetails
	return details, rt.call(map[string]any{"cmd": "probe", "agent": agent}, &details)
}

func (rt *Runtime) PlanUsage(agent any) (PlanUsage, error) {
	var usage PlanUsage
	return usage, rt.call(map[string]any{"cmd": "plan_usage", "agent": agent}, &usage)
}

// Generate is one-shot text with no session to manage: titles, commit messages.
func (rt *Runtime) Generate(agent any, opts OpenOptions, prompt string) (string, error) {
	var text string
	cmd := fields(opts)
	cmd["cmd"], cmd["agent"], cmd["prompt"] = "generate", agent, prompt
	return text, rt.call(cmd, &text)
}

// Open opens a session. The Session is registered inside onLine (W1).
func (rt *Runtime) Open(agent any, opts OpenOptions) (*Session, error) {
	cmd := fields(opts)
	cmd["cmd"], cmd["agent"] = "open", agent
	r, err := rt.send(cmd, true)
	if err != nil {
		return nil, err
	}
	return r.session, nil
}

// Close is graceful and idempotent (W5): close stdin, wait up to 5 s, then kill. Returns the exit code.
func (rt *Runtime) Close() int {
	rt.mu.Lock()
	dead := rt.dead
	rt.mu.Unlock()
	if dead == nil {
		rt.stdin.Close()
		select {
		case <-rt.exited:
		case <-time.After(5 * time.Second):
			rt.cmd.Process.Kill()
		}
	}
	<-rt.exited
	return rt.code
}

// Writes one command line; the reply's `ok` is unmarshaled into out (nil to ignore it).
func (rt *Runtime) call(cmd map[string]any, out any) error {
	r, err := rt.send(cmd, false)
	if err != nil || out == nil || len(r.ok) == 0 || string(r.ok) == "null" {
		return err
	}
	return json.Unmarshal(r.ok, out)
}

// Registers the pending reply, then writes the frame. W4: a dead runtime fails at once.
func (rt *Runtime) send(cmd map[string]any, opens bool) (reply, error) {
	rt.mu.Lock()
	if rt.dead != nil {
		rt.mu.Unlock()
		return reply{}, rt.dead
	}
	id := rt.next
	rt.next++
	cmd["id"] = id
	ch := make(chan reply, 1)
	rt.pending[id] = pending{ch, opens}
	rt.mu.Unlock()
	data, err := json.Marshal(cmd)
	if err != nil {
		return reply{}, err
	}
	rt.write(data)
	r := <-ch
	return r, r.err
}

// One line on stdin. A write after death fails; onExit reports it.
func (rt *Runtime) write(line []byte) {
	rt.wmu.Lock()
	defer rt.wmu.Unlock()
	rt.stdin.Write(append(line, '\n'))
}

// The reader goroutine: routes every stdout line, then reports the exit.
func (rt *Runtime) read(stdout io.Reader) {
	sc := bufio.NewScanner(stdout)
	sc.Buffer(make([]byte, 64<<10), lineLimit)
	for sc.Scan() {
		rt.onLine(append([]byte(nil), sc.Bytes()...)) // a copy: a reply's RawMessage outlives the scanner's buffer
	}
	if err := sc.Err(); err != nil {
		rt.mu.Lock()
		rt.abort(err.Error())
		rt.mu.Unlock()
	}
	code := 0
	if err := rt.cmd.Wait(); err != nil {
		code = -1
		if exit, ok := err.(*exec.ExitError); ok {
			code = exit.ExitCode()
		}
	}
	rt.onExit(code)
}

// Routes one stdout line. Registers a session before the next line is read (W1).
func (rt *Runtime) onLine(raw []byte) {
	rt.mu.Lock()
	defer rt.mu.Unlock()
	if rt.dead != nil {
		return
	}
	var l line
	if err := json.Unmarshal(raw, &l); err != nil {
		rt.abort("not a frame: " + string(raw))
		return
	}
	switch {
	case l.Hello != nil:
		if l.Hello.Protocol != protocol {
			rt.abort(fmt.Sprintf("protocol %d, this package speaks %d", l.Hello.Protocol, protocol))
		} else if !rt.helloed {
			rt.helloed = true
			rt.hello <- nil
		}
	case l.ID != nil:
		p, ok := rt.pending[*l.ID]
		if !ok {
			return
		}
		delete(rt.pending, *l.ID)
		switch {
		case l.Error != nil:
			p.ch <- reply{err: l.Error}
		case p.opens:
			s, err := rt.register(l.OK)
			p.ch <- reply{session: s, err: err}
		default:
			p.ch <- reply{ok: l.OK}
		}
	case l.Event != nil:
		if s := rt.sessions[l.Event.SessionID]; s != nil {
			s.push(l.Event)
		}
	case l.Session != "" && l.Error != nil: // W3
		if s := rt.sessions[l.Session]; s != nil {
			s.fail(l.Error)
		}
	case l.Closed != "":
		if s := rt.sessions[l.Closed]; s != nil {
			delete(rt.sessions, l.Closed)
			s.end()
		}
	}
}

// The Session for an `open` reply, registered before the reply is delivered.
func (rt *Runtime) register(ok json.RawMessage) (*Session, error) {
	var info SessionInfo
	if err := json.Unmarshal(ok, &info); err != nil {
		return nil, err
	}
	s := &Session{ID: info.ID, rt: rt, info: info, status: SessionStatusIdle, queue: make(chan *Event, queueCap)}
	if info.Status != nil {
		s.status = *info.Status
	}
	rt.sessions[info.ID] = s
	return s, nil
}

// A binary that does not speak the protocol (W10): kill it; onExit fails the rest. Called with mu held.
func (rt *Runtime) abort(why string) {
	rt.dead = &Error{Kind: "ProtocolFailed", Message: why}
	rt.cmd.Process.Kill()
}

// Process gone: fail everything still waiting (W4).
func (rt *Runtime) onExit(code int) {
	rt.mu.Lock()
	defer rt.mu.Unlock()
	if rt.dead == nil {
		rt.dead = &Error{Kind: "ProcessExited", Message: fmt.Sprintf("anyagent exited (%d)", code), Data: map[string]any{"status": fmt.Sprint(code), "stderr": ""}}
	}
	if !rt.helloed {
		rt.helloed = true
		rt.hello <- rt.dead
	}
	for id, p := range rt.pending {
		p.ch <- reply{err: rt.dead}
		delete(rt.pending, id)
	}
	for id, s := range rt.sessions {
		s.fail(rt.dead)
		delete(rt.sessions, id)
	}
	rt.code = code
	close(rt.exited)
}

// ---------------------------------------------------------------------------
// SESSION: one open session
// ---------------------------------------------------------------------------

// Unread events a session may hold before it is closed as lagging (W6).
const queueCap = 4096

// Session is one open session: commands in, an ordered event stream out.
type Session struct {
	ID     string
	rt     *Runtime
	mu     sync.Mutex
	info   SessionInfo
	status SessionStatus
	queue  chan *Event // events; closed after the error (if any) is set
	err    error
	ended  bool
}

// Info is live: replaced on every SessionUpdated (W2).
func (s *Session) Info() SessionInfo {
	s.mu.Lock()
	defer s.mu.Unlock()
	return s.info
}

// Status is live: replaced on every StatusChanged (W2).
func (s *Session) Status() SessionStatus {
	s.mu.Lock()
	defer s.mu.Unlock()
	return s.status
}

func (s *Session) Prompt(text string, attachments ...string) (Delivery, error) {
	var delivery Delivery
	if attachments == nil {
		attachments = []string{}
	}
	return delivery, s.call(map[string]any{"cmd": "prompt", "text": text, "attachments": attachments}, &delivery)
}

func (s *Session) Answer(request string, answer Answer) error {
	return s.call(map[string]any{"cmd": "answer", "request": request, "answer": answer}, nil)
}

// Configure: value is a string or a bool.
func (s *Session) Configure(option string, value any) error {
	return s.call(map[string]any{"cmd": "configure", "option": option, "value": value}, nil)
}

func (s *Session) Cancel(clearQueue bool) error {
	return s.call(map[string]any{"cmd": "cancel", "clear_queue": clearQueue}, nil)
}

func (s *Session) Dequeue(prompt string) error {
	return s.call(map[string]any{"cmd": "dequeue", "prompt": prompt}, nil)
}

func (s *Session) Rollback(turns int, scope RollbackScope) error {
	return s.call(map[string]any{"cmd": "rollback", "turns": turns, "scope": scope}, nil)
}

func (s *Session) Compact() error {
	return s.call(map[string]any{"cmd": "compact"}, nil)
}

func (s *Session) Close() error {
	return s.call(map[string]any{"cmd": "close"}, nil)
}

// Events yields this session's events in order. Ends after `closed`; yields
// one error on a session error, then ends (W3). Ranging again continues where
// the last range stopped.
func (s *Session) Events() iter.Seq2[Event, error] {
	return func(yield func(Event, error) bool) {
		for ev := range s.queue {
			if !yield(*ev, nil) {
				return
			}
		}
		if err := s.takeErr(); err != nil {
			yield(Event{}, err)
		}
	}
}

func (s *Session) call(cmd map[string]any, out any) error {
	cmd["session"] = s.ID
	return s.rt.call(cmd, out)
}

// push, fail and end are called by Runtime.onLine/onExit; not part of the API.
// Keeps info and status live (W2), applies the cap (W6).
func (s *Session) push(ev *Event) {
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.ended {
		return
	}
	if ev.Kind.SessionUpdated != nil {
		s.info = *ev.Kind.SessionUpdated
	}
	if ev.Kind.StatusChanged != nil {
		s.status = *ev.Kind.StatusChanged
	}
	select {
	case s.queue <- ev:
	default:
		s.failLocked(&Error{Kind: "ConsumerLagged", Message: fmt.Sprintf("%d events unread", queueCap)})
		go s.Close()
	}
}

func (s *Session) fail(err error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.failLocked(err)
}

func (s *Session) failLocked(err error) {
	if !s.ended {
		s.ended, s.err = true, err
		close(s.queue)
	}
}

func (s *Session) end() {
	s.mu.Lock()
	defer s.mu.Unlock()
	if !s.ended {
		s.ended = true
		close(s.queue)
	}
}

func (s *Session) takeErr() error {
	s.mu.Lock()
	defer s.mu.Unlock()
	err := s.err
	s.err = nil
	return err
}

// ---------------------------------------------------------------------------
// HELPERS
// ---------------------------------------------------------------------------

// An OpenOptions' fields, as the top-level fields of `open` and `generate`.
func fields(opts OpenOptions) map[string]any {
	data, _ := json.Marshal(opts)
	var m map[string]any
	json.Unmarshal(data, &m)
	return m
}

// ---------------------------------------------------------------------------
// INTERNAL: finding the binary
// ---------------------------------------------------------------------------

// Options.Bin, then ANYAGENT_BIN, then PATH.
func resolveBinary(bin string) (string, error) {
	if bin == "" {
		bin = os.Getenv("ANYAGENT_BIN")
	}
	if bin != "" {
		return bin, nil
	}
	path, err := exec.LookPath("anyagent")
	if err != nil {
		return "", &Error{Kind: "NotInstalled", Message: "no anyagent binary on PATH: ship it next to your app, set ANYAGENT_BIN, or pass Options.Bin", Data: map[string]any{"agent": "anyagent"}}
	}
	return path, nil
}
