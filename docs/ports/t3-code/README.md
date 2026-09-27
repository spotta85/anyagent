# T3 Code on anyagent

T3 Code's server talked to six agents through its own adapters. The fork replaces all of them with one adapter over `anyagent-ts`, deletes the old layer, and keeps the features T3 needs. What anyagent still cannot serve is in [gaps.md](gaps.md).

```
before (wc -l at 679c34c096)              after
ProviderService                           ProviderService
  ├ ClaudeAdapter   (5.6k)                  └ provider/anyagent/ (2.3k: adapter, driver, snapshot, events, text-gen)
  ├ CodexAdapter + SessionRuntime (5.5k)          │
  ├ OpenCodeAdapter + runtime (5.1k)              └ anyagent-ts ── anyagent serve ── claude · codex · cursor · grok · opencode · antigravity
  ├ GrokAdapter     (2.2k)
  ├ CursorAdapter   (1.3k)
  ├ AntigravityAdapter (1.3k)
  ├ acp/            (4.9k)
  └ effect-acp + effect-codex-app-server (3.9k + 68k generated)
```

## Numbers

`git diff --numstat main..anyagent` on the fork (head `7f297bb45d`, docs-only commit after it), one bucket per file, from [measure.sh](measure.sh). Tests: `*.test.ts`, test fixtures and examples, `apps/server/integration/`, the ACP mock agent script. Docs: `*.md`. `pnpm-lock.yaml` left out.

| | Deleted | Added |
|---|---|---|
| Hand-written agent code (product) | **41,549** | 2,475 (2,312 in `provider/anyagent/`, 163 elsewhere) |
| Generated protocol schemas | 68,414 | 0 |
| Tests | 50,824 | 3,070 |
| Live check script (`scripts/anyagent-port-check.ts`) | – | 1,647 |
| Docs | 46 | 189 |
| of the deleted product lines: Antigravity sign-in over T3's ACP runtime (needed the deleted runtime) | 1,262 | |

Server `provider/` folder, hand-written, before → after: 48,893 → about 17,070 lines. What is left is T3's own logic: registries, `ProviderService`, auth, session directory, maintenance and updates, skills, model catalog, Antigravity installer. T3's skill picker is filled from anyagent's commands that are skills.

## What the port serves

Three rounds. The first replaced the adapters. The second and third closed the gaps the first one found, in anyagent, and wired them into T3.

| T3 feature | Served by |
|---|---|
| Sessions, streaming, tools with diffs, permissions, questions, subagents | anyagent events |
| Subagent rows: role, model, progress, tokens | `ToolUpdate.subagent` |
| Tool progress, the turn's running diff, model rerouted | `ToolProgress`, `TurnDiff`, `ModelRerouted` |
| Cancel an approval; decline with a reason | `Answer::Cancel`, `Answer::Deny { message }` |
| Skill picker | `SlashCommand.source` |
| Model and option pickers, per model | `ConfigChoice.options` |
| Plan mode with a proposed plan | `mode: plan`, `PlanProposed` |
| Runtime modes: approval-required, auto-accept-edits, full-access | `PermissionMode` `Ask`, `AcceptEdits`, `AutoApprove` |
| Interrupt one turn, rollback, resume | `cancel` with a turn, `rollback`, resume tokens |
| Turn token usage, usage limits with banked resets | `TurnEnded.usage`, `plan_usage` |
| T3's MCP server in every session | `mcp_servers`; secrets stay off the command line and out of the wire log |
| Session instructions, instance binary, environment, home, launch args | `instructions`, agent `{ id, path }`, `env`, `config_home`, `args` |
| Titles, branch names, commit messages, with images | `generate` with `attachments`, and `output_schema` on claude and codex |
| Native wire log | `record_wire` |

## What was verified

23 rows, driven over T3's WebSocket RPC against the real agents (script `scripts/anyagent-port-check.ts` in the fork). Every cell that is not PASS says why.

| Row | claude | codex | cursor | grok | opencode | antigravity |
|---|---|---|---|---|---|---|
| discover, usage-limits | PASS | PASS | PASS | PASS | PASS | PASS |
| open+stream | PASS | PASS | PASS¹ | PASS¹ | PASS | PASS |
| tool+diff | PASS | PASS | PASS¹ | PASS | PASS | PASS |
| permission | PASS | PASS | SKIP: edits need no approval¹ | PASS | FAIL (model)³ | PASS |
| deny | PASS | PASS | not proven (quota) | PASS¹ | PASS | PASS |
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
| turn-diff | SKIP: sends none | PASS | not proven (quota) | SKIP: sends none | SKIP: sends none | SKIP: sends none |
| subagent-info | PASS: role, tokens | PASS: model, tokens; no role⁴ | not proven (quota) | SKIP: ran no subagent | PASS: role, model; no tokens⁴ | SKIP: ran no subagent |
| cancel-request | PASS | PASS | not proven (quota) | PASS | PASS | PASS |
| skills | PASS | PASS | not proven (quota) | SKIP: reads no skill folder | PASS | SKIP: reads no skill folder |
| schema-generate | PASS | PASS | not proven (quota) | SKIP: no output schema | SKIP: no output schema | SKIP: no output schema |

¹ Passed, or was seen on the wire, before that account's free quota ran out mid-run. ² Ran with cursor's quota out; the check reads a config event or the outgoing wire, not the reply. ³ opencode's free model answered a queued prompt with the wrong text in 3 of 4 runs; the wire shows the prompt arrived intact. Not an adapter or anyagent fault. ⁴ Failed on the first run: anyagent did not link codex's spawned children and read no subagent info from opencode. Both fixed in anyagent, then passed. The row passes when a task row names a role or a model; codex's wire has no role and opencode's no token count, so the cell lists what arrived.

No cell fails because of anyagent. Two did on their first run (⁴); the live check is what found them.

T3's server suite: 4,428 tests pass, 2 skipped, 307 files. Typecheck, lint and format clean.

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
| ACP status checks open a throwaway session | gaps row "Session-free status check" |
| claude's skills show in the `/` menu, not the skill picker | claude names no path for a skill, and T3's picker needs one |
| Settings with no anyagent equivalent do nothing | codex shadow home, claude auto-compact window, cursor API endpoint, antigravity auth fields, opencode external server |

Plans and reports: [plan.md](plan.md) (the port), [gaps-plan.md](gaps-plan.md) and [gaps-plan-2.md](gaps-plan-2.md) (the gap fixes), [baseline.md](baseline.md).
