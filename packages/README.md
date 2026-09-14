# Packages

Every package is the same thing: a pipe over `anyagent serve`. Spawn the
binary, write one JSON command per line, route each reply by its `id` and
each event by its `session_id`, buffer events per session. The rules live
in the binary and in `docs/sidecar.mdx` ("Writing a wrapper"); the packages
only carry them.

```text
app ── stdin ──►  anyagent serve  ──►  anyagent crate  ──►  claude / codex / acp
app ◄─ stdout ──  replies · events · session errors · closed
```

```text
packages/
  schema.json     the wire, generated from the Rust types (`just schema`)
  typegen.py      schema.json → Swift and Go types (`just types`)
  mock-scripts/   scripted agents the tests of every package play
  node/  python/  swift/  go/
Package.swift     at the repo root: SwiftPM needs the manifest there
```

| | TypeScript `node/` | Python `python/` | Swift `swift/` | Go `go/` |
|---|---|---|---|---|
| Install | `npm install anyagent-ts` | `pip install anyagent-py` | SwiftPM, this repo | `go get .../packages/go` |
| Where the binary comes from | `@anyagent-ts/<os>-<arch>` optional dependency | inside the wheel, on the venv's PATH | the app bundles it (auxiliary executable) | next to the program |
| Also honored | `bin`, `ANYAGENT_BIN` | `bin`, `ANYAGENT_BIN` | `bin:`, `ANYAGENT_BIN`, PATH | `Options.Bin`, `ANYAGENT_BIN`, PATH |
| Types | `json2ts` → `src/types.ts` | `datamodel-codegen` → `types.py` | `typegen.py` → `Types.swift` | `typegen.py` → `types.go` |
| Concurrency | promises, async generator | asyncio, async generator | actors, `AsyncThrowingStream` | goroutine, channels, `iter.Seq2` |
| Tests | `npm test` | `uv run pytest` | `swift test` (repo root) | `go test` |

Why the binary story differs: npm and PyPI host per-platform binaries, so
those packages carry it. Swift and Go have no such registry, a Mac app must
sign what it spawns, and a Go program ships as one file. So there the app
owns the binary and the package only finds it.

How enums cross the wire: Rust's externally tagged form, `{"TextDelta": {..}}`
or a bare `"ContextCompacted"`. Swift decodes that into an enum with one case
per variant; Go into a struct with one field per variant where exactly one
is set, plus `Name()`.

The same eleven cases (S1–S11) run in every package over `mock-scripts/`,
so one behavior is tested four times. Swift runs on macOS and Linux in CI;
the others on Windows too.
