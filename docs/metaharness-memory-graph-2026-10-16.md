# Decision integrity improvement cycle

## Baseline and acceptance

Baseline main: `d9b9dc5eb7ac313eedc2831dce6af46641bfaaa7`.
The fixed live evaluator in `tests/http-contract.mjs` exercises actual HTTP
requests against the exported Express application, without substituting agents
or storage. All 12 integrity criteria failed on main; all six compatibility
anchors passed. The acceptance criteria were frozen before implementation.

The implementation protects server-generated decision IDs/timestamps and batch
transport receipts, rejects batches above 1,000 entries before any write, applies
a 64 KiB request cap and JSON media-type checks, normalizes app parser errors,
validates agent/event object envelopes and pagination, reflects configured CORS
origins exactly, and reports UTF-8 byte sizes accurately. The pre-existing default
wildcard behavior remains available; set ALLOWED_ORIGINS to restrict browser access.

Functions Framework can pre-parse requests, so the application also checks its
raw body. A separate real framework-process smoke test verifies health and the
over-limit response through the actual framework, not only the exported app.
Parser errors rejected outside the application by a hosting framework remain
the hosting layer's responsibility; the app JSON error contract is scoped to
errors reaching its middleware.

## Measurements and limitations

- Existing execution tests: 70/70 before and after.
- HTTP integrity: 0/12 -> 12/12; compatibility anchors: 6/6 -> 6/6.
- One real Functions Framework process smoke test passed, including its
  pre-parsed-body quota path. Node's test runner reports 20 tests including the
  parent contract container; there are 18 HTTP cases plus the framework smoke.
- Production npm advisories: 9 (including one critical and two high) -> 0.
  Functions Framework 5.0.5 and Express 4.22.1 are pinned; transitive UUID 11.1.1
  and qs 6.16.1 are pinned by overrides and exercised by the actual framework.
- MetaHarness 0.4.17: fit 72 -> 72, hard constraints 6/6 -> 6/6.
- Benchmark: five rounds of 200 sequential real HTTP decision captures after
  50 warmups. Median round mean: 1.901 ms baseline, 1.985 ms candidate (+4.4%).
  Ranges overlap; this is a small local overhead observation, not a speedup or
  statistical equivalence claim. Reproduce with `node scripts/bench-http.mjs REPO`.
- Rust baseline required installing protobuf-compiler and libprotobuf-dev after
  the restored OpenSSL bootstrap. Real locked tests then failed with 129 compiler
  errors; the all-features build failed with 119 errors. Strict Clippy and
  rustfmt also failed. Cargo audit 0.22.2 found 10 vulnerabilities and 11 warnings
  (six unmaintained, four unsound, one yanked). These baseline failures are not
  hidden or waived. No Rust source or Cargo.lock change is made in this branch.

The new Linux CI runs Node tests/audit and independent Rust fmt, strict Clippy,
locked tests, locked build and audit gates. The aggregate gate requires every
matrix entry. Release workflows and protections are not weakened. A passing
Node job or local Flywheel result does not authorize a merge while Rust is red.

## Tool ledger

- Upstream MetaHarness revision: `e0dfd44da72b7adfb7c57fdd42d124be58ed7086`.
- Ruflo 3.56.3: pre-task `task-mvb9vl9m`; real hierarchical coordination
  initialized as `swarm-1792173615024-fvc0rr`. Coordination is not implementation.
- MetaHarness: `metaharness score . --json` exited zero on both trees.
- Darwin 0.10.3: `evolve . --generations 1 --children 2 --concurrency 1
  --seed 2026101614 --sandbox real --selection pareto`. Fixed `npm test` and no
  model calls. Both retryPolicy children tied the 0.985 harness baseline and
  were rejected. This substrate does not distinguish harness policy quality;
  no harness optimization is claimed. An initial run tied; retrying in its same
  archive was refused because child IDs already existed. The archive was
  preserved separately and the final contract was rerun in a fresh archive.
- Flywheel 0.1.12: live `runFlywheelGenerations`, unchanged promotion rule,
  signed lineage, parent-byte rollback inverse and independent replay executed.
  Primary 0 -> 1 with unchanged anchors; all eight integrity checks passed.
  The JSON receipt is adjacent. One promotion is not a multi-generation milestone.

## Guidance and architecture

Core-memory returned no screened playbook entries or anti-patterns. Explicit
user guidance remains authoritative: retain the Rust core/TypeScript boundary,
environment configuration, Linux-first deterministic checks, and RuVector as
the authorized durable persistence layer. No persistence service, deployment,
package publication or paid infrastructure is introduced.

The decision routes already use a bounded per-instance Map. This branch does
not claim cross-instance durability or complete architectural compliance for
that legacy behavior. Migrating it to an existing authorized RuVector interface
requires a separate contract-backed change; no Firestore plan is introduced.

Coverage: full inventory pagination returned 58 nonarchived repositories (29
public, 29 private), then an empty second page. Current main and the latest
workflow result were read for all 58; memory-graph received substantive work.
The remaining 57 repositories were deferred. Existing improvement PRs were
inspected and not duplicated.

## Reproduction and rollback

Run `npm ci --ignore-scripts`, `npm test`, `npm audit --omit=dev`, and
`node scripts/evaluate-http.mjs REPO`. Install pkg-config, libssl-dev,
protobuf-compiler and libprotobuf-dev for Rust, then run the exact CI commands.
Run `node scripts/bench-http.mjs REPO` for the bounded local workload.

Revert the candidate commit if published. The signed Flywheel inverse reproduces
the parent `functions/index.js` bytes and SHA-256; git revert restores the full
implementation, dependency and workflow change. No production rollback or
deployment is performed by this cycle.
