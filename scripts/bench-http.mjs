import { createServer } from 'node:http';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { performance } from 'node:perf_hooks';

const { api } = await import(pathToFileURL(resolve(process.argv[2] || '.', 'functions/index.js')).href);
const server = createServer(api);
await new Promise((done) => server.listen(0, '127.0.0.1', done));
const url = `http://127.0.0.1:${server.address().port}/api/v1/decisions`;
const invoke = async () => {
  const response = await fetch(url, { method: 'POST', headers: { 'content-type': 'application/json' }, body: '{"agent_id":"benchmark","decision_type":"capture"}' });
  if (response.status !== 200) throw new Error(`Unexpected status ${response.status}`);
  await response.arrayBuffer();
};
try {
  for (let i = 0; i < 50; i++) await invoke();
  const rounds = [];
  for (let round = 0; round < 5; round++) {
    const start = performance.now();
    for (let i = 0; i < 200; i++) await invoke();
    rounds.push((performance.now() - start) / 200);
  }
  console.log(JSON.stringify({ workload: '50 warmup + 5 rounds of 200 sequential real HTTP decision captures', msPerRequest: rounds, medianMs: [...rounds].sort((a, b) => a - b)[2], peakRssBytes: process.resourceUsage().maxRSS * 1024 }));
} finally {
  server.closeAllConnections();
  await new Promise((done) => server.close(done));
}
