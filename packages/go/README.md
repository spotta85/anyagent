# anyagent

One API over the coding agents installed on a machine: Claude Code, Codex,
Cursor, opencode, Kiro, Grok, Hermes, Qwen, pi, Antigravity. This package
spawns the `anyagent` binary your program ships and talks to it over JSON
lines. Every rule lives in the binary; the package is a thin, typed pipe.

```bash
go get github.com/spotta85/anyagent/packages/go
```

The binary is yours to ship: a Go program is one file, so put the release
binary for each platform (`anyagent-<target>` on the GitHub release) next
to it or on PATH. `Start` looks at `Options.Bin`, then `ANYAGENT_BIN`, then PATH.

```go
import anyagent "github.com/spotta85/anyagent/packages/go"

rt, err := anyagent.Start(anyagent.Options{})
session, err := rt.Open("claude", anyagent.OpenOptions{Dir: cwd})

session.Prompt("explain this repo")
for ev, err := range session.Events() {
	if err != nil {
		log.Fatal(err)
	}
	if d := ev.Kind.TextDelta; d != nil {
		fmt.Print(d.Text)
	}
	if r := ev.Kind.RequestOpened; r != nil && r.Permission != nil {
		choice := anyagent.PermissionChoiceAllowOnce
		session.Answer(r.Permission.ID, anyagent.Answer{Permission: &choice})
	}
	if ev.Kind.TurnEnded != nil {
		break
	}
}
session.Close()
rt.Close()
```

`ev.Kind` has one field per event and exactly one is set; `ev.Kind.Name()`
is the wire name (`"TextDelta"`) for a `switch` or a log line.

`session.Info()` and `session.Status()` stay current. A session error
(`AuthRequired`, `ProcessExited`) is yielded once by `Events`, then the
range ends; a reader that falls 4096 events behind gets `ConsumerLagged`
and its session is closed. Ranging again continues where the last range stopped.

One-shot text, no session: `rt.Generate("codex", anyagent.OpenOptions{Dir: dir}, "one-line title for this diff")`.

Test your app without agents: build the binary with `--features mock` and
pass `anyagent.Options{Bin: ..., Mock: "packages/mock-scripts/turn.json"}`.

Docs: https://anyagent.mintlify.site/sidecar · Types: `types.go` is
generated from the binary's JSON schema, so commands and events are one contract.
