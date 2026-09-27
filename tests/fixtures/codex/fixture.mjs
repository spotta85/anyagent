// Codex app-server fixture agent, shaped like the recordings in this
// directory (codex 0.147.0): line-delimited JSON-RPC 2.0 both ways.
// Flags: --logged-out (no account; a turn 401s), --api-key (auth.json key
// login), --question (a requestUserInput mid-turn), --echo-config-home
// (echo the CODEX_HOME the child received). Prompt words steer scenarios:
// "write-file" (a fileChange escalates past the sandbox -> approval),
// "mcp-tool"/"mcp-two"/"mcp-always" (MCP tool calls ask through an
// elicitation), "sleep" (a command that only an interrupt ends), "die"
// (exit mid-turn), "subagent" (a child thread runs a whole turn before the
// parent's ends, "subagent-fails" for a child turn that fails), "spawn-live"
// (a subagent in the live 0.154.0 order of recording 13),
// "end-failed"/"end-aborted" (the turn ends via turn/failed / turn/aborted
// instead of turn/completed), "refuse-start" (turn/start is refused).
// --rename: the server renames the thread after the first turn.
// --host-feature: the host config enables an under-development feature too.
// "hook-blocked": the user's prompt hook completes as `blocked`.
// A turn/start in the `plan` collaboration mode also yields a `plan` item
// ("no-plan": one with empty text).
import { createInterface } from 'node:readline';
import { appendFileSync } from 'node:fs';

// FIXTURE_ARGV_LOG, set through the session's env: log the launch args there.
if (process.env.FIXTURE_ARGV_LOG) appendFileSync(process.env.FIXTURE_ARGV_LOG, JSON.stringify(process.argv.slice(2)) + '\n');
// FIXTURE_MCP_LOG: log each env var an `mcp_servers.…` override names, with the value received.
if (process.env.FIXTURE_MCP_LOG) {
  const named = process.argv.filter((a) => a.startsWith('mcp_servers.')).flatMap((a) => [...a.matchAll(/"(\w+)"/g)].map((m) => m[1]));
  appendFileSync(process.env.FIXTURE_MCP_LOG, JSON.stringify(Object.fromEntries(named.filter((n) => n in process.env).map((n) => [n, process.env[n]]))) + '\n');
}
const flag = (name) => process.argv.includes(name);
const send = (m) => process.stdout.write(JSON.stringify({ jsonrpc: '2.0', ...m }) + '\n');
const notify = (method, params) => send({ method, params });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const THREAD = { id: 'th-1', name: null };
let turnN = 0, serverReqN = 0, itemN = 0, rolled = 0, experimental = false;
// `-c mcp_servers.<name>.<key>=…` launch overrides, as the real CLI takes them.
const MCP_NAMES = [...new Set(process.argv
  .flatMap((a, i) => (a === '-c' ? [process.argv[i + 1] ?? ''] : []))
  .map((kv) => kv.match(/^mcp_servers\.([^.]+)\./)?.[1])
  .filter(Boolean))];
// `-c features.<name>=true` overrides plus --host-feature; the server warns about them.
const FEATURES = [...process.argv
  .flatMap((a, i) => (a === '-c' ? [process.argv[i + 1] ?? ''] : []))
  .map((kv) => kv.match(/^features\.([^=]+)=true$/)?.[1])
  .filter(Boolean), ...(flag('--host-feature') ? ['current_time_reminder'] : [])].sort();
let turn = null; // { id, started, interrupted, steered: [] }
const turnIds = []; // completed turns, oldest first
let lastModel = null; // the model the thread last ran a turn with
const waiters = {}; // server request id -> resolver

// A user hook's run (0.154.0 app-server schema, params trimmed).
function hookRun(status, entries = []) {
  const run = { id: 'hook-1', eventName: 'userPromptSubmit', executionMode: 'sync', handlerType: 'command', scope: 'turn', status, entries, statusMessage: null };
  notify(status === 'running' ? 'hook/started' : 'hook/completed', { threadId: THREAD.id, turnId: turn.id, run });
}

// Recorded (05-resume-and-fork): right after a resume or fork reply, the
// restored thread's last model call, while no turn runs.
function restoredUsage(last) {
  notify('thread/tokenUsage/updated', { threadId: THREAD.id, turnId: 'turn-prev', tokenUsage: { total: last, last, modelContextWindow: 258400 } });
}

// Recorded 2026-09-26 (0.154.0): after thread/start, and again on a revert.
function featureWarning() {
  if (!FEATURES.length) return;
  notify('warning', { threadId: THREAD.id, message: `Under-development features enabled: ${FEATURES.join(', ')}. Under-development features are incomplete and may behave unpredictably. To suppress this warning, set \`suppress_unstable_features_warning = true\` in /Users/user/.codex/config.toml.` });
}

const MODELS = [
  { id: 'gpt-6', model: 'gpt-6', displayName: 'GPT-6', description: 'Frontier model.', serviceTiers: [{ id: 'priority', name: 'Fast', description: '1.5x speed' }], defaultServiceTier: 'priority', hidden: false, isDefault: true, defaultReasoningEffort: 'medium', supportedReasoningEfforts: [{ reasoningEffort: 'low', description: 'Fast' }, { reasoningEffort: 'medium', description: 'Balanced' }, { reasoningEffort: 'high', description: 'Deep' }] },
  { id: 'gpt-6-mini', model: 'gpt-6-mini', displayName: 'GPT-6 Mini', description: 'Small model.', additionalSpeedTiers: flag('--no-mini-fast') ? [] : ['fast'], hidden: false, isDefault: false, defaultReasoningEffort: 'low', supportedReasoningEfforts: [{ reasoningEffort: 'low', description: 'Fast' }, { reasoningEffort: 'medium', description: 'Balanced' }] },
  { id: 'gpt-secret', model: 'gpt-secret', displayName: 'Secret', description: null, hidden: true, isDefault: false, defaultReasoningEffort: 'low', supportedReasoningEfforts: [] },
];
const RATE_LIMITS = {
  limitId: 'codex', planType: 'edu',
  primary: { usedPercent: 5, windowDurationMins: 300, resetsAt: 1787903985 },
  secondary: { usedPercent: 4, windowDurationMins: 10080, resetsAt: 1788329085 },
};

const item = (fields) => ({ id: `it-${itemN++}`, ...fields });
const itemStarted = (it) => notify('item/started', { item: it, threadId: THREAD.id, turnId: turn.id });
const itemCompleted = (it) => notify('item/completed', { item: it, threadId: THREAD.id, turnId: turn.id });
const delta = (itemId, d) => notify('item/agentMessage/delta', { threadId: THREAD.id, turnId: turn.id, itemId, delta: d });

// Awaits the client's response to one server->client request.
function ask(method, params) {
  const id = serverReqN++;
  return new Promise((r) => { waiters[id] = r; send({ method, id, params: { threadId: THREAD.id, turnId: turn.id, ...params } }); });
}

const rl = createInterface({ input: process.stdin });
rl.on('line', (line) => {
  const m = JSON.parse(line);
  if (m.method === undefined && m.id !== undefined) {
    const r = waiters[m.id];
    delete waiters[m.id];
    return r?.(m.result ?? m.error);
  }
  if (m.id !== undefined) onRequest(m).catch(() => process.exit(1));
});
rl.on('close', () => process.exit(0));

// The thread bind response, echoing creation params like the real server.
function threadResult(params) {
  const sandboxType = { 'read-only': 'readOnly', 'workspace-write': 'workspaceWrite', 'danger-full-access': 'dangerFullAccess' }[params.sandbox] ?? 'readOnly';
  return {
    thread: THREAD,
    model: 'gpt-6', // the config-file default; per-turn model rides turn/start
    // --xhigh-effort: a config-file effort the default model does not list.
    reasoningEffort: flag('--xhigh-effort') ? 'xhigh' : null,
    serviceTier: flag('--default-fast') ? 'priority' : null,
    approvalPolicy: params.approvalPolicy ?? 'on-request',
    sandbox: { type: sandboxType, networkAccess: false },
  };
}

async function onRequest(m) {
  const reply = (result) => send({ id: m.id, result });
  const refuse = (message) => send({ id: m.id, error: { code: -32600, message } });
  switch (m.method) {
    case 'initialize':
      // requestUserInput only fires for clients that opt into the
      // experimental API, like the real server.
      experimental = m.params?.capabilities?.experimentalApi === true;
      return reply({ userAgent: 'anyagent/0.147.0 (Mac OS 26.5.1; arm64)', codexHome: process.env.CODEX_HOME ?? '', platformOs: 'macos' });
    case 'account/read':
      return reply(flag('--logged-out')
        ? { account: null, requiresOpenaiAuth: true }
        : flag('--api-key')
        ? { account: { type: 'apiKey' }, requiresOpenaiAuth: true }
        : { account: { type: 'chatgpt', email: 'user@example.com', planType: 'edu' }, requiresOpenaiAuth: true });
    case 'model/list':
      return reply({ data: MODELS, nextCursor: null });
    case 'skills/list':
      // Grouped by root; the same skill appears under every root (dedupe by
      // name), a nameless entry is junk, and only `review` has an interface.
      // A disabled skill still comes back, `enabled: false` (probed 2026-09-27, 0.154.0).
      return reply({ data: [
        { cwd: process.cwd(), skills: [
          { name: 'review', description: 'A long model-facing paragraph.', interface: { shortDescription: 'Review a diff.' }, enabled: true, scope: 'repo', path: '/repo/.codex/skills/review/SKILL.md' },
          { name: 'off', description: 'Disabled.', enabled: false, scope: 'user', path: '/home/skills/off/SKILL.md' },
          { name: 'release', description: 'Cut a release.', enabled: true, scope: 'user', path: '/home/skills/release/SKILL.md' },
          { name: '', description: 'no name', enabled: true, scope: 'user', path: '/home/skills/junk/SKILL.md' },
        ] },
        { cwd: '/other', skills: [{ name: 'review', description: 'dup', enabled: true, scope: 'user', path: '/home/skills/review/SKILL.md' }] },
      ] });
    case 'account/rateLimits/read':
      if (flag('--logged-out')) return refuse('codex account authentication required to read rate limits');
      return reply({ rateLimits: RATE_LIMITS });
    case 'thread/start':
      reply(threadResult(m.params));
      return featureWarning();
    case 'thread/resume':
      // Recorded 2026-09-26 (0.154.0): a well-formed id with no thread, and a malformed one.
      if (m.params.threadId === 'th-gone') return refuse('no rollout found for thread id th-gone');
      if (m.params.threadId === 'not-a-uuid') return refuse('invalid session id: invalid character: expected an optional prefix of `urn:uuid:` followed by [0-9a-fA-F-], found `n` at 1');
      THREAD.id = m.params.threadId;
      turnIds.push('turn-prev'); // the thread's history rides the bind
      reply({ ...threadResult(m.params), thread: { ...THREAD, turns: [{ id: 'turn-prev' }] } });
      return restoredUsage({ totalTokens: 10259, inputTokens: 0, cachedInputTokens: 0, outputTokens: 0 });
    case 'thread/fork':
      THREAD.id = 'th-fork-1';
      THREAD.forkPoint = m.params.lastTurnId ?? null;
      reply({ ...threadResult(m.params), thread: { ...THREAD, forkedFromId: m.params.threadId } });
      return restoredUsage({ totalTokens: 14382, inputTokens: 14377, cachedInputTokens: 11008, outputTokens: 5 });
    case 'turn/start': {
      if (turn) return refuse('phantom: turn/start while a turn is running'); // adapters must steer instead
      if (m.params.input[0].text.includes('refuse-start')) return refuse('turn refused');
      turn = { id: `turn-${turnN++}`, started: false, interrupted: false };
      lastModel = m.params.model ?? 'gpt-6';
      reply({ turn: { id: turn.id, status: 'inProgress' } });
      runTurn(m.params).catch(() => process.exit(1));
      return;
    }
    // Recorded 2026-08-27 (11-config-plan-compact): an empty result, then a
    // turn of its own carrying one `contextCompaction` item.
    case 'thread/compact/start': {
      if (flag('--compact-refuses')) return refuse('nothing to compact');
      reply({});
      const id = `turn-${turnN++}`;
      send({ method: 'turn/started', params: { threadId: THREAD.id, turn: { id, status: 'inProgress' } } });
      send({ method: 'item/started', params: { threadId: THREAD.id, turnId: id, item: { type: 'contextCompaction', id: 'cc-1' } } });
      send({ method: 'thread/tokenUsage/updated', params: { threadId: THREAD.id, turnId: id, tokenUsage: { total: { totalTokens: 3765 }, last: { totalTokens: 3765, inputTokens: 0, cachedInputTokens: 0, outputTokens: 0 }, modelContextWindow: 258400 } } });
      send({ method: 'item/completed', params: { threadId: THREAD.id, turnId: id, item: { type: 'contextCompaction', id: 'cc-1' } } });
      send({ method: 'turn/completed', params: { threadId: THREAD.id, turn: { id, status: 'completed', error: null } } });
      return;
    }
    case 'turn/steer': {
      // Like the real server: a steer before turn/started is refused.
      if (!turn || m.params.expectedTurnId !== turn.id) return refuse(`expected active turn id \`${m.params.expectedTurnId}\` but found none`);
      if (!turn.started) return refuse('no active turn to steer');
      turn.steered.push(m.params.input[0].text);
      return reply({ turnId: turn.id });
    }
    case 'turn/interrupt': {
      if (!turn) return refuse('no active turn to interrupt');
      turn.interrupted = true;
      reply({});
      return;
    }
    case 'thread/revert': {
      if (turn) return refuse('cannot revert while a turn is running');
      const at = turnIds.indexOf(m.params.beforeTurnId);
      if (at < 0) return refuse(`unknown turn \`${m.params.beforeTurnId}\``);
      rolled += turnIds.splice(at).length;
      // The reloaded thread warns before the reply, naming the last turn's model (0.154.0).
      featureWarning();
      if (lastModel && lastModel !== 'gpt-6') notify('warning', { threadId: THREAD.id, message: `This session was recorded with model \`${lastModel}\` but is resuming with \`gpt-6\`. Consider switching back to \`${lastModel}\` as it may affect Codex performance.` });
      notify('thread/reverted', { threadId: THREAD.id });
      return reply({ thread: THREAD, turnsBackwardsCursor: null, itemsBackwardsCursor: null });
    }
    default:
      return refuse(`Invalid request: unknown variant \`${m.method}\``);
  }
}

// One turn: userMessage echo, then the scenario the prompt asks for. The
// in-flight tool item gets no item/completed when interrupted (recording 04).
async function runTurn(params) {
  const prompt = params.input[0].text;
  turn.steered = [];
  await sleep(30); // the real started-window: steers before this are refused
  if (turn.interrupted) return endTurn('interrupted');
  notify('turn/started', { threadId: THREAD.id, turn: { id: turn.id, status: 'inProgress' } });
  turn.started = true;
  notify('thread/settings/updated', { threadId: THREAD.id, threadSettings: { model: params.model ?? 'gpt-6', cwd: process.cwd() } });
  hookRun('running');
  if (prompt.includes('hook-blocked')) hookRun('blocked', [{ kind: 'stop', text: 'no secrets in prompts' }]);
  else hookRun('completed');
  const user = item({ type: 'userMessage', clientId: params.clientUserMessageId, content: [{ type: 'text', text: prompt }] });
  itemStarted(user);
  itemCompleted(user);

  if (flag('--logged-out')) {
    // The 401 retries, then the turn still ends deterministically (recording 08).
    const error = { message: 'Reconnecting... 2/5', codexErrorInfo: { responseStreamDisconnected: { httpStatusCode: 401 } } };
    notify('error', { error, willRetry: true, threadId: THREAD.id, turnId: turn.id });
    notify('error', { error, willRetry: true, threadId: THREAD.id, turnId: turn.id });
    return endTurn('failed', { message: 'unexpected status 401 Unauthorized' });
  }
  if (prompt.includes('die')) { process.stderr.write('boom: fixture died\n'); process.exit(3); }
  // Some wires end a turn with these instead of turn/completed.
  if (prompt.includes('end-failed')) return endTurn('failed', { message: 'wire failed' }, 'turn/failed');
  if (prompt.includes('end-aborted')) return endTurn('aborted', null, 'turn/aborted');

  if (prompt.includes('sleep')) {
    const exec = item({ type: 'commandExecution', command: '/bin/zsh -lc "sleep 45"', cwd: process.cwd(), status: 'inProgress', aggregatedOutput: null, exitCode: null });
    itemStarted(exec);
    while (!turn.interrupted) await sleep(10);
    return endTurn('interrupted');
  }

  const reasoning = item({ type: 'reasoning', summary: [], content: [] });
  itemStarted(reasoning);
  notify('item/reasoning/summaryPartAdded', { threadId: THREAD.id, turnId: turn.id, itemId: reasoning.id, summaryIndex: 0 });
  notify('item/reasoning/summaryTextDelta', { threadId: THREAD.id, turnId: turn.id, itemId: reasoning.id, delta: 'thinking…' });
  itemCompleted(reasoning);

  const msg = item({ type: 'agentMessage', text: '', phase: 'final_answer' });
  itemStarted(msg);
  delta(msg.id, 'Hello ');
  delta(msg.id, `model=${params.model ?? 'unset'} effort=${params.effort ?? 'unset'} tier=${params.serviceTier ?? 'unset'} summary=${params.summary ?? 'unset'} `);
  if (flag('--echo-config-home')) delta(msg.id, `cfg=${process.env.CODEX_HOME ?? 'unset'} `);
  if (THREAD.forkPoint !== undefined) delta(msg.id, `fork=${THREAD.forkPoint} `);
  if (prompt.includes('Attached files:')) delta(msg.id, 'ref=1 ');
  // Per-turn policy, sandbox, images, launch MCP servers, and rollbacks so
  // far, each visible to the tests.
  const images = (params.input ?? []).filter((i) => i.type === 'localImage').length;
  delta(msg.id, `policy=${params.approvalPolicy ?? 'unset'} sandbox=${params.sandboxPolicy?.type ?? 'unset'} images=${images} mcp=${MCP_NAMES.join(',') || 'none'} rolled=${rolled} collab=${JSON.stringify(params.collaborationMode ?? null)} `);
  // Plan mode (probed 2026-09-27, 0.154.0): the proposal is a `plan` item;
  // its deltas repeat what the completed item carries.
  if (params.collaborationMode?.mode === 'plan') {
    const plan = item({ type: 'plan', text: '' });
    itemStarted(plan);
    notify('item/plan/delta', { threadId: THREAD.id, turnId: turn.id, itemId: plan.id, delta: '# Plan' });
    itemCompleted({ ...plan, text: prompt.includes('no-plan') ? '' : '# Plan\n\n1. Add README.md' });
  }

  if (flag('--question')) {
    if (!experimental) {
      delta(msg.id, 'answer=noapi ');
    } else {
      const resp = await ask('item/tool/requestUserInput', { itemId: 'it-q', questions: [{ id: 'q1', header: 'Color', question: 'Which color?', options: [{ label: 'Red', description: 'Prefer red' }, { label: 'Blue', description: 'Prefer blue' }], isOther: false }] });
      if (turn.interrupted) return endTurn('interrupted');
      delta(msg.id, `answer=${resp?.answers?.q1?.answers?.[0] ?? 'none'} `);
    }
  }

  const exec = item({ type: 'commandExecution', command: '/bin/zsh -lc "echo PEAR"', cwd: process.cwd(), status: 'inProgress', aggregatedOutput: null, exitCode: null });
  itemStarted(exec);
  itemCompleted({ ...exec, status: 'completed', aggregatedOutput: 'PEAR\n', exitCode: 0 });

  if (prompt.includes('write-file')) {
    // The write escalates past the sandbox (recording 02): the approval
    // request names only the item; decline leaves it `declined`.
    const change = item({ type: 'fileChange', changes: [{ path: 'fruit.txt', kind: { type: 'add' }, diff: 'PEAR\n' }], status: 'inProgress' });
    itemStarted(change);
    const resp = await ask('item/fileChange/requestApproval', { itemId: change.id, reason: null, grantRoot: null });
    if (turn.interrupted) return endTurn('interrupted');
    notify('serverRequest/resolved', { threadId: THREAD.id, requestId: serverReqN - 1 });
    const accepted = resp?.decision === 'accept' || resp?.decision === 'acceptForSession';
    itemCompleted({ ...change, status: accepted ? 'completed' : 'declined' });
    delta(msg.id, `write=${resp?.decision} `);
    // `cancel` also interrupts the turn (generated schema, 0.154.0).
    if (resp?.decision === 'cancel') return endTurn('interrupted');
  }

  if (prompt.includes('mcp-')) {
    // Recorded 2026-09-27 (codex 0.154.0): names the server, not the item; decline fails it.
    // "mcp-two" runs two calls of one tool; each is asked about while both are in flight.
    // "mcp-always" offers only the persistent remember form, as a string.
    const args = prompt.includes('mcp-two') ? [{ word: 'a' }, { word: 'b' }] : [{}];
    const persist = prompt.includes('mcp-always') ? 'always' : ['session', 'always'];
    const calls = args.map((a) => item({ type: 'mcpToolCall', server: 'probe', tool: 'secret_word', status: 'inProgress', arguments: a, result: null, error: null }));
    calls.forEach(itemStarted);
    const resps = [];
    for (const call of calls) {
      const resp = await ask('mcpServer/elicitation/request', { serverName: 'probe', mode: 'form', _meta: { codex_approval_kind: 'mcp_tool_call', persist, tool_description: 'Returns the secret word.', tool_params: call.arguments, tool_params_display: [] }, message: 'Allow the probe MCP server to run tool "secret_word"?', requestedSchema: { type: 'object', properties: {} } });
      // A cancel reply comes just before the interrupt: record the action it carried.
      while (resp?.action === 'cancel' && !turn.interrupted) await sleep(10);
      if (turn.interrupted) { delta(msg.id, `mcpcall=${resp?.action} `); return endTurn('interrupted'); }
      resps.push(resp);
      notify('serverRequest/resolved', { threadId: THREAD.id, requestId: serverReqN - 1 });
    }
    calls.forEach((call, i) => {
      const accepted = resps[i]?.action === 'accept';
      itemCompleted({ ...call, status: accepted ? 'completed' : 'failed', error: accepted ? null : { message: 'user rejected MCP tool call' } });
      delta(msg.id, `mcpcall=${[resps[i]?.action, resps[i]?._meta?.persist].filter(Boolean).join('/')} `);
    });
  }

  if (prompt.includes('subagent')) await runSubagent(prompt.includes('subagent-fails'));
  if (prompt.includes('spawn-live')) await runLiveSubagent();

  await sleep(20); // yield so a mid-turn steer on stdin gets read, like the real server
  for (const steer of turn.steered) delta(msg.id, `steered=${steer} `);
  notify('turn/plan/updated', { threadId: THREAD.id, turnId: turn.id, plan: [{ step: 'step 1', status: 'inProgress' }] });
  delta(msg.id, 'done');
  itemCompleted({ ...msg, text: 'done' });
  notify('thread/tokenUsage/updated', { threadId: THREAD.id, turnId: turn.id, tokenUsage: { total: { totalTokens: 2400 }, last: { totalTokens: 1200, inputTokens: 1100, cachedInputTokens: 600, outputTokens: 100 }, modelContextWindow: 258400 } });
  notify('account/rateLimits/updated', { rateLimits: RATE_LIMITS });
  if (turn.interrupted) return endTurn('interrupted');
  endTurn('completed');
  if (flag('--rename') && THREAD.name === null) {
    THREAD.name = 'Pear talk';
    notify('thread/name/updated', { threadId: THREAD.id, name: THREAD.name });
  }
}

function endTurn(status, error = null, method = 'turn/completed') {
  notify(method, { threadId: THREAD.id, turn: { id: turn.id, status, error, items: [] } });
  turnIds.push(turn.id);
  turn = null;
}

// A spawned subagent: the parent gets a collab tool call plus the child-thread
// item, and the child then runs a whole turn — started, content, usage, and its
// own turn/completed — on its own threadId, all BEFORE the parent's turn ends.
async function runSubagent(fails) {
  const CHILD = 'th-child-1', CHILD_TURN = 'turn-child-1';
  const child = (method, params) => notify(method, { threadId: CHILD, turnId: CHILD_TURN, ...params });
  const collab = item({ type: 'collabAgentToolCall', tool: 'spawnAgent', senderThreadId: THREAD.id, receiverThreadIds: [CHILD], agentsStates: {}, status: 'inProgress', prompt: 'review the diff' });
  itemStarted(collab);
  const activity = item({ type: 'subAgentActivity', agentThreadId: CHILD, agentPath: '.codex/agents/reviewer.md', kind: 'started' });
  itemStarted(activity);

  child('turn/started', { turn: { id: CHILD_TURN, status: 'inProgress' } });
  const said = { id: 'it-child-msg', type: 'agentMessage', text: '', phase: 'final_answer' };
  child('item/started', { item: said, startedAtMs: 0 });
  child('item/agentMessage/delta', { itemId: said.id, delta: 'child text' });
  child('item/completed', { item: { ...said, text: 'child text' } });
  // Would overwrite the parent's context gauge and plan if it were not consumed.
  child('thread/tokenUsage/updated', { tokenUsage: { total: { totalTokens: 77 }, last: { totalTokens: 77 }, modelContextWindow: 1024 } });
  child('turn/plan/updated', { plan: [{ step: 'child step', status: 'inProgress' }] });
  notify('turn/completed', { threadId: CHILD, turn: { id: CHILD_TURN, status: fails ? 'failed' : 'completed', error: fails ? { message: 'child blew up' } : null, items: [] } });

  itemCompleted({ ...collab, status: 'completed', agentsStates: { [CHILD]: { status: fails ? 'errored' : 'completed' } } });
  await sleep(10);
}

// A subagent in the live 0.154.0 order (recording 13): a spawn activity item, a `wait`
// collab call, and the child's finish as a second activity item under a new id.
async function runLiveSubagent() {
  const CHILD = 'th-child-2', CHILD_TURN = 'turn-child-2';
  const child = (method, params) => notify(method, { threadId: CHILD, turnId: CHILD_TURN, ...params });
  const activity = (id, kind) => ({ type: 'subAgentActivity', id, kind, agentThreadId: CHILD, agentPath: '/root/pong' });
  notify('thread/status/changed', { threadId: CHILD, status: { type: 'idle' } });
  itemStarted(activity('call_spawn', 'started'));
  itemCompleted(activity('call_spawn', 'started'));
  child('turn/started', { turn: { id: CHILD_TURN, status: 'inProgress' } });
  const wait = { type: 'collabAgentToolCall', id: 'call_wait', tool: 'wait', status: 'inProgress', senderThreadId: THREAD.id, receiverThreadIds: [], prompt: null, model: null, reasoningEffort: null, agentsStates: {} };
  itemStarted(wait);
  const said = { id: 'it-child-msg-2', type: 'agentMessage', text: '', phase: 'final_answer' };
  child('item/started', { item: said });
  child('item/agentMessage/delta', { itemId: said.id, delta: 'PONG' });
  child('item/completed', { item: { ...said, text: 'PONG' } });
  const used = { totalTokens: 22059, inputTokens: 22053, cachedInputTokens: 15872, cacheWriteInputTokens: 0, outputTokens: 6, reasoningOutputTokens: 0 };
  child('thread/tokenUsage/updated', { tokenUsage: { total: used, last: used, modelContextWindow: 258400 } });
  const finish = activity(`subagent-completed-${CHILD_TURN}`, 'completed');
  itemStarted(finish);
  notify('thread/status/changed', { threadId: CHILD, status: { type: 'idle' } });
  notify('turn/completed', { threadId: CHILD, turn: { id: CHILD_TURN, status: 'completed', error: null, items: [] } });
  itemCompleted(finish);
  itemCompleted({ ...wait, status: 'completed' });
  await sleep(10);
}
