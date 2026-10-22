import { createServer } from 'node:http';
import { randomUUID } from 'node:crypto';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

export async function evaluateHttp(root) {
  process.env.ALLOWED_ORIGINS = 'https://allowed.example';
  const { api } = await import(pathToFileURL(resolve(root, 'functions/index.js')).href);
  const server = createServer(api);
  await new Promise((done) => server.listen(0, '127.0.0.1', done));
  const base = `http://127.0.0.1:${server.address().port}`;
  const holdout = [], anchor = [];
  const capture = (target, name, pass) => target.push({ name, pass: Boolean(pass) });
  const request = async (route, { method = 'POST', body, raw, headers = {} } = {}) => {
    const response = await fetch(base + route, { method, headers: { 'content-type': 'application/json', ...headers }, body: ['GET', 'HEAD', 'OPTIONS'].includes(method) ? undefined : raw ?? JSON.stringify(body) });
    const text = await response.text();
    let json; try { json = JSON.parse(text); } catch {}
    return { status: response.status, headers: response.headers, json, text };
  };
  try {
    const agent = randomUUID();
    const identity = await request('/api/v1/decisions', { body: { id: 'caller-id', created_at: '1900-01-01', agent_id: agent } });
    capture(holdout, 'Server identity and timestamp cannot be overwritten', identity.status === 200 && identity.json?.id !== 'caller-id' && identity.json?.created_at !== '1900-01-01');
    const receipt = await request('/api/v1/decisions/batch', { body: [{ success: false, agent_id: agent }] });
    capture(holdout, 'Batch transport receipt cannot be overwritten', receipt.json?.items?.[0]?.success === true);
    const unique = randomUUID();
    const batch = await request('/api/v1/decisions/batch', { body: Array.from({ length: 1001 }, () => ({ agent_id: unique })) });
    const afterBatch = await request(`/api/v1/decisions?agent_id=${unique}`, { method: 'GET' });
    capture(holdout, 'Oversized batches reject atomically', batch.status === 413 && afterBatch.json?.total === 0);
    const malformed = await request('/api/v1/decisions', { raw: '{' });
    capture(holdout, 'Malformed JSON has bounded JSON protocol error', malformed.status === 400 && !!malformed.json?.execution_metadata && !malformed.text.includes('SyntaxError'));
    const large = await request('/api/v1/decisions', { body: { text: 'a'.repeat(64 * 1024) } });
    capture(holdout, '64 KiB request cap returns JSON 413', large.status === 413 && !!large.json?.execution_metadata);
    const media = await request('/api/v1/decisions', { raw: '{}', headers: { 'content-type': 'text/plain' } });
    capture(holdout, 'Unsupported media type returns 415', media.status === 415);
    const array = await request('/v1/memory-graph/conversation', { body: { payload: [] } });
    capture(holdout, 'Agent payload must be an object', array.status === 400);
    const preflight = await request('/api/v1/decisions', { method: 'OPTIONS', headers: { origin: 'https://attacker.example' } });
    capture(holdout, 'Untrusted preflight is denied', preflight.status === 403 && !preflight.headers.has('access-control-allow-origin'));
    const trusted = await request('/health', { method: 'GET', headers: { origin: 'https://allowed.example' } });
    capture(holdout, 'Configured CORS is exact and varies by origin', trusted.headers.get('access-control-allow-origin') === 'https://allowed.example' && /origin/i.test(trusted.headers.get('vary') || ''));
    const event = { type: 'unicode', value: 'é🧠' };
    const bytes = await request('/events', { body: event });
    capture(holdout, 'Event payload size counts UTF-8 bytes', bytes.json?.received?.payload_size_bytes === Buffer.byteLength(JSON.stringify(event)));
    const eventArray = await request('/events', { body: [] });
    capture(holdout, 'Events require an object envelope', eventArray.status === 400);
    const pagination = await request('/api/v1/decisions?limit=1junk', { method: 'GET' });
    capture(holdout, 'Malformed pagination is rejected', pagination.status === 400);
    capture(anchor, 'Health remains healthy', trusted.status === 200 && trusted.json?.healthy === true);
    const allowed = await request('/api/v1/decisions', { method: 'OPTIONS', headers: { origin: 'https://allowed.example' } });
    capture(anchor, 'Trusted preflight remains available', allowed.status === 204);
    const id = randomUUID();
    const decision = await request('/api/v1/decisions', { body: { agent_id: id, decision_type: 'test' } });
    const query = await request(`/api/v1/decisions?agent_id=${id}&limit=1&offset=0`, { method: 'GET' });
    capture(anchor, 'Capture and filtered query still round-trip', decision.status === 200 && query.json?.total === 1 && query.json?.items?.[0]?.id === decision.json?.id);
    const mixed = await request('/api/v1/decisions/batch', { body: [{ agent_id: id }, null] });
    capture(anchor, 'Mixed batch item receipts are preserved', mixed.status === 200 && mixed.json?.items?.[0]?.success === true && mixed.json?.items?.[1]?.success === false);
    const absent = await request('/v1/memory-graph/conversation', { body: { payload: { text: 'hello' } } });
    capture(anchor, 'Unavailable agent does not report execution success', absent.status === 501 && absent.json?.error?.error_code === 'AGENT_NOT_AVAILABLE');
    const missing = await request('/missing', { method: 'GET' });
    capture(anchor, 'Unknown route remains JSON 404', missing.status === 404 && missing.json?.error === 'Not found');
    return { holdout: { checks: holdout }, anchor: { checks: anchor } };
  } finally {
    server.closeAllConnections();
    await new Promise((done) => server.close(done));
  }
}
