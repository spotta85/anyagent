# Plan: close the T3 port gaps

Spec: [gaps.md](gaps.md). This plan picks the rows worth having and says exactly what each one adds.

```
anyagent repo (tasks 1-8)  ──►  new `anyagent` binary + types  ──►  T3 fork (tasks 9-10)
```

## Scope

| # | Task | Repo | Gap rows it closes | Est. lines |
|---|---|---|---|---|
| 1 | Banked resets in plan usage | anyagent | new (user request) | ~50 |
| 2 | Turn token usage on opencode and pi | anyagent | per-agent hole | ~40 |
| 3 | Correctness: quiet codex, narrow `ResumeFailed`, rollback confirmation, cancel one turn | anyagent | 4 rows | ~90 |
| 4 | `AcceptEdits` permission mode, `Denied` tool status | anyagent | 2 rows | ~45 |
| 5 | Plan mode and `PlanProposed` | anyagent | 1 row | ~80 |
| 6 | Launch options and sidecar exposure | anyagent | instructions, per-instance binary/env, wire log, images in generate, workspace probe, isolated throwaway (claude) | ~120 |
| 7 | Per-model options on model choices | anyagent | 1 row | ~50 |
| 8 | Client MCP servers on opencode | anyagent | 1 row (opencode half) | ~50 |
| 9 | T3 adapter uses tasks 1-8 | t3code | wiring | ~200 |
| 10 | T3 live check on all six kinds, docs | t3code + anyagent | verification | ~100 |
| 11 | codex asks before an MCP tool call | anyagent | found by Task 8's live test | ~60 |
| 12 | claude MCP server secrets stay out of argv; the wire recording redacts them | anyagent | found by the final review | ~80 |

Left out on purpose (stay open in gaps.md, with the reason):

| Row | Why not now |
|---|---|
| Withdraw a permission request | `answer(DenyOnce)` then `cancel_turn` does the same thing |
| In-app login, redeem a reset credit, feedback upload | Account actions; bigger design, decide separately |
| Tool progress, model rerouted, live turn diff, subagent progress | Nice-to-have events; no app behavior depends on them |
| Output schema for `generate` | Free-text JSON works today |
| Session-free probe for codex and ACP | Their details come from the thread/session reply |
| MCP on antigravity | `agy` has no per-session MCP config |

## What changed while building

The live checks and reviews corrected the plan in these places. The task text below is the original; this table wins.

| Task | Plan said | What the real agent does, and what was built |
|---|---|---|
| 1 | claude reports banked resets in `get_usage` | The field is `null` on CLI 2.1.283. The parser is in place; claude shows none until the CLI passes it through |
| 2 | opencode and pi | antigravity too: a live probe showed how its thinking and cache tokens are counted |
| 3a | a `configWarning` names our flag | It arrives as a `warning` frame; the filter matches the text in either frame |
| 3d | `cancel_turn(turn)` | `cancel_turn(turn, clear_queue)`; a stale turn id does nothing at all |
| 4b | claude's frame has `decision_reason`; rules and hooks | The frame has `message`; it is sent for deny rules and the CLI's own mode. A hook refusal stays `Failed` |
| 5 | `mode` is what the caller set | claude leaves plan mode by itself after an allowed plan, and the adapter follows it. A resumed codex thread states its mode on its first turn |
| 5 | permission modes unchanged | The request that approves a proposed plan always reaches the caller, even under `AutoApprove` |
| 6 | claude `--append-system-prompt` | Instructions ride claude's `initialize` request, off the command line |
| 6 | prepend to the first prompt | A first prompt that starts with `/` is sent untouched; the instructions go with the next one |
| 8 | reuse the MCP live test server | None existed. A stdio fixture server and the live test `mcp_server_tools_are_called` were added |

## Global Constraints

These bind every task.

1. **Simplest code that gives the exact behavior.** No speculative options, no new dependencies, no new files unless the task names one.
2. **Comments:** every function gets a 1-2 line comment saying what it does. Non-trivial blocks may get one line. Nothing longer.
3. **Layout:** main functions on top, helpers below them. Match the surrounding code's naming and idiom.
4. **Wire changes are additive.** New fields use `#[serde(default, skip_serializing_if = ...)]`. `PROTOCOL` stays `1`. Old transcripts must still load.
5. **Never guess a wire fact.** Before mapping an agent frame or sending a new request, confirm its shape against the real agent (a probe script or a live test) or an existing recorded fixture in `tests/`. Probe scripts print debug lines and write their output to a temp file. If the agent is not installed or logged in, say so in the report; do not invent the shape.
6. **anyagent never reads an agent's credentials and never calls a vendor HTTP API.** Everything comes from the agent's own process.
7. **Checks for anyagent tasks:** `just check` (fmt, clippy `-D warnings`, `cargo test`) must pass. When a wire type changed: `just schema`, `just types`, and `npm run types` in `packages/node/anyagent`; commit the regenerated files.
8. **Wrappers:** when a sidecar command gains a field, the hand-written method for that command in `packages/node/anyagent/src/index.ts` gains the matching optional argument. Other language wrappers only get regenerated types.
9. **Docs:** each task updates the rows it changes in `docs/core-api.mdx`, `docs/features.mdx`, `docs/sidecar.mdx`, `docs/agents.mdx` (only where the feature is listed). Short, tables, no essays.
10. **Commits:** small, one per logical change, imperative subject, no co-author line, no push.
11. **Live tests** go in `tests/live.rs` behind the existing `ANYAGENT_LIVE` gate, one feature per test. Run with `just live <agent> <feature>`.
12. **Do not touch** `gaps.md`, `README.md` numbers, or the T3 repo unless the task says so.

---

## Task 1: Banked resets in plan usage

**Goal:** `plan_usage` also says how many banked limit resets the account has.

**Public API** (`src/event.rs`):

```rust
pub struct PlanUsage {
    pub plan: Option<String>,
    pub windows: Vec<UsageWindow>,
    /// Banked limit resets on the account. `None` when this report does not carry them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reset_credits: Option<ResetCredits>,
    pub fetched_at: SystemTime,
}

/// Limit resets the account can use now.
pub struct ResetCredits {
    pub available: u32,
    /// When the next one to be used expires.
    pub next_expires_at: Option<SystemTime>,
}
```

`ResetCredits` derives the same traits as `UsageWindow` and is exported from `lib.rs`.

**Behavior:**

| Agent | Source | Mapping |
|---|---|---|
| codex | `account/rateLimits/read` result, field `rateLimitResetCredits: { availableCount, credits: [{ status, expiresAt }] }` (sibling of `rateLimits`; live-verified 2026-09-26) | `available = availableCount`; `next_expires_at` = the smallest `expiresAt` (epoch seconds) among credits with `status == "available"` |
| codex | same result, `rateLimitsByLimitId.codex` | When present, read the windows from it instead of `rateLimits` (the legacy field can name another limit) |
| codex | `account/rateLimits/updated` notification | Has no credits: `reset_credits: None` |
| claude | `get_usage` reply, `rate_limits.cedar_ember` | When it is an object with `eligible: true`: grants that are not `paused`, have `usable_now: true`, and whose `ends_at` is null or in the future are live. If `next_grant_id` names a live grant: `available` = sum of live `resets_left`, `next_expires_at` = that grant's `ends_at`. Otherwise `available = 0`. When the field is null or not eligible: `None` |
| claude | `get_usage` request | Send `skip_behaviors: true` (the CLI then skips a scan of every transcript from the last 7 days) |

Known limit, to write in `docs/features.mdx`: claude CLI 2.1.283 returns `cedar_ember: null` on this request, so claude reports `None` until the CLI passes the block through.

**Also:** `anyagent list` (`src/bin/anyagent.rs`): if it prints quota windows, print `resets: N` beside them when `reset_credits` is `Some`. If it prints no quota, change nothing.

**Tests:** unit tests beside each parser using fixture JSON: codex with 0 credits, codex with 2 credits (one `available`, one `redeemed`), codex notification (None), claude null, claude eligible with two live grants, claude paused grant. Live: extend the existing plan-usage live test to print `reset_credits`.

**Files:** `src/event.rs`, `src/lib.rs`, `src/adapter/codex.rs`, `src/adapter/claude.rs`, `src/bin/anyagent.rs` (maybe), `tests/live.rs`, docs, regenerated schema and types.

---

## Task 2: Turn token usage on opencode and pi

**Goal:** `TurnEnded.usage` is filled for opencode and pi, the same way claude and codex fill it.

**Behavior:** the adapter sends `DriverEvent::TurnUsage(TurnUsage)` before `TurnEnded`. The engine already attaches the latest one to the turn and drops usage outside a running turn.

| Agent | Source | Mapping |
|---|---|---|
| opencode | `tokens` on the step-finish part (already read for `ContextUsage`) | Sum over the turn's steps: `input_tokens` = input + cache read + cache write, `cached_input_tokens` = cache read, `output_tokens` = output + reasoning. Reset at `StartTurn` |
| pi | `usage` on the assistant message (already read for `ContextUsage`) | Same three sums from pi's own field names (confirm them from `tests/pi.rs` fixtures or a live run). Reset at `StartTurn` |
| antigravity | only if its wire reports token counts per turn | Same; if it reports none, leave it and say so in the report |

ACP agents report only context fill; they stay `None`.

**Tests:** one fixture test per adapter asserting the `TurnEnded.usage` numbers; one asserting a second turn does not inherit the first turn's sums. Live: extend `turn_usage_rides_turn_ended` to run for opencode and pi.

**Files:** `src/adapter/opencode.rs`, `src/adapter/pi.rs`, `tests/opencode.rs`, `tests/pi.rs`, `tests/live.rs`, `docs/agents.mdx`.

No wire type changes in this task.

---

## Task 3: Correctness fixes

Four small fixes. One commit each.

### 3a. Quiet codex sessions

anyagent sets `features.default_mode_request_user_input=true` at launch, so codex warns about it on every open. After `thread/revert` codex warns that the thread "was recorded with model X but is resuming with Y" although the session's model did not change.

| Warning | Fix |
|---|---|
| `configWarning` that names only flags the adapter itself set | Drop it. A `configWarning` that names anything else still becomes a `Diagnostic` |
| Model mismatch after `thread/revert` | First choice: send the session's current model with the revert if the request takes one. Otherwise drop that warning when it arrives after a revert and the session's model is unchanged |

Confirm both messages live first (`just live codex`), then add fixture tests with the real text.

### 3b. Narrow `ResumeFailed`

| Agent | Today | Fix |
|---|---|---|
| codex | any RPC error on `thread/resume` → `ResumeFailed` | Only the error codex returns for an unknown thread id. Probe it live with a made-up id. Everything else stays the error it is |
| opencode | any HTTP error on `GET /session/{id}` → `ResumeFailed` | Only `404`. Other statuses and connect errors keep their own error |

### 3c. Rollback confirmation

Today `Session::rollback` resolves when the command is forwarded, and a refusal arrives later as a `Diagnostic`.

**New behavior:** `rollback` resolves after the adapter reports the outcome.

```rust
// src/adapter/mod.rs
/// Outcome of the last `Rollback` command; `Err` carries the agent's reason.
DriverEvent::RolledBack(Result<(), String>)
```

| Piece | Rule |
|---|---|
| Engine | Holds the caller's reply until `RolledBack` arrives. `Ok` → `Ok(())`. `Err(reason)` → `AgentError::InvalidRequest(reason)`. Driver gone while waiting → `AgentError::SessionClosed` |
| Engine | A second `rollback` while one is pending → `AgentError::SessionBusy` |
| Every adapter with `Capability::Rollback` | Sends `RolledBack` on every path of its rollback, once. The refusal `Diagnostic`s those paths send today are removed (the reason is now the error) |
| Mock adapter | Script step to refuse a rollback, so conformance can test both outcomes |

### 3d. Cancel one turn

```rust
// src/session.rs
/// Cancels `turn` only if it is the running turn; otherwise does nothing.
pub async fn cancel_turn(&self, turn: TurnId) -> Result<(), AgentError>
```

Sidecar: `cancel` gains optional `turn`. With `turn` set and not matching the running turn, the reply is ok and nothing is cancelled. `clear_queue` keeps its meaning. Node wrapper: `cancel(clearQueue = false, turn?: string)`.

**Tests:** engine tests through the mock adapter (`src/adapter/conformance.rs`): rollback ok, rollback refused, rollback while pending, cancel with a stale turn id leaves the running turn alive, cancel with the right id ends it `Cancelled`. Fixture tests for 3a and 3b.

**Files:** `src/session.rs`, `src/sidecar.rs`, `src/adapter/mod.rs`, `src/adapter/{claude,codex,opencode,acp,antigravity,mock}.rs` (those that implement rollback), `src/adapter/conformance.rs`, tests, `packages/node/anyagent/src/index.ts`, docs.

---

## Task 4: `AcceptEdits` and `Denied`

### 4a. `PermissionMode::AcceptEdits`

```rust
pub enum PermissionMode {
    Ask,
    /// Allow file edits once without asking; forward every other request.
    AcceptEdits,
    AutoApprove,
}
```

Engine rule: a permission request whose `tool.kind` is `Edit`, `Delete` or `Move` and that offers `AllowOnce` is answered `AllowOnce` and never shown. Everything else is forwarded as in `Ask`. Reuse the `AutoApprove` branch in `handle_content`; do not duplicate it.

### 4b. `ToolStatus::Denied`

```rust
pub enum ToolStatus { Pending, Running, Completed, Failed, Cancelled,
    /// Refused by a rule or hook without asking the caller.
    Denied }
```

| Agent | Source | Mapping |
|---|---|---|
| claude | system frame `permission_denied` (`tool_name`, `tool_use_id`, `decision_reason`) | `ToolUpdated` for that tool with `status: Denied` and `output: Some(decision_reason)` |

Confirm the frame live or from a fixture first. Other agents are unchanged. `is_active()` stays false for `Denied`.

**Tests:** conformance tests through the mock adapter for `AcceptEdits` (an Edit request is answered, an Execute request is forwarded, an Edit request without `AllowOnce` is forwarded). Fixture test for the claude frame.

**Files:** `src/agent.rs`, `src/event.rs`, `src/session.rs`, `src/adapter/claude.rs`, `src/adapter/conformance.rs`, `tests/claude.rs`, docs, regenerated schema and types.

---

## Task 5: Plan mode and `PlanProposed`

**Goal:** one way to put an agent in plan mode and one event that carries the plan it proposes.

**Public API:**

```rust
// src/event.rs, in EventKind
/// The agent proposes this plan and waits for a go-ahead.
PlanProposed { markdown: String },
```

Well-known `mode` choice: the value `plan` means plan mode on every agent that has one. Document it next to the well-known option ids in `src/agent.rs`.

| Agent | Entering plan mode | Proposed plan |
|---|---|---|
| claude | already `mode: plan` | When the `ExitPlanMode` tool is requested: emit `PlanProposed { markdown: input.plan }`, then the normal `RequestOpened(Permission)` for that tool. Allow = implement, Deny = keep planning |
| codex | add `plan` to the `mode` choices. While selected, every `turn/start` carries `collaborationMode: { mode: "plan", settings: { model, reasoning_effort, developer_instructions: null } }`; the approval policy keeps the last non-plan value. Selecting another mode sends `collaborationMode.mode: "default"` with the same settings | When an item of type `plan` completes: `PlanProposed { markdown: <the item's text> }` |
| antigravity, ACP agents | unchanged (they advertise their own modes) | none |

Confirm codex live first: that the request shape is accepted, that `null` instructions select codex's built-in plan prompt, and the exact item type and text field of a plan item. If codex refuses the shape, stop and report; do not invent another.

`PlanProposed` is turn content: add it to `is_content`.

**Tests:** fixture tests for both adapters (claude: event order `PlanProposed` then `RequestOpened`; codex: `turn/start` params in plan mode and back, and the plan item). Live: one `plan_mode_proposes_a_plan` test for claude and codex with a prompt like "plan how to add a README, do not write files".

**Files:** `src/event.rs`, `src/agent.rs`, `src/session.rs`, `src/adapter/claude.rs`, `src/adapter/codex.rs`, `src/adapter/mock.rs` (a script step for the event, if the mock lists content kinds), tests, docs, regenerated schema and types.

---

## Task 6: Launch options and sidecar exposure

**Goal:** an app can pick the binary, environment, extra args, instructions, config home and wire log per session, from Rust and from `serve`.

**Public API** (`src/agent.rs`, on `SessionOptions`):

```rust
/// Extra instructions for the agent, added to its system prompt.
pub fn instructions(mut self, text: impl Into<String>) -> Self
/// One environment variable for the agent process, over the inherited ones.
pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self
/// One extra launch argument, placed after anyagent's own.
pub fn arg(mut self, arg: impl Into<String>) -> Self
```

`src/runtime.rs`:

```rust
/// `probe` with the caller's options: dir, env, args, config home. Always throwaway.
pub async fn probe_with(&self, agent: &AgentInstallation, options: SessionOptions) -> Result<AgentDetails, AgentError>
/// `plan_usage` for the login these options point at.
pub async fn plan_usage_with(&self, agent: &AgentInstallation, options: &SessionOptions) -> Result<PlanUsage, AgentError>
```

`probe` and `plan_usage` keep their signatures and call the `_with` forms with default options. The usage cache key gains the config home and env. `Adapter::plan_usage` gains an `options: &SessionOptions` argument; claude and codex apply config home, env and args to the short-lived process.

**Behavior per agent:**

| Option | claude | codex | pi | opencode | ACP agents, antigravity |
|---|---|---|---|---|---|
| `instructions` | `--append-system-prompt` | `developerInstructions` on thread start, resume and fork | its append-system-prompt flag | `system` on each prompt | put before the first prompt's text of a new session, separated by a blank line |
| `env` | merged into the child's env after `config_home` | same | same | same | same |
| `arg` | appended | appended after `app-server` and anyagent's overrides | appended | appended | appended |

Confirm each field name against the agent before using it (constraint 5). With `instructions` and `resume` on an ACP agent nothing is prepended.

**Throwaway isolation (claude only):** a throwaway session (probe, generate) launches with user hooks off and user MCP servers off: `--strict-mcp-config` and `--settings '{"disableAllHooks":true}'`. If `--settings` is already used for `fast`, merge both into one JSON value. Confirm live that the probe still returns models, options and commands.

**Sidecar** (`src/sidecar.rs`):

| Command | New fields |
|---|---|
| any `agent` | third form `{ "id": "claude", "path": "/opt/claude" }` → `AgentInstallation::at` |
| `open`, `generate` | `instructions`, `env` (object), `args` (array), `config_home`, `record_wire` |
| `generate` | `attachments` (array of paths), forwarded like `prompt`'s |
| `probe` | optional `dir` (default: temp dir) plus `env`, `args`, `config_home` |
| `plan_usage` | `env`, `args`, `config_home` |

Keep one options struct; do not copy fields per command. `probe` and `plan_usage` given `resume` or `fork` fail with `InvalidConfiguration`.

Node wrapper: `probe(agent, options?)`, `planUsage(agent, options?)`, and the new `open`/`generate` fields.

**Tests:** sidecar tests (`tests/sidecar.rs`) through the mock for each new field reaching `SessionOptions`; fixture or argv tests per adapter for `instructions`, `env`, `arg`; claude throwaway flags. Live: `instructions_reach_the_agent` (instructions: "End every reply with the word PINEAPPLE", assert it) for claude, codex, and one ACP agent that is installed.

**Files:** `src/agent.rs`, `src/runtime.rs`, `src/sidecar.rs`, `src/adapter/mod.rs`, every adapter's launch, tests, `packages/node/anyagent/src/index.ts`, docs, regenerated schema and types.

---

## Task 7: Per-model options on model choices

**Goal:** a model picker can show each model's own options (effort levels, fast) before the model is selected.

**Public API** (`src/agent.rs`):

```rust
pub struct ConfigChoice {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
    /// For a `model` choice: the options this model offers once selected.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<ConfigOption>,
}
```

**Behavior:** on the `model` option, each choice's `options` holds that model's `effort` (its levels, `current` = the model's default level when the catalog names one, else `None`) and `fast` (when the model supports it). Choices of every other option keep `options` empty. Nested options never carry nested choices of their own.

| Agent | Catalog |
|---|---|
| claude | the `initialize` reply's models, already used by `set_effort_option` / `set_fast_option` |
| codex | `model/list`, already used the same way |
| opencode | model variants, if the adapter already reads them per model |
| others | unchanged (empty) |

Build the per-model list with the same helper that builds the live `effort` / `fast` options, so the two cannot disagree.

**Tests:** fixture tests: a model without effort has no `effort` in its `options`; a model with an extra level lists it; selecting that model makes the session's live `effort` option equal its nested one.

**Files:** `src/agent.rs`, `src/adapter/mod.rs`, `src/adapter/claude.rs`, `src/adapter/codex.rs`, `src/adapter/opencode.rs` (maybe), every place that builds a `ConfigChoice`, tests, docs, regenerated schema and types.

---

## Task 8: Client MCP servers on opencode

**Goal:** `open { mcp_servers }` works on opencode.

**Behavior:** after the session's server is up and before the first prompt, register each declared server through opencode's own API (expected `POST /mcp`; confirm the path and body from the running server's OpenAPI document first). Advertise the transports that work in `capabilities.mcp_transports`. A server opencode refuses fails `open` with `InvalidConfiguration` and the agent's message. Remove the current refusal branch.

If opencode keeps MCP servers per server process and not per session, register at open and say so in `docs/agents.mdx`.

**Tests:** fixture test with the recorded request. Live: `mcp_server_tools_are_called` for opencode, reusing the MCP test server the claude and codex live tests use.

**Files:** `src/adapter/opencode.rs`, `tests/opencode.rs`, `tests/live.rs`, `docs/agents.mdx`.

No wire type changes in this task.

---

## Task 11: codex asks before an MCP tool call

**Found by:** the live test `mcp_server_tools_are_called` (added in Task 8). It passes on claude and opencode and fails on codex.

**Goal:** an MCP tool call on codex reaches the caller as a permission request, like every other tool, and runs when allowed.

**Today:** codex 0.154.0 asks for approval of an MCP tool call with the server request `mcpServer/elicitation/request`. The codex adapter declines it, so the tool fails before it reaches the server.

**New behavior:**

| Piece | Rule |
|---|---|
| The request | `mcpServer/elicitation/request` for a tool call becomes `RequestOpened(Request::Permission)`. `tool` is the pending MCP tool call as the caller already saw it (`ToolKind::Mcp { server, tool }`). `options` lists only the choices the codex request can express |
| The answer | `Answer::Permission` maps back to the elicitation reply codex expects (accept / decline, and the for-session form only if the wire has one) |
| Permission modes | Nothing new. The engine already answers for `AutoApprove`, and `AcceptEdits` forwards it because the kind is `Mcp` |
| Cancel | A pending request is dropped on cancel like the adapter's other requests |
| Other elicitations | An elicitation that is not a tool-call approval (a server asking the user for form input) keeps today's behavior |

Confirm the request and reply shapes live first (constraint 5): record the wire of a codex session with the stdio MCP fixture (`tests/fixtures/mcp/server.mjs`), and check codex's generated schema (`codex app-server generate-json-schema`). If the request carries no link to the tool call item, say so in the report and build the `tool` from what the request does carry.

**Tests:** a fixture test in `tests/codex.rs` with the recorded frames: request opened, allow → the tool completes; deny → the tool ends not completed and the turn goes on. Live: `just live codex mcp_server_tools_are_called` passes.

**Files:** `src/adapter/codex.rs`, `tests/codex.rs`, `tests/fixtures/codex/fixture.mjs`, `docs/agents.mdx` (one line, only if the page lists what codex asks permission for).

No wire type changes in this task.

---

## Task 12: claude MCP server secrets stay out of argv

**Found by:** the final review. It is the same exposure that moved claude's instructions off the command line.

**Today:** the claude adapter passes the app's declared MCP servers as inline JSON in `--mcp-config` (`mcp_config` and `option_args` in `src/adapter/claude.rs`). That JSON holds stdio `env` values and http/sse header values (bearer tokens). Any local user can read them with `ps`.

**Goal:** declared MCP servers reach claude without their secrets appearing in the process arguments.

**How, in order of preference. Probe first (constraint 5), then pick the first one that works:**

| Option | What | Accept when |
|---|---|---|
| A | Send the servers on the control channel after `initialize`. The CLI lists a `mcp_set_servers` control request | A live session shows the declared server connected and its tool callable, the server is present before the first prompt, and it survives the rollback respawn (which relaunches the process) |
| B | Write the config to a file only the user can read (mode 0600, in the system temp dir) and pass its path to `--mcp-config`. Delete the file when the session closes and when launch fails | Option A does not work |

If option A works only partly (for example it cannot express one transport), say so in the report and use B for everything. Do not mix.

**Rules:**

1. The three transports (stdio, http, sse) keep working exactly as today, including `--strict-mcp-config` for throwaway sessions.
2. No secret in argv, in an error message, in a `Diagnostic`, or in `Debug` output. The opt-in wire recording (`record_wire`) may contain them; that file is already documented as unredacted.
3. No new dependency.
4. Stay inside how MCP config is passed. Do not touch other launch options.

**Tests:** a fixture test asserting the child's argv holds no header or env value (the claude fixture already logs its argv when `FIXTURE_ARGV_LOG` is set) and that the fixture received the servers. Live: `just live claude mcp_server_tools_are_called` passes.

**Docs:** one line in the claude section of `docs/agents.mdx` saying how MCP servers are passed. The codex section gets one line stating its known limit: stdio `env` and non-bearer headers of declared MCP servers travel as `-c` overrides in argv (the bearer token does not).

**Files:** `src/adapter/claude.rs`, `tests/claude.rs`, `tests/fixtures/claude/fixture.mjs`, `docs/agents.mdx`.

No wire type changes in this task.

---

## Task 9: T3 adapter uses the new features

Repo: `/Users/spotta/Desktop/Projects/t3code`, branch `anyagent`. Needs the binary built from anyagent's HEAD (`ANYAGENT_BIN`) and the regenerated `anyagent-ts` types.

| T3 call | Today | After |
|---|---|---|
| `sendTurn({ interactionMode: "plan" })` | fails with a validation error | `configure("mode", "plan")` before the turn; leaving plan restores the mode the runtime mode maps to |
| `turn.proposed.completed` | never | from `PlanProposed` |
| `runtimeMode: "auto-accept-edits"` | opens with `Ask` | `permission_mode: "AcceptEdits"` |
| `rollbackThread` | believes a refused rollback | awaits the reply; a refusal is the adapter's typed error |
| `interruptTurn(threadId, turnId)` | ignores `turnId` | `cancel` with `turn` |
| `tool.denied` | never | from `ToolStatus: Denied` |
| `turn.completed.payload.tokenUsage` | claude, codex | also opencode |
| Session instructions | none | `RuntimeInstructions.ts` text as `open { instructions }`; codex also gets `CodexDeveloperInstructions.ts`'s tool guide |
| Settings `binaryPath`, `environment`, `homePath`, `launchArgs` | ignored | agent `{ id, path }`, `env`, `config_home`, `args` on open, probe, generate and plan usage |
| Model picker option descriptors | current model's options copied to every model | from each choice's `options` |
| Usage limits | not wired | `plan_usage` → `usageLimits`, with `resetCredits` from `reset_credits` |
| Native wire log | not written | `record_wire` into the native log directory |
| `generate*` with attachments | names only | `attachments` |
| T3 MCP server on opencode | skipped | declared, now that opencode advertises the transport |

Checks: `vp run --filter t3 test` with `TMPDIR=/private/tmp/t3tmp`, typecheck, adapter tests for each row.

## Task 10: Live check on all six kinds, docs

1. Add rows to `scripts/anyagent-port-check.ts`: plan, accept-edits, instructions, usage-limits.
2. Run it for every kind that is installed and logged in: claude, codex, cursor, grok, opencode, antigravity. Output to `PORT_CHECK_OUT`.
3. Update `docs/ports/t3-code/README.md` (matrix) and `gaps.md` (status per row, new rows for anything the run finds) in anyagent, and `docs/anyagent-port.md` in T3.
