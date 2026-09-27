# Gaps found porting T3 Code to anyagent

Things T3 Code's server needed from its agent layer that anyagent did not have. Each row is one feature of T3 and what anyagent does about it today.

```
33 gaps found  ──►  20 fixed  ·  3 partly fixed  ·  10 open
```

How they were fixed: [gaps-plan.md](gaps-plan.md).

## Fixed

| Feature | T3 call | anyagent now |
|---|---|---|
| Plan mode | `sendTurn({ interactionMode: "plan" })`, `turn.proposed.completed` | The `mode` choice `plan` on claude and codex; `EventKind::PlanProposed { markdown }`. The request that approves a plan always reaches the app |
| Accept-edits runtime mode | `startSession({ runtimeMode: "auto-accept-edits" })` | `PermissionMode::AcceptEdits`: edits, deletes and moves are allowed once, the rest is forwarded |
| Per-turn token usage | `turn.completed.payload.tokenUsage` | `TurnEnded.usage` on claude, codex, opencode, pi, antigravity. ACP agents report context fill only |
| Banked resets in usage | the usage panel's reset credits | `PlanUsage.reset_credits { available, next_expires_at }` on codex. Claude: see Open |
| Rollback confirmation | `rollbackThread(threadId, n)` | `rollback` resolves when the agent answers; a refusal is `InvalidRequest(reason)` |
| Cancel one turn | `interruptTurn(threadId, turnId)` | `Session::cancel_turn(turn, clear_queue)`; `dequeue` withdraws a queued prompt |
| Narrow `ResumeFailed` | `startSession({ resumeCursor })` | Only an unknown thread (codex) or a 404 (opencode) is `ResumeFailed` |
| Quiet codex sessions | every `runtime.warning` is a work-log row | Warnings anyagent caused itself, and codex's bookkeeping frames, are dropped. A blocked user hook is a warning |
| Tool denied | `tool.denied` | `ToolStatus::Denied` with the reason in `output`, for a claude deny rule or mode. A hook refusal stays `Failed` (claude sends no frame for it) |
| Session instructions | T3's runtime instructions and codex tool guide | `open { instructions }`, sent through each agent's own channel |
| Per-instance binary and environment | settings `binaryPath`, `environment`, homes, `launchArgs` | agent `{ id, path }`, `env`, `config_home`, `args` on open, probe, generate and plan usage |
| Per-model option descriptors | the model picker's effort and fast per model | `ConfigChoice.options` on each `model` choice (claude, codex, opencode) |
| Images in one-shot generation | `generateBranchName` / `generateThreadTitle` with attachments | `generate { attachments }` |
| Raw wire log | `ProviderEventLoggers.native` | `open { record_wire }`; declared MCP servers' header and env values are written as `<redacted>` |
| Client MCP servers on opencode | T3's `t3-code` MCP server | Registered with opencode at open; MCP tool calls are `ToolKind::Mcp` |
| Codex ignores declared MCP servers | same | Overrides follow the `app-server` subcommand; the bearer token travels in an env var |
| Codex declines MCP tool approvals | an MCP tool that needs approval | The approval is a normal permission request; allow runs the tool |
| Claude MCP secrets in argv | bearer header of T3's MCP server | Declared servers ride claude's control channel; a server that fails to connect is a warning |
| Full-access approved a proposed plan | plan mode under `full-access` | The engine never answers a plan's approval by itself |
| Secrets in `Debug` output | logs | `SessionOptions` and `McpServer` print env and header names only |

## Partly fixed

| Feature | Fixed | Still missing | Size (lines) |
|---|---|---|---|
| Session-free status check | A claude probe runs no user hooks and no user MCP servers | codex and ACP probes still open a throwaway session (their details come from the thread or session reply) | ~60 |
| Isolated one-shot generation | claude `generate` runs no user hooks and no user MCP servers | An output schema (`--json-schema`, `--output-schema`); isolation on other agents | ~40 |
| Workspace skills | `probe_with` in the workspace's dir returns that workspace's commands | `SlashCommand.source: Builtin \| Skill { path, scope }` | ~30 |

## Open

| Feature | T3 call | What anyagent lacks | Proposed change | Size (lines) |
|---|---|---|---|---|
| Banked resets on claude | the usage panel's reset credits | The CLI's `get_usage` returns the block as `null` (2.1.283). The data needs the login token and a direct API call, which anyagent does not do. The parser is in place for when the CLI passes it through | Owner decision: read the token, or wait for the CLI | ~40 |
| Redeem a reset credit | `ProviderInstance.consumeResetCredit` | No account actions | `Runtime::redeem_reset(agent)` | ~40 |
| In-app login | `ProviderInstance.auth` | `LoginMethod::Terminal { command }` only; anyagent cannot run a login | `Runtime::login(agent, method)` streaming url, code, done, failed; `logout(agent)` | ~120 |
| Feedback upload | `adapter.uploadFeedback` | No command | `Session::upload_feedback { reason }` (codex) | ~30 |
| Withdraw a permission request | `respondToRequest(.., "cancel")` | No withdraw answer. `answer(DenyOnce)` then `cancel_turn` has the same effect | `PermissionChoice::Cancel`, only if an app needs the difference | ~20 |
| Subagent progress | `task.progress`, role and model on `task.started` | A subagent is a tool with a title and status | `ToolUpdate.subagent: Option<SubagentInfo>` | ~50 |
| Live events: turn diff, tool progress, model rerouted | `turn.diff.updated`, `tool.progress`, `model.rerouted` | Not mapped; the app still gets the final diff, the tool's end state, and a warning | One event kind or field each | ~60 |
| MCP on antigravity; codex MCP env in argv | T3's MCP server | `agy` has no per-session MCP config. codex passes stdio env and non-bearer headers as `-c` overrides | None known for `agy`; a config file for codex | ~50 |
| Deny with a message | T3 stops claude at its plan by declining the exit request | A deny carries no text, so claude is told "User denied this action" and the tool row reads failed. claude still stops and the next turn runs | `Answer::Permission` with an optional message, passed where the wire takes one | ~20 |
| MCP tool kind on ACP agents | tool rows for T3's MCP tools | An ACP agent's MCP call is `ToolKind::Other` although its `_meta.mcp` names the server and tool (seen on antigravity's ACP server) | Map `_meta.mcp` to `ToolKind::Mcp` | ~15 |

## Known small limits

Found by the reviews, not worth a row each. They are listed with file and line in the plan's ledger.

| Limit | Effect |
|---|---|
| Subagent model calls are not counted in turn usage (opencode, antigravity) | A turn that spawns subagents under-reports tokens |
| No timeout on codex `thread/revert` | A codex that never answers keeps `rollback()` waiting until `close()` |
| ACP steer does not carry owed instructions | First prompt `/cmd`, then a steered message: the instructions arrive one prompt later |
| Two identical MCP calls in flight on codex | The approval is shown on one of the two |
| claude `mode` lists 4 choices | The CLI can report `dontAsk` or `auto`, which are not in the list |
| cursor asks no permission for edits | An approval-required app still sees files written unasked; commands do ask |
