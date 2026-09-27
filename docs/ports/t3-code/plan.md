# T3 Code on anyagent: port plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. One Opus 5.5 subagent per task, fresh reviewer between tasks. Steps use `- [ ]`.

**Goal:** T3 Code's server talks to every agent through one `AnyagentAdapter` over `anyagent-ts`, its own per-agent adapters are deleted, and every feature T3 needs that anyagent lacks is a row in `gaps.md`.

**Architecture:** T3 routes all agent traffic through one interface, `ProviderAdapterShape` (12 methods + one event stream). We implement that interface once on top of `anyagent-ts`, register it for the six agent kinds T3 ships, prove each feature with a live script, then delete the old adapters and count.

**Tech stack:** T3 = TypeScript + Effect (`Effect`, `Stream`, `Layer`), vitest via `vp test`. anyagent = Rust binary `anyagent serve` + `anyagent-ts` (thin typed pipe, `packages/node/anyagent`).

```
T3 server                         fork adds                      anyagent
ProviderService ──► ProviderAdapterShape ──► AnyagentAdapter ──► anyagent-ts ──► anyagent serve ──► claude/codex/cursor/grok/opencode/antigravity
  startSession                                 rt.open()
  sendTurn                                     session.prompt()
  interruptTurn                                session.cancel()
  respondToRequest / respondToUserInput        session.answer()
  rollbackThread                               session.rollback()
  stopSession / stopAll                        session.close() / rt.close()
  streamEvents  ◄──────────────────────────── session.events()  (EventKind → ProviderRuntimeEventV2)
```

## Where things are

| What | Path |
|---|---|
| T3 fork (branch `anyagent`, based on today's main `679c34c096`) | `/Users/spotta/Desktop/Projects/t3code` |
| T3 adapter contract | `apps/server/src/provider/Services/ProviderAdapter.ts` |
| T3 driver contract | `apps/server/src/provider/ProviderDriver.ts` |
| T3 event type | `packages/contracts/src/providerRuntime.ts` (`ProviderRuntimeEventV2`) |
| Reference driver to copy the shape of | `Drivers/GrokDriver.ts` → `Layers/GrokProvider.ts` (snapshot) → `Layers/GrokAdapter.ts` (adapter) |
| Driver registration | `apps/server/src/provider/builtInDrivers.ts` |
| anyagent repo | `/Users/spotta/Desktop/Projects/anyagent` |
| anyagent-ts source + README | `packages/node/anyagent/src/index.ts`, `packages/node/anyagent/README.md` |
| anyagent wire docs | `docs/sidecar.mdx`, `docs/core-api.mdx`, `docs/features.mdx` |
| anyagent binary (built with `--features mock`) | `target/release/anyagent` |
| Mock scripts for unit tests | `packages/mock-scripts/*.json` |
| Live feature list anyagent already proves per agent | `tests/live.rs` |
| Gaps doc (append rows as found) | `docs/ports/t3-code/gaps.md` |
| Baseline: toolchain, headless server, RPC surface, pre-port line counts | `docs/ports/t3-code/baseline.md` |

## Global constraints

- Node 26, pnpm 11. The server package is named `t3`. Typecheck from the fork root: `pnpm --filter t3 typecheck`. Tests from `apps/server`: `TMPDIR=/private/tmp/t3tmp npx vp test run [file]` (`mkdir -p /private/tmp/t3tmp` first; TMPDIR must be a real, short path). The only expected failure on clean main is `device/sshDeviceScript.test.ts`. Full suite is ~400s; run one file while iterating. Everything else about the toolchain, the headless server, and the RPC surface is in `docs/ports/t3-code/baseline.md`, read it before touching the server.
- `anyagent-ts` is linked from source: `"anyagent-ts": "link:../../../anyagent/packages/node/anyagent"` in `apps/server/package.json`. Binary via env `ANYAGENT_BIN=/Users/spotta/Desktop/Projects/anyagent/target/release/anyagent`.
- One `anyagent serve` process per T3 server. `Runtime` is an Effect service (`AnyagentRuntime`), shared by all six drivers.
- Never edit `ProviderAdapterShape`, `ProviderDriver`, or `ProviderRuntimeEventV2`. The port proves anyagent fits T3's seam as-is. If it cannot, that is a gaps row, not a T3 edit.
- Unit tests run against the mock binary, never a live agent. Live checks are the Task 3 script only.
- Every script writes its full log to a file under `/private/tmp/claude-501/-Users-spotta-Desktop-Projects-anyagent/25d99f8a-132e-4e0a-9dda-09d234fbca38/scratchpad/` and prints the path. Debug lines on every step.
- Commits on the fork branch `anyagent`, one per task, no co-author line. Nothing pushed.
- Comments: 1–2 lines per function saying what it does. Main flow at the top of a file, helpers below.
- A T3 feature anyagent cannot serve: keep the T3 call compiling (return `UnsupportedFeature`-style typed error or `capabilities` flag false), add a gaps row. Do not stub it silently.

## Review focus

Inputs the spec implies but no unit test covers; the reviewer of each task checks the owner added the pinned test.

1. Permission request arrives while a second turn is queued: the answer must target the right session and the queued turn must still run. (Task 1)
2. Agent process dies mid-turn: T3 gets a terminal turn event and the thread is marked stopped, not hung. (Task 1)
3. `rollbackThread` on an agent whose capabilities lack rollback: typed error, `supportsConversationRollback: false`, no crash. (Task 1)
4. `startSession` with a resume token for a session anyagent no longer knows: typed error surfaces to T3's thread UI as resumable-failed, not a silent fresh session. (Task 1)
5. Two agent kinds open at once (claude + codex) share one `anyagent serve`: events route by `session_id`, closing one does not close the other. (Task 2)

---

### Task 0: Baseline

**Files:** create `docs/ports/t3-code/baseline.md` (in anyagent repo).

- [ ] `pnpm install` finished (log: scratchpad `t3-install.log`). If `prepare` failed, fix and note it.
- [ ] `pnpm --filter @t3tools/server typecheck` passes on `main` and on `anyagent` (identical now).
- [ ] `pnpm --filter @t3tools/server test` passes. Record count and wall time.
- [ ] Record how to start the server headless and how a script reaches its API (find `apps/server/src/bin.ts`, the RPC/WS layer under `apps/server/src/`, and the provider/thread RPC names a client uses to: list providers, create thread, send turn, answer approval, interrupt, read thread). Write the exact endpoint names.
- [ ] Record line counts, non-test, hand-written vs generated:
  ```bash
  cd /Users/spotta/Desktop/Projects/t3code
  for d in Drivers Layers Services acp .; do printf "%-10s " $d; find apps/server/src/provider/$d -maxdepth 1 -name '*.ts' -not -name '*.test.ts' -not -name '*testFixtures*' | xargs wc -l | tail -1; done
  find packages/effect-acp/src packages/effect-codex-app-server/src -name '*.ts' -not -name '*.test.ts' -not -path '*_generated*' | xargs wc -l | tail -1
  find packages/effect-codex-app-server/src/_generated -name '*.ts' | xargs wc -l | tail -1
  find apps/server/src/provider -name '*.test.ts' | xargs wc -l | tail -1
  ```
- [ ] Commit nothing on the fork. Write `baseline.md` (short: commands, numbers, endpoint names).

### Task 1: `AnyagentAdapter` behind the seam, unit-tested on the mock binary

**Files:**
- Create `apps/server/src/provider/anyagent/AnyagentRuntime.ts` — Effect service wrapping one `anyagent-ts` `Runtime` (`Runtime.start({ bin: process.env.ANYAGENT_BIN, mock? })`), acquire/release with `Layer.scoped`.
- Create `apps/server/src/provider/anyagent/AnyagentEvents.ts` — pure function `toProviderRuntimeEvents(threadId, turnId, ev: anyagent Event): ReadonlyArray<ProviderRuntimeEventV2>`. One anyagent event may fan out to several T3 events.
- Create `apps/server/src/provider/anyagent/AnyagentAdapter.ts` — `makeAnyagentAdapter(kind: ProviderDriverKind, agent: string): Effect<ProviderAdapterShape<AnyagentAdapterError>, never, AnyagentRuntime>`.
- Create `apps/server/src/provider/anyagent/Errors.ts` — `AnyagentAdapterError` tagged error wrapping `AnyagentError` body.
- Test `apps/server/src/provider/anyagent/AnyagentEvents.test.ts`, `AnyagentAdapter.test.ts` (uses `Runtime.start({ bin, mock: ".../packages/mock-scripts/<script>.json" })`).
- Modify `apps/server/package.json`: add `anyagent-ts` link dep.

**Interfaces produced (Task 2 relies on these names):**
```ts
export class AnyagentRuntime extends Context.Tag("AnyagentRuntime")<AnyagentRuntime, { readonly runtime: import("anyagent-ts").Runtime }>() {}
export const AnyagentRuntimeLive: Layer.Layer<AnyagentRuntime>;
export function makeAnyagentAdapter(kind: ProviderDriverKind, agent: string): Effect.Effect<ProviderAdapterShape<AnyagentAdapterError>, never, AnyagentRuntime | Scope.Scope>;
export function toProviderRuntimeEvents(ctx: { threadId: ThreadId; turnId: TurnId | undefined }, ev: AnyagentEvent): ReadonlyArray<ProviderRuntimeEventV2>;
```

**Mapping (fill the table in code comments, one row per EventKind, from `docs/core-api.mdx`):**

| anyagent `EventKind` | T3 `ProviderRuntimeEventV2` |
|---|---|
| `TurnStarted` / first event after `prompt` | turn started |
| `TextDelta` | assistant text delta |
| `ReasoningDelta` | reasoning delta |
| `ToolUpdated` (statuses, diff) | tool call started / updated / completed |
| `RequestOpened { Permission }` | approval request (map `Answer::Permission` variants to T3 decisions) |
| `RequestOpened { Question }` | user-input request |
| `RequestClosed` | approval resolved |
| `PlanUpdated` | plan / todo items |
| `UsageUpdated` / plan usage | token usage |
| `TurnEnded { stop }` | turn completed / cancelled / failed by `stop` |
| `SessionUpdated` | model / mode changed |
| `ContextCompacted` | compaction done |
| session error (`AuthRequired`, `ProcessExited`) | session terminated with reason |

**Semantics:**
- `startSession`: `rt.open(agent, { dir: cwd, permission_mode, model?, resume? })` → `ProviderSession`. Keep `Map<ThreadId, Session>`.
- `sendTurn`: `session.prompt(text, attachments)`; model change first via `session.configure("model", ...)` when `modelSelection` differs; `sessionModelSwitch: "in-session"`.
- `respondToRequest`: `session.answer(id, { Permission: AllowOnce | AllowAlways | Deny })`. `respondToUserInput`: `session.answer(id, { Question: ... })`.
- `rollbackThread`: `session.rollback(n, "Conversation")`; `readThread` returns a snapshot built from the events the adapter has seen (T3 keeps its own transcript, so a minimal snapshot is fine; note if T3 tests need more).
- `compaction`: `{ type: "native", start: () => session.compact() }`.
- `streamEvents`: one `Stream` fed by a `Queue`; a fiber per open session pumps `session.events()` through `toProviderRuntimeEvents`.

- [ ] Write `AnyagentEvents.test.ts` first: one `it` per row above with a literal anyagent event and the expected T3 event(s). Run, see fail.
- [ ] Implement `AnyagentEvents.ts` until green.
- [ ] Write `AnyagentAdapter.test.ts` against the mock binary: start session → send turn → collect events until turn completed; interrupt; respond to a permission; stop; plus review-focus items 1–4 (pick or add a mock script under `packages/mock-scripts/` in the anyagent repo when none exists; keep it minimal and commit it there).
- [ ] Implement `AnyagentRuntime.ts`, `Errors.ts`, `AnyagentAdapter.ts` until green.
- [ ] `pnpm --filter @t3tools/server typecheck && pnpm --filter @t3tools/server test` green.
- [ ] Every anyagent shortfall met while mapping → row in `gaps.md`.
- [ ] Commit: `feat(server): AnyagentAdapter over anyagent-ts`.

### Task 2: `AnyagentDriver` for the six kinds, registered

**Files:**
- Create `apps/server/src/provider/anyagent/AnyagentDriver.ts` — `makeAnyagentDriver(kind: ProviderDriverKind, agent: string): ProviderDriver<AnyagentSettings, AnyagentDriverEnv>`. `create` builds the snapshot from `rt.discover()` + `rt.probe(agent)` (installed, version, login state, models, capabilities) and returns `makeAnyagentAdapter(kind, agent)`.
- Create `apps/server/src/provider/anyagent/AnyagentSnapshot.ts` — pure `toServerProviderSnapshot(kind, details: AgentDetails): ServerProviderShape` (models, auth status, capabilities). Copy the field set from `Layers/GrokProvider.ts` `buildInitialGrokProviderSnapshot`.
- Modify `apps/server/src/provider/builtInDrivers.ts`: replace the six built-in drivers with `makeAnyagentDriver("claude","claude")`, `("codex","codex")`, `("cursor","cursor")`, `("grok","grok")`, `("opencode","opencode")`, `("antigravity","antigravity")`. Keep every non-agent driver field T3's UI needs (display name, accent color, maintenance/update resolver: reuse the existing per-kind `UPDATE` resolvers by moving them into a small `anyagent/maintenance.ts` if they are the only thing left of the old driver files; otherwise import them).
- Test `AnyagentSnapshot.test.ts`, `AnyagentDriver.test.ts` (mock binary: driver `create` → instance → adapter start/turn).

**Interfaces consumed:** Task 1 names. **Produces:** `makeAnyagentDriver`.

- [ ] Snapshot tests first (literal `AgentDetails` → expected snapshot fields), then implement.
- [ ] Driver test: `create` then one turn over mock; review-focus item 5 (two kinds, one runtime, close one).
- [ ] Register in `builtInDrivers.ts`. Old drivers stay in the tree but unregistered (deletion is Task 4).
- [ ] On a fresh base dir only `codex` and `claudeAgent` are enabled by default (baseline.md). Keep that default; record in `baseline.md`-style one line in the report how a client enables the other four (config field or RPC), Task 3 needs it.
- [ ] `typecheck` + `test` green. Start the server headless (`baseline.md` command) with `ANYAGENT_BIN` set; the providers list shows the six kinds with real login state from `probe`. Save server log to scratchpad.
- [ ] Gaps rows for any snapshot field anyagent cannot supply (e.g. per-model capabilities T3 shows in its picker).
- [ ] Commit: `feat(server): route built-in providers through AnyagentDriver`.

### Task 3: Live feature script

**Files:** create `scripts/anyagent-port-check.ts` in the fork. Runs against a headless server started by the script (or a `--url`), agents from `--agents claude,codex`.

Rows, each printed `PASS`/`FAIL` + one-line reason, full event log to a scratchpad file:

| Row | Passes when |
|---|---|
| discover | provider list has the agent, `installed` and `loggedIn` true |
| open+stream | turn on "reply with the single word pong" yields text containing `pong` and a turn-completed event |
| tool+diff | "create a file named port-check.txt containing hello" in a temp dir: a tool event with a diff or file path, file exists after |
| permission | same prompt in Ask mode: approval request appears, script approves, turn completes, file exists |
| deny | approval denied: turn completes, file absent |
| question | "ask me one yes/no question and wait": user-input request appears, script answers, turn completes |
| model switch | send turn with a different model: a session-updated / model event reports it |
| cancel | long prompt, interrupt after first text delta: turn ends as cancelled, next turn still works |
| resume | stop session, start with resume token, "what file did you create?" mentions port-check.txt |
| rollback | rollback 1 turn, "what did I last ask?" does not mention the rolled-back prompt (skip with reason when `supportsConversationRollback` false) |
| usage | a token-usage event with non-zero input tokens arrived |

- [ ] Write the script with `--dry-run` listing rows (so the reviewer can run it without agents).
- [ ] Rows read `orchestration.subscribeThread` events (`thread.activity-appended`, `thread.approval-response-requested`, `thread.user-input-response-requested`, `thread.session-set`, ...), not raw provider events; the exact wire is in `baseline.md`.
- [ ] Run for `claude` and `codex` (both installed and logged in on this machine; check with `anyagent list`). Attach the log paths in the commit message body.
- [ ] Each FAIL: decide T3-side bug (fix in Task 1/2 files, rerun) or anyagent shortfall (gaps row with the row name, what T3 expected, what arrived).
- [ ] Commit: `chore: anyagent port feature check script`.

### Task 4: Delete the old agent layer

**Files (delete):** `Drivers/{Claude,Codex,Cursor,Grok,OpenCode,Antigravity}Driver.ts`, `Layers/*Adapter.ts`, `Layers/*Provider.ts`, `Layers/CodexSessionRuntime.ts`, `Layers/CodexCollab*.ts`, `Layers/codexLaunchArgs.ts`, `Layers/ProviderAdapterRegistry.ts` only if now unused, `acp/` (whole dir), `opencodeRuntime.ts`, `OpenCodeServerOwner.ts`, `packages/effect-acp`, `packages/effect-codex-app-server`, and their tests. Keep anything that is T3 product logic and still imported: skills discovery (`*Skills.ts`, `ClaudeSkillDispatch.ts`), `ClaudeModelCatalog`/`ModelManifest`, `Antigravity{Auth,Installation}.ts`, maintenance/update files, `ProviderService`, registries, auth service, session directory, reaper, event loggers.

- [ ] Delete in that order, running `typecheck` after each group; fix the imports in the ~18 outside files (`grep -rn "provider/" apps/server/src --include='*.ts' | grep -v "src/provider/"`). Where an outside file imported an old adapter's helper for product behavior (e.g. Claude home layout, skills), keep the helper by moving it next to its consumer rather than keeping the adapter.
- [ ] Remove the two packages from `pnpm-workspace.yaml` / root references, `pnpm install`, `typecheck` across the repo: `pnpm typecheck`.
- [ ] `pnpm --filter @t3tools/server test` green; `pnpm test` green or each remaining failure explained in the commit body.
- [ ] Rerun Task 3 script for claude + codex; all rows same result as before deletion.
- [ ] Commit: `refactor(server): remove per-agent adapters, anyagent is the only path`.

### Task 5: Numbers and gaps (anyagent repo)

**Files:** `docs/ports/t3-code/README.md`, `docs/ports/t3-code/gaps.md` (final pass), root `README.md` new section "Built on anyagent".

- [ ] `cd t3code && git diff --shortstat main..anyagent` and the Task 0 counting commands on both branches. Table: hand-written deleted, generated deleted, tests deleted, added.
- [ ] `README.md` in `docs/ports/t3-code/`: what was replaced (diagram above), the numbers table, how to reproduce (fork branch, commands, script), link to `gaps.md`.
- [ ] `gaps.md` rows normalized: `| feature | T3 call | what anyagent lacks | proposed anyagent change | size (lines) |`.
- [ ] Root README section: T3 Code row (numbers), laptop-agent row (`877 deleted / 1200 added`, commit `Replace the ACP harness layer with an anyagent-backed driver`), Zeron row (already built on anyagent).
- [ ] Commit in anyagent repo: `docs: T3 Code port numbers and gaps`.
