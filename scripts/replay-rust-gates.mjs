// Project actual command evidence through Flywheel, with live Node compatibility anchors.
// Usage: node scripts/replay-rust-gates.mjs LEDGER BASELINE CANDIDATE FLYWHEEL_PACKAGE_DIR
import { readFileSync } from 'node:fs';
import { execFileSync, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { resolve, join } from 'node:path';
import { pathToFileURL } from 'node:url';

const [ledgerPath, baselineRepo, candidateRepo, packageDir] = process.argv.slice(2);
const manifest = JSON.parse(readFileSync(join(packageDir, 'package.json'), 'utf8'));
if (manifest.name !== '@metaharness/flywheel' || manifest.version !== '0.1.12') {
  throw new Error('Expected isolated @metaharness/flywheel@0.1.12');
}
const { runFlywheelGenerations, meetsPromotionRule, gateFingerprint, makeSigner, verifyReplayBundle } =
  await import(pathToFileURL(resolve(packageDir, 'dist/index.js')).href);
const ledger = JSON.parse(readFileSync(ledgerPath, 'utf8'));
const sourcePaths = ['crates/llm-memory-graph/Cargo.toml', 'crates/llm-memory-graph/src/integrations/mod.rs', 'crates/llm-memory-graph-types/src/edges.rs', 'crates/llm-memory-graph-types/src/lib.rs', 'Cargo.lock', 'functions/index.js', 'package.json', 'package-lock.json'];
for (const label of ['baseline', 'candidate']) {
  const cwd = label === 'baseline' ? baselineRepo : candidateRepo;
  for (const path of sourcePaths) {
    const expected = execFileSync('git', ['show', `${ledger[label].revision}:${path}`], { cwd });
    if (!readFileSync(join(cwd, path)).equals(expected)) throw new Error(`Source drift: ${label}:${path}`);
  }
}
const gates = ['fmt', 'clippy', 'test', 'build', 'audit'];
const anchorResults = new Map();
const evaluator = async (policy, suite) => {
  let wins;
  if (suite.id === 'memory-graph-full-rust-v1') {
    const evidence = ledger[policy.implementation];
    wins = gates.map(gate => {
      const result = evidence.results.find(row => row.gate === gate);
      if (!result) throw new Error(`Missing measured gate: ${gate}`);
      return result.exit === 0 && result.error === null && result.signal === null;
    });
  } else {
    const cwd = policy.implementation === 'baseline' ? baselineRepo : candidateRepo;
    const run = spawnSync('npm', ['test'], { cwd, encoding: 'utf8', timeout: 120_000, maxBuffer: 4 * 1024 * 1024 });
    wins = [run.status === 0 && !run.error && !run.signal];
    anchorResults.set(policy.implementation, { exit: run.status, outputSha256: createHash('sha256').update(`${run.stdout || ''}${run.stderr || ''}`).digest('hex') });
  }
  const passed = wins.filter(Boolean).length;
  return { primary: passed / wins.length, noopRate: 1 - passed / wins.length, costPerWin: passed ? wins.length / passed : 999, regressed: suite.id === 'memory-graph-node-anchor-v1' && passed !== wins.length, itemWins: wins };
};
const path = 'crates/llm-memory-graph/src/integrations/mod.rs';
const parentBytes = execFileSync('git', ['show', `${ledger.baseline.revision}:${path}`], { cwd: baselineRepo, encoding: 'utf8' });
const inverse = { path, parentBytes, hash: createHash('sha256').update(parentBytes).digest('hex') };
const diff = execFileSync('git', ['diff', ledger.baseline.revision, ledger.candidate.revision, '--', 'crates/llm-memory-graph/Cargo.toml', path, 'crates/llm-memory-graph-types/src/edges.rs', 'crates/llm-memory-graph-types/src/lib.rs'], { cwd: candidateRepo, encoding: 'utf8' });
if (!diff.trim()) throw new Error('Missing source rollback patch');
execFileSync('git', ['apply', '--check', '--reverse'], { cwd: candidateRepo, input: diff });
const pinnedGateFingerprint = gateFingerprint(meetsPromotionRule);
const result = await runFlywheelGenerations({
  rootPolicy: { implementation: 'baseline' },
  proposer: async () => ({ value: 'candidate', summary: 'Bounded Rust retry/manifest and executable documentation repair; full gates still mandatory', inverse }),
  evaluator,
  promotionRule: meetsPromotionRule,
  holdout: { id: 'memory-graph-full-rust-v1', items: gates },
  anchor: { id: 'memory-graph-node-anchor-v1', items: ['npm test'] },
  mutationTargets: ['implementation'],
  maxGenerations: 1,
  signer: makeSigner(),
  dataSource: 'MEASURED_RUOS_COMMAND_REPLAY_WITH_LIVE_NODE_ANCHORS',
});
const replay = verifyReplayBundle(result.replayBundle, { pinnedGateFingerprint });
if (!replay.pass) throw new Error('Signed replay verification failed');
console.log(JSON.stringify({
  model: 'none; fixed command contract',
  package: `${manifest.name}@${manifest.version}`,
  pinnedGateFingerprint,
  baselineRevision: ledger.baseline.revision,
  candidateRevision: ledger.candidate.revision,
  rollback: { primaryPath: path, hash: inverse.hash, sourcePatchReverseCheck: true, productionRollbackExecuted: false },
  anchorResults: Object.fromEntries(anchorResults),
  finalPolicy: result.finalPolicy,
  liftCurve: result.liftCurve,
  replayVerdict: replay,
  replayBundle: result.replayBundle,
}, null, 2));
