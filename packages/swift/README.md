# Anyagent

One API over the coding agents installed on a machine: Claude Code, Codex,
Cursor, opencode, Kiro, Grok, Hermes, Qwen, pi, Antigravity. This package
spawns the `anyagent` binary your app ships and talks to it over JSON lines.
Every rule lives in the binary; the package is a thin, typed pipe.

```swift
// Package.swift
.package(url: "https://github.com/spotta85/anyagent", from: "0.0.5")
```

The binary is yours to ship: a Mac app signs and notarizes everything it
spawns, so take `anyagent-macos-universal` from the GitHub release and add
it to the app target (Copy Files → Executables, "Code Sign On Copy").
`Runtime.start()` finds it as the bundle's auxiliary executable; `bin:`,
`ANYAGENT_BIN`, and PATH also work.

```swift
import Anyagent

let rt = try await Runtime.start()
let session = try await rt.open("claude", OpenOptions(dir: FileManager.default.currentDirectoryPath))

_ = try await session.prompt("explain this repo")
turn: for try await ev in session.events() {
    switch ev.kind {
    case .textDelta(let d): print(d.text, terminator: "")
    case .requestOpened(.permission(let p)): try await session.answer(p.id, .permission(.allowOnce))
    case .turnEnded: break turn
    default: break
    }
}
try await session.close()
await rt.close()
```

`ev.kind` is an enum with one case per event; `ev.kind.name` is the wire
name (`"TextDelta"`) for a log line. A variant this package does not know
yet decodes as `.unrecognized(name)`, never as an error.

`session.info` and `session.status` stay current (read them with `await`).
A session error (`AuthRequired`, `ProcessExited`) throws from the
`for try await`; a reader that falls 4096 events behind gets
`ConsumerLagged` and its session is closed. Cancelling the reading task
ends the loop. Starting a runtime ignores `SIGPIPE` for the whole app, so
a write to a dead binary throws instead of killing the app.

One-shot text, no session: `try await rt.generate("codex", OpenOptions(dir: dir), "one-line title for this diff")`.

Test your app without agents: build the binary with `--features mock` and
pass `Runtime.start(bin: ..., mock: "packages/mock-scripts/turn.json")`.
`swift test` from the repo root runs this package's own tests.

Docs: https://anyagent.mintlify.site/sidecar · Types: `Types.swift` is
generated from the binary's JSON schema, so commands and events are one contract.
