# Gaps found porting T3 Code to anyagent

Things T3 Code's server needed from its agent layer that anyagent did not have. Each row is one feature of T3 and what anyagent does about it today.

```
33 gaps found  ──►  26 fixed  ·  3 partly fixed  ·  4 open
```

How they were fixed: [gaps-plan.md](gaps-plan.md) (round 1), [gaps-plan-2.md](gaps-plan-2.md) (round 2).

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
| Workspace skills | the composer's skill picker | `SlashCommand.source`: `Builtin` or `Skill { path, scope }`. codex gives path and scope; claude gives scope only |
| Subagent progress | `task.progress`, role and model on `task.started` | `ToolUpdate.subagent { role, model, summary, tokens }`: claude fills all four, codex model and tokens, opencode role and model. A background subagent stays `Running` until it finishes |
| Live events | `turn.diff.updated`, `tool.progress`, `model.rerouted` | `TurnDiff`, `ToolProgress`, `ModelRerouted`. They ride a running turn and never open one. claude sends progress only when it runs remote or in a container |
| Deny with a message | T3 stops claude at its plan by declining the exit request | `Answer::Deny { message }`. claude and opencode pass the text to the model; codex and ACP have no field for it |
| Withdraw a request | `respondToRequest(.., "cancel")` | `Answer::Cancel`, valid for any open request |
| MCP tool kind on ACP agents | tool rows for T3's MCP tools | `_meta` names the server and tool on antigravity, kiro and qwen: `ToolKind::Mcp` |

## Partly fixed

| Feature | Fixed | Still missing | Why |
|---|---|---|---|
| Session-free status check | claude: no user hooks or MCP servers. codex: no thread at all | ACP agents still open a throwaway session | Their login state and commands only come with a session |
| Isolated one-shot generation | claude isolation; an output schema on claude and codex (`output_schema`, `Capability::OutputSchema`) | Isolation and schemas on the other agents | Their wire has neither |
| MCP secrets and servers | codex header and env values ride its environment, never argv | MCP on antigravity's native CLI | `agy` has no per-session MCP config; its ACP server takes MCP servers |

## Open

All four need the owner's decision; none is blocked by code.

| Feature | T3 call | What anyagent lacks | Proposed change | Size (lines) |
|---|---|---|---|---|
| Banked resets on claude | the usage panel's reset credits | The CLI's `get_usage` returns the block as `null` (2.1.283). The data needs the login token and a direct API call, which anyagent does not do. The parser is in place for when the CLI passes it through | Read the token, or wait for the CLI | ~40 |
| In-app login | `ProviderInstance.auth` | `LoginMethod::Terminal { command }` only; anyagent cannot run a login | `Runtime::login(agent, method)` streaming url, code, done, failed; `logout(agent)` | ~120 |
| Redeem a reset credit | `ProviderInstance.consumeResetCredit` | No account actions | `Runtime::redeem_reset(agent)` (codex) | ~40 |
| Feedback upload | `adapter.uploadFeedback` | No command | `Session::upload_feedback { reason }` (codex) | ~30 |

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
| A codex probe can miss skills | codex loads plugin skills about 0.5 s after it starts; a probe lists the ones loaded by then |
| A codex probe's `sandbox` in a trusted dir | Reads `read-only` where a thread would say `workspace-write`; `open` reports the real value |
| codex takes strict output schemas only | Every object needs `additionalProperties: false` and all properties required |
| A codex stdio MCP server cannot set `PATH`, or a name codex's environment holds with another value | codex forwards env by name; the open fails with the name |
| `tokens` on a subagent differs per agent | claude: the latest call's size. codex: the child thread's total |
| Nested text from a claude background subagent opens a turn | No recording shows one; tools and progress do not |
