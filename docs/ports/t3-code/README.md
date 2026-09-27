# T3 Code on anyagent

T3 Code's server talked to six agents through its own adapters. The fork replaces all of them with one adapter over `anyagent-ts`, deletes the old layer, and keeps every feature T3 needs that anyagent could serve. What anyagent could not serve is in [gaps.md](gaps.md).

```
before (wc -l at 679c34c096)              after
ProviderService                           ProviderService
  ├ ClaudeAdapter   (5.6k)                  └ provider/anyagent/ (1.9k: adapter, driver, snapshot, events, text-gen)
  ├ CodexAdapter + SessionRuntime (5.5k)          │
  ├ OpenCodeAdapter + runtime (5.1k)              └ anyagent-ts ── anyagent serve ── claude · codex · cursor · grok · opencode · antigravity
  ├ GrokAdapter     (2.2k)
  ├ CursorAdapter   (1.3k)
  ├ AntigravityAdapter (1.3k)
  ├ acp/            (4.9k)
  └ effect-acp + effect-codex-app-server (3.9k + 68k generated)
```

## Numbers

`git diff --numstat main..anyagent` on the fork, one bucket per file. Tests: `*.test.ts`, test fixtures and examples, `apps/server/integration/`, the ACP mock agent script. Docs: `*.md`. `pnpm-lock.yaml` left out.

| | Deleted | Added |
|---|---|---|
| Hand-written agent code (product) | **41,326** | 2,005 (1,856 in `provider/anyagent/`, 149 elsewhere) |
| Generated protocol schemas | 68,414 | 0 |
| Tests | 50,803 | 1,889 |
| Live check script (`scripts/anyagent-port-check.ts`) | – | 1,235 |
| Docs | 46 | 120 |
| of the deleted product lines: Antigravity sign-in over T3's ACP runtime (needed the deleted runtime) | 1,262 | |

Server `provider/` folder, hand-written, before → after: 48,893 → 16,955 lines. What is left is T3's own logic: registries, `ProviderService`, auth, session directory, maintenance and updates, skills, model catalog, Antigravity installer. About 2,400 of those lines are kept but unused until the matching gaps rows land (skills, session instructions).

## What was verified

13 rows, driven over T3's WebSocket RPC against the real agents, script `scripts/anyagent-port-check.ts` in the fork:

| Row | claude | codex |
|---|---|---|
| discover, open+stream, tool+diff, permission, deny, question, model switch, cancel, resume, rollback, generate | PASS | PASS |
| subagent (task events, nested tools, no subagent text in the chat) | PASS | SKIP (claude only) |
| usage (per-turn tokens) | FAIL: gaps row "Per-turn token usage" | same |

Full matrix from Task 4. After the final review fixes, rerun: claude open+stream, tool+diff, subagent, resume, generate and codex resume, all PASS.

T3's server suite: 307/307 files green on the fork at Task 4; after the final review fixes, the `provider/anyagent/` suite (77 tests) and the server and scripts typecheck. Typecheck green on all 13 workspace projects at Task 4.

## Reproduce

```bash
# fork: /Users/spotta/Desktop/Projects/t3code, branch anyagent (base: upstream main 679c34c096, 2026-09-26)
cd packages/node/anyagent && npm run build          # anyagent-ts dist is untracked
cargo build --release --features mock               # anyagent binary
cd /path/to/t3code && pnpm install && pnpm typecheck
cd apps/server && TMPDIR=/private/tmp/t3tmp npx vp test run
ANYAGENT_BIN=/path/to/anyagent/target/release/anyagent node scripts/anyagent-port-check.ts --agents claude,codex
git diff --shortstat main..anyagent
bash /path/to/anyagent/docs/ports/t3-code/measure.sh   # the per-bucket table above
```

## Known limits of the fork

- Codex gets no T3 MCP tools (browser, devices, PR linking) until anyagent fixes the `-c` flag order and moves the bearer token out of argv (gaps row "Codex ignores declared MCP servers").
- Threads created before the port, and threads whose resume token the agent no longer knows, reopen a fresh provider session on their next turn, with one warning.
- Antigravity's in-app sign-in is gone; anyagent reports the login command instead (gaps row "In-app login").
- Background provider refresh opens a throwaway session per agent (gaps row "Session-free status check").

Plan and per-task reports: [plan.md](plan.md), [baseline.md](baseline.md).
