# MetaHarness cycle — 2026-10-08

## Result

Remove the unused `claude-flow@2.7.41` production dependency from the deployable Cloud Function package and prune its lockfile graph. Repository-wide runtime-source search found no imports; `CLAUDE.md` invokes the orchestration CLI through `npx`, so that developer workflow remains available without shipping it in the service artifact.

This preserves the existing Rust core, TypeScript integration, RuVector architecture, GCP configuration and persistence boundaries. It does not deploy or publish anything.

## Pinned execution ledger

- Upstream MetaHarness commit: `ea287d6ef7548b0b32fa3e20956fa548cfe51edb`
- `metaharness@0.4.17`: `metaharness score memory-graph --json`
  - fit 72, compile confidence 100, task coverage 100, tool safety 95, memory usefulness 53, hard constraints 6/6
- `@metaharness/darwin@0.10.3`: `metaharness-darwin evolve . --generations 1 --children 2 --concurrency 1 --seed 1008 --sandbox real --mutator deterministic`
  - baseline 0.435; planner 0.435; reviewer 0.435; winner baseline; delta +0.000
  - repository has no standard `npm test` script, so Darwin reported pass 0.00 and no harness lift
- `@metaharness/flywheel@0.1.12`: `node docs/metaharness/2026-10-08/evaluate.mjs <baseline> <candidate> <runtime>`
  - unmodified `meetsPromotionRule`; real npm install/audit, Vitest and loopback HTTP evaluation
  - one dependency-policy candidate promoted; repeated proposal memoized
  - signed replay verification passed

Ruflo federation identity, live claims and Seraphina guidance were executed before work. `memory-graph` was unclaimed; the only live claim was on an unrelated upstream resource. The recovery branch and isolated worktrees prevent overlap.

## Measurements

| Gate | Baseline | Candidate |
|---|---:|---:|
| Clean `npm ci` | exit 1 (`hnswlib-node` native install) | exit 0 |
| Production lock packages | 563 | 154 |
| Production audit total | 41 | 9 |
| Critical / high audit findings | 2 / 27 | 1 / 2 |
| Root execution tests | 70/70 | 70/70 |
| Cloud Function smoke | 3/3 | 3/3 |
| Flywheel holdout | 0/3 | 3/3 |
| Flywheel anchor | 4/4 | 4/4 |
| Flywheel promotion/replay | n/a | 1 / pass |

The TypeScript client independently builds, lints and passes formatting in both trees. Its existing test state is unchanged: 120 pass, 38 fail and 16 skip. The failures include stale enum expectations, gRPC fixture loading, retry fake-timer timeouts and error mapping; they are outside this package-only candidate.

## Rejected experiments and uncertainty

- Darwin planner and reviewer harness variants tied the baseline and were discarded.
- The first Flywheel invocation used the wrong proposer value shape, produced zero promotion, and was rejected. The evaluator contract was corrected without changing thresholds, then rerun from scratch.
- Node 24 emits an engine warning for `cloudevents` (declared through Node 22); supported deployment runtime remains Node 20.
- Rust tooling is unavailable in this executor, so no Rust-build claim is made.
- Nine production audit findings remain in the necessary Cloud Functions/Express graph and require a separate bounded upgrade cycle.

## Rollback

Revert the implementation commit to restore `claude-flow` and the prior lock graph. This also restores the clean-install failure and removed advisory surface, so rollback is only for an unexpected compatibility issue.

