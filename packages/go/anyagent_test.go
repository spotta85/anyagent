// S1–S11 from ticket 13: the wrapper against `anyagent serve --mock`.
// Needs a mock-enabled binary: `cargo build --features mock` (or ANYAGENT_BIN).

package anyagent

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"slices"
	"sync"
	"syscall"
	"testing"
	"time"
)

var (
	root, _ = filepath.Abs("../..")
	bin     = os.Getenv("ANYAGENT_BIN")
	scripts = filepath.Join(root, "packages/mock-scripts")
	dir     = os.TempDir()
)

// The test binary stands in for anyagent in S11: with FAKE_LINE set it prints that line and exits.
func TestMain(m *testing.M) {
	if line := os.Getenv("FAKE_LINE"); line != "" {
		fmt.Println(line)
		os.Exit(0)
	}
	if bin == "" {
		bin = filepath.Join(root, "target/debug/anyagent")
		if runtime.GOOS == "windows" {
			bin += ".exe"
		}
	}
	os.Exit(m.Run())
}

// A runtime over a mock script, closed when the test ends however it ends.
func start(t *testing.T, script string) *Runtime {
	t.Helper()
	rt, err := Start(Options{Bin: bin, Mock: filepath.Join(scripts, script+".json")})
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { rt.Close() })
	return rt
}

func open(t *testing.T, rt *Runtime) *Session {
	t.Helper()
	s, err := rt.Open("mock", OpenOptions{Dir: dir})
	if err != nil {
		t.Fatal(err)
	}
	return s
}

// Reads the stream to its end.
func drain(s *Session) ([]Event, error) {
	var out []Event
	for ev, err := range s.Events() {
		if err != nil {
			return out, err
		}
		out = append(out, ev)
	}
	return out, nil
}

// Events until `kind` (inclusive); the last one is the match.
func until(s *Session, kind string) ([]Event, error) {
	var seen []Event
	for ev, err := range s.Events() {
		if err != nil {
			return seen, err
		}
		seen = append(seen, ev)
		if ev.Kind.Name() == kind {
			return seen, nil
		}
	}
	return seen, fmt.Errorf("stream ended before %s after %d events", kind, len(seen))
}

func texts(events []Event) []string {
	var out []string
	for _, ev := range events {
		if ev.Kind.TextDelta != nil {
			out = append(out, ev.Kind.TextDelta.Text)
		}
	}
	return out
}

// Fails unless err is an *Error of `kind`.
func rejects(t *testing.T, err error, kind string) *Error {
	t.Helper()
	var e *Error
	if !errors.As(err, &e) || e.Kind != kind {
		t.Fatalf("expected %s, got %v", kind, err)
	}
	return e
}

func ok(t *testing.T, err error) {
	t.Helper()
	if err != nil {
		t.Fatal(err)
	}
}

func TestS1OpenPromptAnswerThePermissionSeeTheTurnEndClose(t *testing.T) {
	rt := start(t, "turn")
	session := open(t, rt)
	if session.Info().ID != session.ID {
		t.Fatal("info.id")
	}

	delivery, err := session.Prompt("hi")
	ok(t, err)
	if delivery.Kind.Started == nil {
		t.Fatalf("%+v", delivery)
	}

	opened, err := until(session, "RequestOpened")
	ok(t, err)
	request := opened[len(opened)-1].Kind.RequestOpened.Permission
	choice := PermissionChoiceAllowOnce
	ok(t, session.Answer(request.ID, Answer{Permission: &choice}))

	rest, err := until(session, "TurnEnded")
	ok(t, err)
	if got := texts(rest); !slices.Equal(got, []string{"Done."}) {
		t.Fatal(got)
	}
	_, err = until(session, "StatusChanged")
	ok(t, err)
	if session.Status() != SessionStatusIdle { // W2: live
		t.Fatal(session.Status())
	}

	ok(t, session.Close())
	if left, err := drain(session); err != nil || len(left) != 0 {
		t.Fatal("no events after closed", left, err)
	}
	rt.Close()
}

func TestS2EventsBufferedBeforeTheAppIteratesAreAllDelivered(t *testing.T) {
	rt := start(t, "turn")
	session := open(t, rt)
	_, err := session.Prompt("hi")
	ok(t, err)
	time.Sleep(200 * time.Millisecond)
	seen, err := until(session, "RequestOpened")
	ok(t, err)
	if got := texts(seen); !slices.Equal(got, []string{"Let me check. "}) {
		t.Fatal(got)
	}
	if !slices.ContainsFunc(seen, func(ev Event) bool { return ev.Kind.TurnStarted != nil }) {
		t.Fatal("no TurnStarted")
	}
	rt.Close()
}

func TestS3APromptAfterCloseRejectsWithSessionClosed(t *testing.T) {
	rt := start(t, "turn")
	session := open(t, rt)
	ok(t, session.Close())
	_, err := session.Prompt("x")
	rejects(t, err, "SessionClosed")
	rt.Close()
}

func TestS4TwoSessionsSeeOnlyTheirOwnEventsInOrder(t *testing.T) {
	rt := start(t, "chatter")
	sessions := make([]*Session, 2)
	events := make([][]Event, 2)
	errs := make([]error, 2) // t.Fatal is for the test goroutine only: collect, check after Wait
	var wg sync.WaitGroup
	for i := range sessions {
		wg.Add(1)
		go func() {
			defer wg.Done()
			sessions[i], errs[i] = rt.Open("mock", OpenOptions{Dir: dir})
			if errs[i] == nil {
				_, errs[i] = sessions[i].Prompt("x")
			}
			if errs[i] == nil {
				events[i], errs[i] = until(sessions[i], "TurnEnded")
			}
		}()
	}
	wg.Wait()
	ok(t, errors.Join(errs...))
	if sessions[0].ID == sessions[1].ID {
		t.Fatal("same id")
	}
	for i, s := range sessions {
		var seqs []uint64
		for _, ev := range events[i] {
			if ev.SessionID != s.ID {
				t.Fatal("foreign event")
			}
			seqs = append(seqs, ev.Sequence)
		}
		if !slices.IsSorted(seqs) {
			t.Fatal(seqs)
		}
		if got := texts(events[i]); !slices.Equal(got, []string{"one", "two", "three"}) {
			t.Fatal(got)
		}
	}
	rt.Close()
}

func TestS5AMalformedLineIsAnsweredWithBadFrameAndTheRuntimeGoesOn(t *testing.T) {
	rt := start(t, "turn")
	rt.write([]byte("not json"))
	report, err := rt.Discover()
	ok(t, err)
	if report.Agents[0].ID != "mock" {
		t.Fatal(report)
	}
	rt.Close()
}

func TestS6ProcessDeathRejectsPendingCallsFailsIteratorsAndLaterCalls(t *testing.T) {
	rt := start(t, "turn")
	session := open(t, rt)
	// Killed first: nothing written from here on can be answered.
	ok(t, rt.cmd.Process.Kill())
	_, err := rt.Discover()
	rejects(t, err, "ProcessExited")
	_, err = rt.Open("mock", OpenOptions{Dir: dir})
	rejects(t, err, "ProcessExited")
	_, err = drain(session)
	rejects(t, err, "ProcessExited")
	_, err = rt.Discover()
	rejects(t, err, "ProcessExited")
	rt.Close()
}

// The turn fails, the stream yields ProcessExited, then ends.
func TestS7TheAgentDyingMidTurn(t *testing.T) {
	rt := start(t, "die")
	session := open(t, rt)
	_, err := session.Prompt("go")
	ok(t, err)
	events, err := until(session, "TurnEnded")
	ok(t, err)
	if ended := events[len(events)-1].Kind.TurnEnded; ended.Stop.Failed == nil {
		t.Fatalf("%+v", ended)
	}
	_, err = drain(session)
	if e := rejects(t, err, "ProcessExited"); e.Data["status"] != "9" {
		t.Fatal(e.Data)
	}
	if left, err := drain(session); err != nil || len(left) != 0 {
		t.Fatal("stream restarted", left, err)
	}
	rt.Close()
}

func TestS8aA20000EventFloodArrivesWholeAndInOrder(t *testing.T) {
	rt := start(t, "flood")
	session := open(t, rt)
	_, err := session.Prompt("go")
	ok(t, err)
	deltas := 0
	var last uint64
	for ev, err := range session.Events() {
		ok(t, err)
		if ev.Sequence <= last {
			t.Fatal("out of order", ev.Sequence, last)
		}
		last = ev.Sequence
		if ev.Kind.TextDelta != nil {
			deltas++
		}
		if ev.Kind.TurnEnded != nil {
			break
		}
	}
	if deltas != 20_000 {
		t.Fatal(deltas)
	}
	rt.Close()
}

func TestS8bAConsumerThatStopsReadingGetsConsumerLaggedAndTheRuntimeSurvives(t *testing.T) {
	rt := start(t, "flood")
	session := open(t, rt)
	_, err := session.Prompt("go")
	ok(t, err)
	n := 0
	for _, err := range session.Events() {
		ok(t, err)
		if n++; n == 10 {
			break
		}
	}
	time.Sleep(2500 * time.Millisecond) // 4096 events arrive in ~0.8 s at the flood's pace
	_, err = until(session, "TurnEnded")
	rejects(t, err, "ConsumerLagged")
	report, err := rt.Discover()
	ok(t, err)
	if report.Agents[0].ID != "mock" {
		t.Fatal(report)
	}
	rt.Close()
}

// `closed` arrives, the process exits 0 and is gone.
func TestS9CloseWithASessionOpen(t *testing.T) {
	rt := start(t, "turn")
	session := open(t, rt)
	pid := rt.cmd.Process.Pid
	if code := rt.Close(); code != 0 {
		t.Fatal(code)
	}
	if left, err := drain(session); err != nil || len(left) != 0 {
		t.Fatal("closed should end the stream", left, err)
	}
	if runtime.GOOS != "windows" {
		if p, _ := os.FindProcess(pid); p.Signal(syscall.Signal(0)) == nil {
			t.Fatal("process still there")
		}
	}
}

func TestS10ConfigureSendsOptionAndSessionUpdatedUpdatesInfo(t *testing.T) {
	rt := start(t, "configure")
	session := open(t, rt)
	ok(t, session.Configure("model", "opus"))
	_, err := until(session, "SessionUpdated")
	ok(t, err)
	if model := session.Info().Configuration.Options["model"].String; model == nil || *model != "opus" {
		t.Fatal(session.Info().Configuration)
	}
	rt.Close()
}

func TestS11ABinaryThatDoesNotSpeakTheProtocolFailsStartWithProtocolFailed(t *testing.T) {
	speaks := func(line string) error {
		_, err := Start(Options{Bin: os.Args[0], Env: append(os.Environ(), "FAKE_LINE="+line)})
		return err
	}
	rejects(t, speaks("nope"), "ProtocolFailed")
	rejects(t, speaks(`{"hello": {"protocol": 99, "anyagent": "x"}}`), "ProtocolFailed")
}
