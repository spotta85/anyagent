# Plan: close the remaining T3 port gaps (round 2)

Spec: [gaps.md](gaps.md). Round 1 is [gaps-plan.md](gaps-plan.md). This round closes every open row that is not an account action.

```
anyagent (tasks 1-6)  ──►  new binary + types  ──►  T3 fork (task 7)  ──►  live check (task 8)
```

## Scope

| # | Task | Repo | Gap rows it closes | Est. lines |
|---|---|---|---|---|
| 1 | Subagent info on the subagent tool | anyagent | Subagent progress | ~50 |
| 2 | MCP tool kind on ACP agents; codex MCP secrets off argv | anyagent | 2 rows | ~45 |
| 3 | Where a slash command comes from | anyagent | Workspace skills | ~30 |
| 4 | Live events: tool progress, turn diff, model rerouted | anyagent | 1 row (3 events) | ~60 |
| 5 | Answers: deny with a message, withdraw a request | anyagent | 2 rows | ~40 |
| 6 | codex status check without a thread; output schema for generation | anyagent | 2 partly-fixed rows | ~100 |
| 7 | T3 adapter uses tasks 1-6 | t3code | wiring | ~150 |
| 8 | Live check rows for the new features, docs | t3code + anyagent | verification | ~100 |

Stays open after this round, on purpose:

| Row | Why |
|---|---|
| In-app login, redeem a reset credit, feedback upload | Account actions. The owner decides |
| Banked resets on claude | Needs the login token; the CLI does not report them |
| MCP on antigravity's native CLI | `agy` has no per-session MCP config (its ACP server takes them) |
| Status check without a session on ACP agents | Their login state is only known from `session/new` |

## Global Constraints

These bind every task.

1. **Simplest code that gives the exact behavior.** No speculative options, no new dependencies, no new files unless the task names one.
2. **Comments:** every function gets a 1-2 line comment saying what it does. A non-trivial block inside a function may get one line. Nothing longer.
3. **Layout:** main functions on top, helpers below them. Match the surrounding code's naming and idiom.
4. **Wire changes are additive.** New fields use `#[serde(default, skip_serializing_if = ...)]`. `PROTOCOL` stays `1`. Old transcripts must still load.
5. **Never guess a wire fact.** Before mapping an agent frame or sending a new request, confirm its shape against the real agent (a probe script or a live test), the agent's own generated schema, or an existing recorded fixture in `tests/`. Cite the source in a short comment (agent version, date). Probe scripts print debug lines and write their output to a temp file. If the agent cannot be run, say so in the report; do not invent the shape.
6. **anyagent never reads an agent's credentials and never calls a vendor HTTP API.** Everything comes from the agent's own process.
7. **No secret in argv, an error message, a `Diagnostic`, or `Debug` output.**
8. **Checks for anyagent tasks:** `just check` (fmt, clippy `-D warnings`, `cargo test`) and `cargo test --features schema --test schema` must pass. When a wire type changed: `just schema`, `just types`, and `npm run types` in `packages/node/anyagent` (run `npm install --no-package-lock` there first; the package has no lockfile by design); commit the regenerated files.
9. **Wrappers:** when a sidecar command gains a field, the hand-written method for that command in `packages/node/anyagent/src/index.ts` gains the matching optional argument. Other language wrappers only get regenerated types.
10. **Docs:** each task updates the rows it changes in `docs/core-api.mdx`, `docs/features.mdx`, `docs/sidecar.mdx`, `docs/agents.mdx` (only where the feature is listed). Short, tables, no essays.
11. **Commits:** small, one per logical change, imperative subject, no co-author line, no push.
12. **Live tests** go in `tests/live.rs` behind the existing `ANYAGENT_LIVE` gate, one feature per test. Run with `just live <agent> <feature>`. Keep claude live runs short: the account is near its limits.
13. **Do not touch** `gaps.md`, `README.md` numbers, the round 1 plan, or the T3 repo unless the task says so.

---

## Task 1: Subagent info on the subagent tool

**Goal:** an app's agents panel can show who each subagent is and what it is doing.

**Public API** (`src/event.rs`):

```rust
pub struct ToolUpdate {
    // existing fields unchanged
    /// For a `Subagent` tool: who runs and what it reports.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subagent: Option<SubagentInfo>,
}

/// What a subagent tool reports about the agent it spawned.
pub struct SubagentInfo {
    /// The kind of agent, as the parent named it ("general-purpose").
    pub role: Option<String>,
    pub model: Option<String>,
    /// Its latest progress line.
    pub summary: Option<String>,
    /// Token count the agent reports for this subagent so far.
    pub tokens: Option<u64>,
}
```

`SubagentInfo` derives the same traits as `FileDiff` plus `Default`, and is exported from `lib.rs`. Every field uses `skip_serializing_if = "Option::is_none"`.

**Behavior:**

| Agent | Source | Mapping |
|---|---|---|
| claude | the `Agent` / `Task` tool's input (`subagent_type`, `model`) and the system frames about tasks (`task_started`, `task_progress`, `task_notification`) | `role`, `model` from the input; `summary` and `tokens` from the progress frames. Each change re-emits the tool's `ToolUpdated` snapshot |
| codex | the child thread's `thread/tokenUsage/updated` | `tokens` only; codex 0.154.0 sends no role, model or summary |
| others | unchanged | `subagent: None` |

Confirm every frame and field first (constraint 5). A field the wire does not carry stays `None`.

A subagent that runs in the background: its tool must stay `Running` until the agent reports it finished. If the adapter marks it `Completed` at launch today, fix that and say so in the report.

Keep the change to `ToolUpdate` cheap for the rest of the code: every place that builds a `ToolUpdate` must set the new field; prefer one small constructor or `..` with an existing helper over editing dozens of literals.

**Tests:** fixture tests for claude and codex with recorded frames: role and model at start, a summary and tokens after a progress frame, the final state. One test that a background subagent's tool is still `Running` when the turn ends and is listed in `TurnEnded.background`. Live: extend the existing subagent live test for claude to print and assert `subagent.role`.

**Files:** `src/event.rs`, `src/lib.rs`, `src/adapter/claude.rs`, `src/adapter/codex.rs`, every file that builds a `ToolUpdate`, tests, docs, regenerated schema and types.

---

## Task 2: MCP tool kind on ACP agents; codex MCP secrets off argv

Two small fixes. One commit each.

### 2a. ACP

An ACP agent's MCP tool call is `ToolKind::Other` today, although the call's `_meta.mcp` names the server and the tool (seen on antigravity's ACP server, 2026-09-27). Map it to `ToolKind::Mcp { server, tool }`. Confirm the `_meta` shape on at least one installed ACP agent; agents that send no such `_meta` are unchanged.

### 2b. codex

The codex adapter passes a declared MCP server's stdio `env` values and its non-bearer header values as `-c mcp_servers.<name>...` overrides in argv (the bearer token already travels in an env var). Move every such value into the app-server process's environment and name it in the override, the way the bearer token is done. Use codex's own config keys for "take this value from an environment variable" (check codex's config reference or generated schema for the key names for stdio env and for HTTP headers). If codex has no such key for one of the two, leave that one as it is, document it in `docs/agents.mdx`, and say so in the report.

**Tests:** fixture test for 2a with a recorded tool call. For 2b an argv test (the codex fixture logs its argv when `FIXTURE_ARGV_LOG` is set): no secret value in argv, the names are there, and the fixture process received the values in its env. Live: `just live codex mcp_server_tools_are_called` still passes.

**Files:** `src/adapter/acp.rs`, `src/adapter/codex.rs`, `tests/acp.rs`, `tests/codex.rs`, fixtures, `docs/agents.mdx`.

No wire type changes in this task.

---

## Task 3: Where a slash command comes from

**Goal:** an app's skill picker can tell a skill from a built-in command and show where the skill lives.

**Public API** (`src/agent.rs`):

```rust
pub struct SlashCommand {
    pub name: String,
    pub description: String,
    pub input_hint: Option<String>,
    /// Where the command comes from.
    #[serde(default)]
    pub source: CommandSource,
}

#[non_exhaustive]
pub enum CommandSource {
    /// Part of the agent.
    #[default]
    Builtin,
    /// A skill on disk. `scope` is the agent's own word for where it lives ("user", "repo").
    Skill { path: Option<PathBuf>, scope: Option<String> },
}
```

Exported from `lib.rs`.

**Behavior:**

| Agent | Source | Mapping |
|---|---|---|
| codex | `skills/list` (each entry has a name, a path, a scope, an enabled flag) | `Skill { path, scope }`; a disabled skill is not listed, as today |
| claude | whatever its `initialize` reply or a control request says about which commands are skills | `Skill` where the CLI marks one, with the path and scope it gives; `Builtin` otherwise |
| others | unchanged | `Builtin` |

Confirm each source first (constraint 5). Where an agent does not say which commands are skills, everything stays `Builtin`; do not infer from names.

`Runtime::probe_with` in a workspace's dir already returns that workspace's commands; with this task they carry their source. Say so in `docs/core-api.mdx`.

**Tests:** fixture tests for codex and claude. One test that an old stored `SlashCommand` without `source` still loads as `Builtin`.

**Files:** `src/agent.rs`, `src/lib.rs`, `src/adapter/codex.rs`, `src/adapter/claude.rs`, every file that builds a `SlashCommand`, tests, docs, regenerated schema and types.

---

## Task 4: Live events

**Goal:** three things an app could only guess before.

**Public API** (`src/event.rs`, in `EventKind`):

```rust
/// A running tool reports what it is doing, or for how long it has run.
ToolProgress {
    tool_id: ToolId,
    message: Option<String>,
    elapsed_ms: Option<u64>,
},
/// The whole turn's changes so far as one unified diff; replaces the previous one.
TurnDiff { unified: String },
/// The agent answered with another model than the selected one.
ModelRerouted {
    from: String,
    to: String,
    reason: Option<String>,
},
```

They ride a running turn and never open one.

**Behavior:**

| Event | Agent | Source |
|---|---|---|
| `ToolProgress` | claude | the `tool_progress` frame (tool use id, elapsed seconds) |
| `ToolProgress` | codex | `item/mcpToolCall/progress` (item id, message) |
| `TurnDiff` | codex | `turn/diff/updated` |
| `ModelRerouted` | codex | `model/rerouted`. The warning `Diagnostic` it produces today is removed: one fact, one event |
| all three | others | not sent |

Confirm each frame first (constraint 5). `ToolProgress` for a tool the adapter does not track is dropped. A `ToolProgress` from a subagent's tool carries the parent tool id like other nested content.

**Tests:** fixture tests per source. One engine test that the three kinds are attributed to the running turn. Live: a codex turn that edits a file asserts at least one `TurnDiff` whose text names the file.

**Files:** `src/event.rs`, `src/session.rs`, `src/adapter/claude.rs`, `src/adapter/codex.rs`, `src/adapter/mock.rs` (only if script steps must name the new kinds), tests, docs, regenerated schema and types.

---

## Task 5: Answers: deny with a message, withdraw a request

**Goal:** an app can tell the agent why it said no, and can take a request back without choosing.

**Public API** (`src/event.rs`):

```rust
pub enum Answer {
    Permission(PermissionChoice),
    Question(Vec<QuestionAnswer>),
    /// Deny a permission once and tell the agent why.
    Deny { message: String },
    /// Take the request back without choosing. The agent stops waiting for it.
    Cancel,
}
```

**Engine rules** (`src/session.rs`, where answers are validated):

| Answer | Valid for | Otherwise |
|---|---|---|
| `Deny { message }` | a permission request that offers `DenyOnce` | `InvalidRequest`, the request stays open |
| `Cancel` | any open request | – |

**Behavior per agent:**

| Agent | `Deny { message }` | `Cancel` |
|---|---|---|
| claude | the deny reply with the caller's message in place of the fixed text | the deny reply with its interrupt flag set |
| codex | `decline` (the wire takes no message); for an MCP approval, `decline` | `cancel` (the decision codex has for it); for an MCP approval, action `cancel`; for a question, the empty answers reply the adapter sends on cancel today |
| ACP agents | the reject option (no message on the wire) | outcome `cancelled` |
| opencode | its reject reply, with the message if the route takes one | its reject reply |
| others with requests | the closest deny | the closest deny |

Confirm each reply shape first (constraint 5). Where the wire takes no message, the message is dropped silently; say which agents take it in `docs/agents.mdx`.

`Runtime::generate`'s own declining of requests is unchanged.

**Tests:** engine tests through the mock adapter: `Deny` on a request without `DenyOnce` is refused and the request stays open; `Cancel` closes a permission and a question. Fixture tests per adapter for the reply sent. Live: claude denies with the message "use the word PINEAPPLE in your reply" and the reply contains it.

**Files:** `src/event.rs`, `src/session.rs`, `src/adapter/{claude,codex,acp,opencode,mock}.rs`, tests, docs, regenerated schema and types.

---

## Task 6: codex status check without a thread; output schema for generation

Two parts. One commit each.

### 6a. codex probe opens no thread

Today `probe` on codex starts a throwaway thread, which starts the user's MCP servers and hooks on every background refresh. Make `Runtime::probe`, `probe_with` and `probe_auth` on codex stop before `thread/start`.

| Detail | From |
|---|---|
| version, login state, account | `initialize`, `account/read` (as today) |
| models, efforts, fast tiers, per-model options | `model/list` (as today); the current model is the catalog's default |
| commands | `skills/list` for the probe's dir |
| `mode`, `sandbox` | the same choices as today; `current` from codex's effective config if a request for it exists, otherwise codex's documented defaults |
| capabilities | as today |

The result must list the same models, options and commands as the thread-based probe on this machine. Show the comparison in the report (before and after, as the round 1 claude isolation did). If a detail cannot be had without a thread, keep the thread for `probe` and `probe_with`, make only `probe_auth` thread-free, and say so.

`open` and `generate` are unchanged.

### 6b. Output schema

```rust
// src/agent.rs, on SessionOptions
/// A JSON schema the agent's final message of each turn must match.
pub fn output_schema(mut self, schema: serde_json::Value) -> Self

// Capability
/// `SessionOptions::output_schema` is honored.
OutputSchema,
```

| Agent | How |
|---|---|
| claude | its JSON-schema launch flag (a schema is not a secret, so a flag is fine) |
| codex | the output-schema field of `turn/start` |
| others | `open` fails with `UnsupportedFeature("output schema")`; the capability is not advertised |

Sidecar: `open` and `generate` take `output_schema`. Node wrapper: the matching option.

Confirm both agents' field or flag first (constraint 5).

**Tests:** 6a: a fixture test that a codex probe sends no `thread/start`. 6b: argv test for claude, params test for codex, a typed failure on the mock or an ACP fixture. Live: `generate` on claude and codex with a schema `{ "type": "object", "properties": { "title": { "type": "string" } }, "required": ["title"] }` returns text that parses as JSON with a `title`.

**Files:** `src/agent.rs`, `src/runtime.rs`, `src/sidecar.rs`, `src/adapter/codex.rs`, `src/adapter/claude.rs`, tests, `packages/node/anyagent/src/index.ts`, docs, regenerated schema and types.

---

## Task 7: T3 adapter uses the new features

Repo: `/Users/spotta/Desktop/Projects/t3code`, branch `anyagent`.

| T3 feature | Today | After |
|---|---|---|
| `tool.progress` | never | from `ToolProgress` |
| `turn.diff.updated` | never | from `TurnDiff` |
| `model.rerouted` | a warning | from `ModelRerouted`; the warning is not shown twice |
| `task.started` role, model; `task.progress` | title and status only | from the tool's `subagent` info |
| `respondToRequest(.., "cancel")` | `DenyOnce` | `Answer::Cancel` |
| claude plan exit | declined with anyagent's fixed text | `Deny` with upstream T3's text ("the client captured your proposed plan; stop here and wait for the user's feedback") |
| Composer skill picker | empty | the snapshot's `skills` from the workspace probe's commands whose source is a skill |
| Title, branch, commit, PR text | JSON dug out of free text | `output_schema` where the agent advertises it; the free-text path stays for the others |

Checks: adapter tests per row, the server suite, typecheck, lint, fmt.

## Task 8: Live check rows, docs

1. Port-check rows: `tool-progress` or `turn-diff` (codex), `subagent-info` (claude), `cancel-request`, `skills`, `schema-generate`. Each asserts the feature itself; every SKIP is decided from facts.
2. Run them on every kind that is installed, logged in and has quota.
3. Update `docs/anyagent-port.md` in T3. Report the final `measure.sh` output.
