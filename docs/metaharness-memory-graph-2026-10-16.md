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
  hidden or waived. That initial candidate changed no Rust source or Cargo.lock;
  the bounded Rust recovery below subsequently changes Rust source, not Cargo.lock.

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

## Bounded Rust recovery (October 16, evening)

Resumed PR #3 rather than creating a duplicate. Baseline is the previous PR head
`647d9be6e4518dcfe509287a10c033765465e6ae`; main is still
`d9b9dc5eb7ac313eedc2831dce6af46641bfaaa7`. Separate worktrees and a single writer
were used. The production tree measured by the Rust evaluator is
`131262670434b85b7626bebf907f9ae5eb3b82f7` / tree
`e4f6e0bd5b56bcfe74611b340d7267ff8ae3aa47`. Later evidence/runner/CI additions do
not change the measured Rust implementation or Cargo.lock.

The retry helper imported the graph-specific `Result<T>` alias but supplied an
`IntegrationError` type parameter. Both the helper's return and future output
now explicitly use `std::result::Result<T, IntegrationError>`. Four regression
tests cover owned success, terminal error preservation, non-retried client
errors and disabled timeout retries. These new tests have NOT executed because
other core compilation failures still prevent building that test binary.
The existing public benchmark implementation imports tempfile outside cfg(test),
so it is now correctly declared as a normal dependency as well as a dev dependency.
No mandatory ecosystem dependency was removed or modified.

Five executable edge examples used the wrong crate path; the prompt example
used obsolete string IDs and fields. They now use the actual public types crate
and typed constructor. No doc example was ignored, removed or marked no_run.
CI adds an independent types-crate test/doc job; the aggregate still requires
the entire Rust matrix. All original workspace gates remain mandatory.

| Fresh real measurement | Prior PR | Recovery |
| --- | ---: | ---: |
| Core library compiler errors | 119 | 27 |
| Core library-test compiler errors | 129 | 38 |
| Types unit tests | 86/86 | 86/86 |
| Types executable doc tests | 0/6 | 6/6 |
| Existing Node execution tests | 70/70 | 70/70 |
| HTTP integrity / compatibility | 12/12; 6/6 | 12/12; 6/6 |
| Production npm advisories | 0 | 0 |
| Full Rust gates passing | 0/5 | 0/5 |

Command exits on both trees: fmt=1, strict Clippy=101, workspace test=101,
all-features build=101, audit=1. Types test changes from 101 to 0. Strict Clippy
reports 29 library errors; candidate also surfaced 31 lib-test lint errors.
Cargo audit still reports 10 vulnerabilities and 11 warnings. Compilation error
counts are partial diagnostic progress, never the acceptance score.

The RuOS bootstrap was rerun after restart, reinstalling pkg-config/libssl-dev;
`pkg-config --modversion openssl` returned 3.0.13. Protobuf compiler and headers
were restored separately. Actual Rust was 1.99.0; Cargo audit was 0.22.2.
`node scripts/evaluate-rust.mjs REPO` executes the exact gates and emits exits,
output hashes and test summaries. The adjacent `metaharness-memory-graph-rust-2026-10-16.json`
records real commands, revisions, timings, full 58-repository inventory review
and all 57 named deferred repositories. Timings are diagnostic, not a benchmark
or performance claim. No new application performance improvement is claimed.

Ruflo 3.56.3 pre-task `task-mvbmqw9i` and hierarchical swarm
`swarm-1792195197217-8rfmvn` executed, with one writer and no delegated workers.
Offline post-task used the research-lab seccomp launcher, checksum-pinned local
MiniLM revision `751bff37182d3f1213fa05d7196b954e230abad9`, Transformers 4.3.1,
and remote loading disabled. It recorded success=false for the blocked overall
task; learningUpdates.available/outcomePersisted, feedback.recorded and
trajectory.recorded were all true (two feedback updates). A real 384-dimensional
embedding was verified. No Hugging Face task-data egress or hash fallback.

MetaHarness 0.4.17 `score .` exited 0 with fit 72 and constraints 6/6. Darwin
0.10.3 used `evolve . --generations 1 --children 2 --concurrency 1 --seed
2026101620 --sandbox real --selection pareto`, fixed npm test and no model calls.
Both children (contextBuilder and retryPolicy) tied baseline at 0.985 and were
discarded: NO_IMPROVEMENT. This real-test substrate does not measure the effect
of those policy surfaces; no harness optimization is claimed.

Flywheel 0.1.12 really ran the unchanged default promotion rule over the five
frozen full-Rust outcomes, with live npm-test compatibility anchors. Rust data
is explicitly MEASURED_RUOS_COMMAND_REPLAY, not a claim that Flywheel itself
opened a remote shell. Both primary scores are 0; anchors remain 1. The candidate
was REJECTED (noop_rate_not_improved), baseline retained, zero promotions.
All eight signed replay checks passed. The adjacent Rust Flywheel JSON preserves
the rejected lineage and real measurement provenance. Reproduce with:

```bash
node scripts/replay-rust-gates.mjs \
  docs/metaharness-memory-graph-rust-2026-10-16.json \
  BASELINE_CHECKOUT CANDIDATE_CHECKOUT ISOLATED_FLYWHEEL_0_1_12_PACKAGE_DIR
```

The replay rejects drift in evaluated Rust/lock/Node source bytes and requires
the measured Git objects. The parent integration-module bytes are hashed in the
inverse; `git apply --check --reverse` verified the full four-file source rollback
patch applies. No production rollback executed. Revert the Rust recovery and
types-example commits (plus their evidence/CI additions if desired) to restore
the prior PR; do not revert the already-protected HTTP behavior unintentionally.

Outcome: EXECUTED, types component VALIDATED, full candidate BLOCKED, NOT MERGED.
Remaining repairs: telemetry/vault adapters reference missing enum variants and
session fields, error conversions and generic lifetime bounds; workspace
formatting, strict lints and locked dependency vulnerabilities remain unresolved.
Architecture and security constraints were preserved; no service-owned database,
deployment, publication, paid provisioning or protection bypass was introduced.
