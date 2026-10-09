import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';

const [baseline, candidate, runtime] = process.argv.slice(2).map(value => resolve(value));
const flywheel = await import(pathToFileURL(join(runtime, 'node_modules/@metaharness/flywheel/dist/index.js')));
const contract = JSON.parse(readFileSync(new URL('./contract.json', import.meta.url), 'utf8'));
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const run = (cwd, command, args, timeout = 120000) => {
  const result = spawnSync(command, args, { cwd, encoding: 'utf8', timeout });
  return {
    exit: result.status,
    signal: result.signal,
    stdoutTail: result.stdout.slice(-4000),
    stderrTail: result.stderr.slice(-4000),
  };
};
const audit = cwd => {
  const result = run(cwd, 'npm', ['audit', '--omit=dev', '--json']);
  let parsed = {};
  try { parsed = JSON.parse(result.stdoutTail); } catch {
    const full = spawnSync('npm', ['audit', '--omit=dev', '--json'], { cwd, encoding: 'utf8', timeout: 120000 });
    try { parsed = JSON.parse(full.stdout); } catch { parsed = {}; }
  }
  return { ...result, vulnerabilities: parsed.metadata?.vulnerabilities ?? null };
};
const smokeCode = `
  import { api } from './functions/index.js';
  const server=api.listen(0,'127.0.0.1');
  await new Promise(r=>server.once('listening',r));
  const base=\`http://127.0.0.1:\${server.address().port}\`;
  try {
    const health=await fetch(base+'/health');
    const capture=await fetch(base+'/api/v1/decisions',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({agent_id:'flywheel',decision_type:'gate'})});
    const query=await fetch(base+'/api/v1/decisions');
    if (health.status!==200 || capture.status!==200 || query.status!==200) process.exitCode=1;
    console.log(JSON.stringify({health:health.status,capture:capture.status,query:query.status}));
  } finally {
    server.closeAllConnections();
    await new Promise((r,j)=>server.close(e=>e?j(e):r()));
  }
`;
const measurements = [];
const evaluator = async (policy, suite) => {
  const cwd = policy.includeUnusedDependency ? baseline : candidate;
  const logs = {};
  const itemWins = [];
  if (suite.id === 'holdout') {
    logs.install = run(cwd, 'npm', ['ci']);
    logs.audit = audit(cwd);
    itemWins.push(logs.install.exit === 0);
    // Frozen baseline thresholds: promote only a strict reduction from
    // 2 critical and 41 total production findings.
    itemWins.push((logs.audit.vulnerabilities?.critical ?? Infinity) < 2);
    itemWins.push((logs.audit.vulnerabilities?.total ?? Infinity) < 41);
  } else {
    logs.installForTests = run(cwd, 'npm', policy.includeUnusedDependency
      ? ['ci', '--ignore-scripts'] : ['ci']);
    logs.tests = run(cwd, 'npm', ['run', 'test:execution']);
    logs.smoke = run(cwd, 'node', ['--input-type=module', '-e', smokeCode]);
    const runtimeSources = [
      readFileSync(join(cwd, 'functions/index.js'), 'utf8'),
      ...['src/execution/agent-adapter.ts','src/execution/artifact-builder.ts','src/execution/executor.ts','src/execution/index.ts']
        .map(file => readFileSync(join(cwd, file), 'utf8')),
    ].join('\n');
    itemWins.push(logs.installForTests.exit === 0);
    itemWins.push(logs.tests.exit === 0);
    itemWins.push(logs.smoke.exit === 0);
    itemWins.push(!runtimeSources.includes('claude-flow'));
  }
  const passed = itemWins.filter(Boolean).length;
  const score = {
    primary: passed / itemWins.length,
    noopRate: 1 - passed / itemWins.length,
    costPerWin: 0,
    regressed: suite.id === 'anchor' && passed !== itemWins.length,
    itemWins,
  };
  measurements.push({ policy, suite: suite.id, score, logs });
  return score;
};

const pinnedGateFingerprint = flywheel.gateFingerprint(flywheel.meetsPromotionRule);
const result = await flywheel.runFlywheelGenerations({
  rootPolicy: { includeUnusedDependency: true },
  proposer: async () => ({
    value: false,
    summary: 'Remove unused claude-flow production dependency and lockfile graph',
    inverse: { files: ['package.json', 'package-lock.json'], baselineCommit: contract.baselineCommit },
  }),
  evaluator,
  holdout: { id: 'holdout', items: ['clean-install', 'critical-audit-threshold', 'total-audit-threshold'] },
  anchor: { id: 'anchor', items: ['install-for-tests', 'execution-tests', 'function-smoke', 'no-runtime-import'] },
  maxGenerations: 2,
  cacheEvaluations: true,
  signer: flywheel.makeSigner(),
  promotionRule: flywheel.meetsPromotionRule,
  dataSource: 'LOCAL_PROCESS_REAL_NPM_AND_HTTP',
  now: generation => `2026-10-08-generation-${generation}`,
});
const replay = flywheel.verifyReplayBundle(result.replayBundle, { pinnedGateFingerprint });
console.log('FLYWHEEL_DEBUG', JSON.stringify({
  promotions: result.promotions.length,
  finalPolicy: result.finalPolicy,
  measurements: measurements.map(x => ({
    policy: x.policy,
    suite: x.suite,
    score: x.score,
    installExit: x.logs.install?.exit ?? x.logs.installForTests?.exit,
    testExit: x.logs.tests?.exit,
    smokeExit: x.logs.smoke?.exit,
    audit: x.logs.audit?.vulnerabilities,
  })),
}, null, 2));
assert.equal(replay.pass, true);
assert.equal(result.promotions.length, 1);
assert.equal(result.finalPolicy.includeUnusedDependency, false);

const report = {
  contract,
  hashes: {
    baselinePackage: hash(readFileSync(join(baseline, 'package.json'))),
    candidatePackage: hash(readFileSync(join(candidate, 'package.json'))),
    baselineLock: hash(readFileSync(join(baseline, 'package-lock.json'))),
    candidateLock: hash(readFileSync(join(candidate, 'package-lock.json'))),
  },
  measurements,
  pinnedGateFingerprint,
  replay,
  promotions: result.promotions.length,
  finalPolicy: result.finalPolicy,
  replayBundle: result.replayBundle,
  billingUsd: 0,
  runtimeCostAndEnergy: 'unmeasured',
  boundary: 'local dependency install, audit, repository tests and loopback HTTP; no deployment or Rust-build claim',
};
writeFileSync(new URL('./flywheel-results.json', import.meta.url), JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({
  promotions: report.promotions,
  replay: replay.pass,
  measurements: measurements.map(x => ({
    policy: x.policy,
    suite: x.suite,
    primary: x.score.primary,
    noopRate: x.score.noopRate,
    itemWins: x.score.itemWins,
    installExit: x.logs.install?.exit ?? x.logs.installForTests?.exit,
    testExit: x.logs.tests?.exit,
    smokeExit: x.logs.smoke?.exit,
    audit: x.logs.audit?.vulnerabilities,
  })),
}, null, 2));

