# T3 Code on anyagent

T3 Code's server talked to six agents through its own adapters. The fork replaces all of them with one adapter over `anyagent-ts`, deletes the old layer, and keeps every feature T3 needs that anyagent could serve. What anyagent could not serve is in [gaps.md](gaps.md).

```
before                                    after
ProviderService                           ProviderService
  ├ ClaudeAdapter   (4.8k)                  └ AnyagentAdapter (1.7k incl. driver, snapshot, text-gen)
  ├ CodexAdapter + SessionRuntime (4.8k)          │
  ├ OpenCodeAdapter + runtime (4.3k)              └ anyagent-ts ── anyagent serve ── claude · codex · cursor · grok · opencode · antigravity
  ├ GrokAdapter     (2.1k)
  ├ CursorAdapter   (1.2k)
  ├ AntigravityAdapter (1.1k)
  ├ acp/            (4.9k)
  └ effect-acp + effect-codex-app-server (3.9k + 68k generated)
```

## Numbers

`git diff main..anyagent` on the fork, non-test unless said. Tests excluded from the headline, listed separately.

| | Lines |
|---|---|
| Hand-written agent code deleted | **43,295** |
| Generated protocol schemas deleted | 68,414 |
| Tests deleted | 48,864 |
| New code added (adapter, driver, snapshot, text generation, check script) | 3,242 |
| New tests added | 1,817 |
| of the deleted hand-written lines, Antigravity sign-in over T3's ACP runtime (product code that needed the deleted runtime) | 1,262 |

Server `provider/` folder, hand-written, before → after: 48,893 → 16,848 lines. What is left is T3's own logic: registries, `ProviderService`, auth, session directory, maintenance and updates, skills, model catalog, Antigravity installer. About 2,400 of those lines are kept but unused until the matching gaps rows land (skills, session instructions).

## What was verified

12 rows, driven over T3's WebSocket RPC against the real agents, script `scripts/anyagent-port-check.ts` in the fork:

| Row | claude | codex |
|---|---|---|
| discover, open+stream, tool+diff, permission, deny, question, model switch, cancel, resume, rollback, generate | PASS | PASS |
| usage (per-turn tokens) | FAIL: gaps row "Per-turn token usage" | same |

T3's server suite: 307/307 files green on the fork. Typecheck green on all 13 workspace projects.

## Reproduce

```bash
# fork: /Users/spotta/Desktop/Projects/t3code, branch anyagent (base: upstream main 679c34c096, 2026-09-26)
cd packages/node/anyagent && npm run build          # anyagent-ts dist is untracked
cargo build --release --features mock               # anyagent binary
cd /path/to/t3code && pnpm install && pnpm typecheck
cd apps/server && TMPDIR=/private/tmp/t3tmp npx vp test run
ANYAGENT_BIN=/path/to/anyagent/target/release/anyagent node scripts/anyagent-port-check.ts --agents claude,codex
git diff --shortstat main..anyagent
```

## Known limits of the fork

- Codex gets no T3 MCP tools (browser, devices, PR linking) until anyagent fixes the `-c` flag order and moves the bearer token out of argv (gaps row "Codex ignores declared MCP servers").
- Threads created before the port reopen a fresh provider session on their next turn, with one warning.
- Antigravity's in-app sign-in is gone; anyagent reports the login command instead (gaps row "In-app login").
- Background provider refresh opens a throwaway session per agent (gaps row "Session-free status check").

Plan and per-task reports: [plan.md](plan.md), [baseline.md](baseline.md).
