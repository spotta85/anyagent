# T3 Code baseline (before the port)

`$SCRATCH` = `/private/tmp/claude-501/-Users-spotta-Desktop-Projects-anyagent/25d99f8a-132e-4e0a-9dda-09d234fbca38/scratchpad`

Fork `/Users/spotta/Desktop/Projects/t3code`, branch `anyagent` = `main` = `679c34c096`. Node 26.7.0, pnpm 11.10.0 (T3 wants Node `^24.13.1`: warning only).

## Toolchain

The server package is named **`t3`**, not `@t3tools/server`. `pnpm --filter @t3tools/server ...` matches nothing and **exits 0 silently**.

| Step | Command (run from fork root) | Result | Wall |
|---|---|---|---|
| install | `pnpm install` | ok, `prepare` (`effect-tsgo patch && vp config`) ok | 59s |
| typecheck | `pnpm --filter t3 typecheck` | pass (`tsc --noEmit`); main = anyagent, one run covers both | 10s |
| test | `pnpm --filter t3 test` | **fail**: 49 failed / 5389 passed / 10 skipped (5448), 9 of 354 files | 397s |
| test, fixed TMPDIR | `cd apps/server && TMPDIR=/private/tmp/t3tmp npx vp test run` | 1 failed / 5437 passed / 10 skipped (5448) | 396s |

TMPDIR must be a **real path** (macOS `/var` → `/private/var` symlink breaks 48 tests) and **short** (the scratchpad path breaks 7 `cli/app.test.ts` tests: unix socket path limit). `/private/tmp/t3tmp` satisfies both.

### Test failures on clean main

| File | Failed, default TMPDIR | Failed, `/private/tmp/t3tmp` | Cause |
|---|---|---|---|
| `orchestration/ThreadSettlementReactor.test.ts` | 37 | 0 | tmpdir symlink |
| `provider/Drivers/CodexDriver.test.ts` | 3 | 0 | tmpdir symlink |
| `provider/providerMaintenance.test.ts` | 2 | 0 | tmpdir symlink |
| `provider/Layers/CursorProvider.test.ts` | 2 | 0 | tmpdir symlink |
| `provider/Layers/AntigravityAdapter.test.ts` | 1 | 0 | tmpdir symlink ("outside the session workspace") |
| `provider/AntigravityInstallation.test.ts` | 1 | 0 | tmpdir symlink |
| `project/AgentSessionScanner.test.ts` | 1 | 0 | tmpdir symlink |
| `entrypoint.test.ts` | 1 | 0 | tmpdir symlink |
| `device/sshDeviceScript.test.ts` | 1 | 1 | times out at 120s, also alone (needs local ssh) |

Rule for later tasks: always test with `TMPDIR=/private/tmp/t3tmp`; the only expected failure is `sshDeviceScript`.

```bash
mkdir -p /private/tmp/t3tmp
cd /Users/spotta/Desktop/Projects/t3code/apps/server
TMPDIR=/private/tmp/t3tmp npx vp test run                                                # whole suite, ~400s
TMPDIR=/private/tmp/t3tmp npx vp test run src/provider/Layers/AntigravityAdapter.test.ts  # one file, 2s
```

## Start the server headless

```bash
cd /Users/spotta/Desktop/Projects/t3code
node apps/server/src/bin.ts serve --base-dir $SCRATCH/t3home --port 3799 > $SCRATCH/t3-serve.log 2>&1 &
# ready when the log shows "Listening on http://127.0.0.1:3799" (~2s)
node apps/server/src/bin.ts auth session issue --base-dir $SCRATCH/t3home --token-only   # prints bearer token
```

| Item | Value |
|---|---|
| Entry | `apps/server/src/bin.ts` (runs as TS under Node 26); `serve` = no browser, prints pairing URL/QR |
| Default port | `3773` (`DEFAULT_PORT`, `apps/server/src/config.ts`); host `127.0.0.1` |
| Env equivalents | `T3CODE_PORT`, `T3CODE_HOST`, `T3CODE_HOME` (= `--base-dir`), `T3CODE_NO_BROWSER`, `T3CODE_MODE` |
| Desktop app needed | no |
| Auth | required. Bearer token from `t3 auth session issue` (same `--base-dir`) |
| WS endpoint | `ws://127.0.0.1:<port>/ws` with header `Authorization: Bearer <token>` (Node `new WebSocket(url, { headers })` works), or `?wsTicket=<ticket>` from `POST /api/auth/websocket-ticket` |
| Wire | Effect RPC, JSON. Send `{"_tag":"Request","id":"1","tag":"<method>","payload":{...},"headers":[]}`. Get `{"_tag":"Exit","requestId","exit":{"_tag":"Success","value"}}`; streams get `{"_tag":"Chunk","requestId","values":[...]}` and must reply `{"_tag":"Ack","requestId"}`; answer `Ping` with `Pong` |
| Providers on this Mac (fresh base dir) | `codex`, `claudeAgent` ready; `cursor`, `grok`, `opencode`, `antigravity` disabled by default |

Probe that exercises all of this: `$SCRATCH/t3probe/probe.mjs <port> <token>` (log `$SCRATCH/t3-probe.log`).

## RPC surface a client uses

Contracts: `packages/contracts/src/rpc.ts` (`WS_METHODS`), `packages/contracts/src/orchestration.ts` (`ORCHESTRATION_WS_METHODS`, command structs). Every thread action is one method, `orchestration.dispatchCommand`, with a `type` tag. Common fields on every command: `commandId` (uuid), `threadId`, `createdAt` (ISO).

| Action | WS method / command `type` | Extra payload fields | Verified |
|---|---|---|---|
| list providers | `server.getConfig` `{}` → `.providers[]` (`instanceId, driver, enabled, status, models, ...`) | none | yes |
| refresh providers | `server.refreshProviders` | `instanceId?` | no |
| create project | `project.create` | `projectId, title, workspaceRoot` | yes |
| create thread | `thread.create` | `projectId, title, modelSelection:{instanceId, model, options?}, runtimeMode, interactionMode, branch:null, worktreePath:null` | yes |
| send turn | `thread.turn.start` | `message:{messageId, role:"user", text, attachments:[]}, runtimeMode, interactionMode, modelSelection?` | no (live) |
| answer approval | `thread.approval.respond` | `requestId, decision` | no (live) |
| answer user input | `thread.user-input.respond` | `requestId, answers:{[questionId]:unknown}` | no (live) |
| interrupt | `thread.turn.interrupt` | `turnId?` | no (live) |
| rollback | `thread.conversation.revert` / `thread.checkpoint.revert` | `turnCount` | no |
| stop session | `thread.session.stop` | none | no |
| read thread | `GET /api/orchestration/threads/:threadId` (bearer) or first chunk of `orchestration.subscribeThread` (`kind:"snapshot"`) | none | yes |
| subscribe events | `orchestration.subscribeThread` (stream) | `threadId, afterSequence?` | yes |

Enums: `runtimeMode` = `approval-required | auto-accept-edits | auto | full-access`. `interactionMode` = `default | plan`. `decision` = `accept | acceptForSession | acceptAlways | decline | cancel`. `dispatchCommand` returns `{sequence}`.

`subscribeThread` stream items: `{kind:"snapshot"}`, `{kind:"synchronized"}`, `{kind:"event", event}`. Events are orchestration events, not raw `ProviderRuntimeEventV2`: `thread.message-sent`, `thread.activity-appended`, `thread.session-set`, `thread.turn-diff-completed`, `thread.turn-start-requested`, `thread.approval-response-requested`, `thread.user-input-response-requested`, `thread.reverted`, ... No RPC exposes raw provider events.

HTTP alternative (bearer): `POST /api/orchestration/dispatch` (same commands, not run), `GET /api/orchestration/snapshot` (not run).

## Line counts (before the port)

Brief's commands, run verbatim from the fork root.

| Bucket | Files | Lines |
|---|---|---|
| `provider/Drivers` | 15 | 3,745 |
| `provider/Layers` | 32 | 31,041 |
| `provider/Services` | 15 | 902 |
| `provider/acp` | 15 | 4,913 |
| `provider/` top level | 32 | 8,292 |
| **server provider, hand-written** | 109 | **48,893** |
| `effect-acp` + `effect-codex-app-server` src, hand-written | 16 | 3,869 |
| `effect-codex-app-server/src/_generated` | 3 | 58,004 |
| `effect-acp/src/_generated` (not in brief; extra) | – | 10,410 |
| `provider/**/*.test.ts` | 80 | 61,264 |

## Logs

| Log | Content |
|---|---|
| `t3-install.log` | pnpm install |
| `t3-typecheck.log` | typecheck |
| `t3-test.log` | full test run |
| `t3-test-rerun-tmpdir.log` | 8 symlink-failing files, real TMPDIR, all pass |
| `t3-test-shorttmp-full.log` | full suite, `TMPDIR=/private/tmp/t3tmp` |
| `t3-test-rerun-ssh.log` | ssh test alone, still times out |
| `t3-serve.log`, `t3-probe.log` | headless server + RPC probe |
