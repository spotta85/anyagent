// A stdio MCP server for the live suite: newline-delimited JSON-RPC, one
// tool, `secret_word`, that returns MARKER. Each call is appended to the
// file named by the first argument, so a test can see it arrived.
import { appendFileSync } from 'node:fs';
import { createInterface } from 'node:readline';

const MARKER = 'PLUM-4417';
const calls = process.argv[2];

// The reply to one request.
function reply(msg) {
  const result = (value) => ({ jsonrpc: '2.0', id: msg.id, result: value });
  switch (msg.method) {
    case 'initialize':
      return result({ protocolVersion: msg.params.protocolVersion, capabilities: { tools: {} }, serverInfo: { name: 'probe', version: '1.0.0' } });
    case 'tools/list':
      return result({ tools: [{ name: 'secret_word', description: 'Returns the secret word.', inputSchema: { type: 'object', properties: {} } }] });
    case 'tools/call':
      appendFileSync(calls, `${msg.params.name}\n`);
      return result({ content: [{ type: 'text', text: MARKER }] });
    case 'ping':
      return result({});
    default:
      return { jsonrpc: '2.0', id: msg.id, error: { code: -32601, message: `no method ${msg.method}` } };
  }
}

// Notifications (no id) need no reply.
createInterface({ input: process.stdin }).on('line', (line) => {
  const msg = line.trim() && JSON.parse(line);
  if (msg && msg.id !== undefined) process.stdout.write(JSON.stringify(reply(msg)) + '\n');
});
