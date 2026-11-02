// Runs the unchanged full-workspace gates; a failed gate is evidence, never a pass.
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';

const cwd = resolve(process.argv[2] || process.cwd());
const commands = [
  ['fmt', ['fmt', '--all', '--', '--check']],
  ['clippy', ['clippy', '--workspace', '--all-targets', '--all-features', '--locked', '--', '-D', 'warnings']],
  ['test', ['test', '--workspace', '--all-features', '--locked']],
  ['build', ['build', '--workspace', '--all-features', '--locked']],
  ['audit', ['audit']],
  ['types', ['test', '-p', 'llm-memory-graph-types', '--all-features', '--locked']],
];
const revision = spawnSync('git', ['rev-parse', 'HEAD'], { cwd, encoding: 'utf8' });
if (revision.status !== 0) throw new Error('Cannot identify evaluated revision');
const rustVersion = spawnSync('rustc', ['--version'], { cwd, encoding: 'utf8' });
if (rustVersion.status !== 0) throw new Error('Rust compiler unavailable');
const results = commands.map(([gate, args]) => {
  const started = Date.now();
  const run = spawnSync('cargo', args, { cwd, encoding: 'utf8', timeout: 600_000, maxBuffer: 16 * 1024 * 1024 });
  const output = `${run.stdout || ''}${run.stderr || ''}`;
  return {
    gate,
    command: ['cargo', ...args].join(' '),
    exit: run.status,
    signal: run.signal,
    error: run.error?.message ?? null,
    elapsedMs: Date.now() - started,
    outputSha256: createHash('sha256').update(output).digest('hex'),
    compilerSummaries: output.split('\n').filter(line => line.startsWith('error: could not compile')),
    testSummaries: output.split('\n').filter(line => line.startsWith('test result:')),
    auditSummaries: output.split('\n').filter(line => /^(error: .*vulnerabilities|warning: .*warnings)/.test(line)),
  };
});
console.log(JSON.stringify({
  revision: revision.stdout.trim(),
  rustVersion: rustVersion.stdout.trim(),
  measuredAt: new Date().toISOString(),
  acceptance: 'Every full-workspace gate and types check must exit zero; no diagnostic-count scoring',
  accepted: results.every(result => result.exit === 0 && result.error === null),
  results,
}));
