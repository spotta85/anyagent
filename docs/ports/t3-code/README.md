# T3 Code on anyagent

T3 Code's server talked to six agents through its own adapters. The fork replaces all of them with one adapter over `anyagent-ts`, deletes the old layer, and keeps the features T3 needs. What anyagent still cannot serve is in [gaps.md](gaps.md).

```
before (wc -l at 679c34c096)              after
ProviderService                           ProviderService
  ├ ClaudeAdapter   (5.6k)                  └ provider/anyagent/ (2.2k: adapter, driver, snapshot, events, text-gen)
  ├ CodexAdapter + SessionRuntime (5.5k)          │
  ├ OpenCodeAdapter + runtime (5.1k)              └ anyagent-ts ── anyagent serve ── claude · codex · cursor · grok · opencode · antigravity
  ├ GrokAdapter     (2.2k)
  ├ CursorAdapter   (1.3k)
  ├ AntigravityAdapter (1.3k)
  ├ acp/            (4.9k)
  └ effect-acp + effect-codex-app-server (3.9k + 68k generated)
```

## Numbers

`git diff --numstat main..anyagent` on the fork (head `2f67c4072f`), one bucket per file, from [measure.sh](measure.sh). Tests: `*.test.ts`, test fixtures and examples, `apps/server/integration/`, the ACP mock agent script. Docs: `*.md`. `pnpm-lock.yaml` left out.

| | Deleted | Added |
|---|---|---|
| Hand-written agent code (product) | **41,549** | 2,355 (2,197 in `provider/anyagent/`, 158 elsewhere) |
| Generated protocol schemas | 68,414 | 0 |
| Tests | 50,824 | 2,738 |
| Live check script (`scripts/anyagent-port-check.ts`) | – | 1,474 |
| Docs | 46 | 163 |
| of the deleted product lines: Antigravity sign-in over T3's ACP runtime (needed the deleted runtime) | 1,262 | |

Server `provider/` folder, hand-written, before → after: 48,893 → about 17,070 lines. What is left is T3's own logic: registries, `ProviderService`, auth, session directory, maintenance and updates, skills, model catalog, Antigravity installer. T3's skills code is kept but unused until the workspace-skills gap lands.

## What the port serves

Two rounds. The first replaced the adapters. The second closed the gaps the first one found, in anyagent, and wired them into T3.

| T3 feature | Served by |
|---|---|
| Sessions, streaming, tools with diffs, permissions, questions, subagents | anyagent events |
| Model and option pickers, per model | `ConfigChoice.options` |
| Plan mode with a proposed plan | `mode: plan`, `PlanProposed` |
| Runtime modes: approval-required, auto-accept-edits, full-access | `PermissionMode` `Ask`, `AcceptEdits`, `AutoApprove` |
| Interrupt one turn, rollback, resume | `cancel` with a turn, `rollback`, resume tokens |
| Turn token usage, usage limits with banked resets | `TurnEnded.usage`, `plan_usage` |
| T3's MCP server in every session | `mcp_servers`; secrets stay off the command line and out of the wire log |
| Session instructions, instance binary, environment, home, launch args | `instructions`, agent `{ id, path }`, `env`, `config_home`, `args` |
| Titles, branch names, commit messages, with images | `generate` with `attachments` |
| Native wire log | `record_wire` |

## What was verified

18 rows, driven over T3's WebSocket RPC against the real agents (script `scripts/anyagent-port-check.ts` in the fork). Every cell that is not PASS says why.

| Row | claude | codex | cursor | grok | opencode | antigravity |
|---|---|---|---|---|---|---|
| discover, usage-limits | PASS | PASS | PASS | PASS | PASS | PASS |
| open+stream | PASS | PASS | PASS¹ | PASS¹ | PASS | PASS |
| tool+diff | PASS | PASS | PASS¹ | PASS | PASS | PASS |
| permission | PASS | PASS | SKIP: edits need no approval¹ | PASS | FAIL (model)³ | PASS |
| deny | PASS | PASS | SKIP: edits need no approval¹ | PASS¹ | PASS | PASS |
| question | PASS | PASS | not proven (quota) | PASS¹ | PASS | PASS |
| subagent | PASS | SKIP: claude only | SKIP | SKIP | SKIP | SKIP |
| model-switch | PASS | PASS | PASS² | SKIP: one model | PASS | PASS |
| cancel | PASS | PASS | not proven (quota) | PASS¹ | PASS | PASS |
| resume | PASS | PASS | not proven (quota) | PASS | PASS | PASS |
| rollback | PASS | PASS | SKIP: agent has none | SKIP: agent has none | PASS | SKIP: agent has none |
| usage (turn tokens) | PASS | PASS | SKIP: ACP | not proven (quota) | PASS | SKIP: ACP |
| generate | PASS | PASS | not proven (quota) | not proven (quota) | PASS | PASS |
| instructions | PASS | PASS | PASS² | not proven (quota) | PASS | PASS |
| plan | PASS | PASS | not proven (quota) | SKIP: no plan mode | SKIP: no plan mode | SKIP: no plan mode |
| accept-edits | PASS | SKIP: sandbox never asks | not proven (quota) | not proven (quota) | PASS | PASS |
| mcp-tool | PASS | PASS | not proven (quota) | not proven (quota) | PASS | PASS |

¹ Passed, or was seen on the wire, before that account's free quota ran out mid-run. ² Ran with cursor's quota out; the check reads a config event or the outgoing wire, not the reply. ³ opencode's free model answered a queued prompt with the wrong text; it passed the run before. Not an adapter or anyagent fault.

No cell failed because of anyagent.

T3's server suite: 4,417 tests pass, 2 skipped, 307 files. Typecheck, lint and format clean.

## Reproduce

```bash
# fork: /Users/spotta/Desktop/Projects/t3code, branch anyagent (base: upstream main 679c34c096, 2026-09-26)
cd packages/node/anyagent && npm install --no-package-lock && npm run build   # anyagent-ts dist is untracked
cargo build --release --features mock                                         # anyagent binary
cd /path/to/t3code && pnpm install && pnpm typecheck
cd apps/server && TMPDIR=/private/tmp/t3tmp npx vp test run
ANYAGENT_BIN=/path/to/anyagent/target/release/anyagent node scripts/anyagent-port-check.ts --agents claude,codex
git diff --shortstat main..anyagent
bash /path/to/anyagent/docs/ports/t3-code/measure.sh   # the per-bucket table above
```

## Known limits of the fork

| Limit | Why |
|---|---|
| Threads created before the port, and threads whose resume token the agent no longer knows, reopen a fresh provider session on their next turn, with one warning | The old adapters' cursors are not anyagent resume tokens |
| Antigravity's in-app sign-in is gone; T3 shows the login command | gaps row "In-app login" |
| Redeeming a banked reset is not offered; the count is shown | gaps row "Redeem a reset credit" |
| Claude shows no banked resets | The claude CLI does not report them (gaps row "Banked resets on claude") |
| On cursor, approval-required threads write files without asking | cursor's ACP agent asks permission for commands, not for edits |
| A claude plan's exit request shows as a failed tool row in the plan turn | T3 declines it to stop at the plan, and a decline cannot carry a message |
| codex and ACP status checks open a throwaway session | gaps row "Session-free status check" |
| Settings with no anyagent equivalent do nothing | codex shadow home, claude auto-compact window, cursor API endpoint, antigravity auth fields, opencode external server |

Plans and reports: [plan.md](plan.md) (the port), [gaps-plan.md](gaps-plan.md) (the gap fixes), [baseline.md](baseline.md).
