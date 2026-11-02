import test from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'node:net';
import { spawn } from 'node:child_process';
import { once } from 'node:events';

test('Real Functions Framework preserves health and enforces pre-parsed body quota', async () => {
  const reservation = createServer();
  await new Promise((done) => reservation.listen(0, '127.0.0.1', done));
  const port = reservation.address().port;
  await new Promise((done) => reservation.close(done));
  const child = spawn(process.execPath, ['node_modules/@google-cloud/functions-framework/build/src/main.js', '--target=api', '--source=functions', `--port=${port}`], { stdio: ['ignore', 'pipe', 'pipe'] });
  let logs = '';
  child.stdout.on('data', (chunk) => { logs += chunk; });
  child.stderr.on('data', (chunk) => { logs += chunk; });
  try {
    let healthy = false;
    for (let attempt = 0; attempt < 100; attempt++) {
      try {
        const response = await fetch(`http://127.0.0.1:${port}/health`, { signal: AbortSignal.timeout(500) });
        healthy = response.ok && (await response.json()).healthy === true;
        if (healthy) break;
      } catch {}
      if (child.exitCode !== null) break;
      await new Promise((done) => setTimeout(done, 50));
    }
    assert.equal(healthy, true, logs);
    const response = await fetch(`http://127.0.0.1:${port}/api/v1/decisions`, {
      method: 'POST', headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ text: 'a'.repeat(64 * 1024) }),
    });
    assert.equal(response.status, 413);
    assert.equal((await response.json()).error.error_code, 'PAYLOAD_TOO_LARGE');
  } finally {
    const stopped = once(child, 'exit');
    if (child.exitCode === null) child.kill('SIGTERM');
    if (child.exitCode === null) await stopped;
  }
});
