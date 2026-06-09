# feat: live tenant feeds — fixtures from the real adapters + regen task

## Overview

Closes the **live tenant feeds** deferred slice for the career + portfolio
tenants: minibench's demo fixtures are now genuine `workbench_snapshot/v1`
output from the real `thesium-workflows` adapters, not hand-written JSON,
with a reproducible Babashka task to regenerate them.

## Motivation

The harness existed but consumed hand-faked fixtures. Driving the demo
fixtures from the real adapters proves the producer → consumer seam end to
end and keeps the comparison matrix honest (real scores, real divergence).

## Base Branch

`main`.

## Depends On

- [thesium-workflows#309](https://github.com/miniforge-ai/thesium-workflows/pull/309)
  — the `bb workbench:*` tasks were broken under `-M:dev` (kit git-dep
  regression). The regen task needs that fix in the producer checkout. **Merged.**

## Layer

Demo fixtures + dev tooling. No crate API change; one test assertion updated.

## What This Adds

- `bb regen-fixtures` (`tasks/regen.clj`) runs the real `bb workbench:*`
  tasks on the synthetic, non-personal inputs in `fixtures/inputs/` and
  writes `fixtures/experiments/{opus-semantic,haiku-mechanical}.json` (one
  career lens experiment, two divergent variants) + `fixtures/portfolio-daily.json`.
- Regenerated those fixtures from the real adapters; updated the kernel
  comparison test's spread assertion to the real `0.45` (0.88 vs 0.43).
- `bb.edn` introduced (bb-over-shell): `pre-commit` gate task + `regen-fixtures`;
  the old `scripts/regen-fixtures.sh` is deleted.
- Vendored `standards/miniforge` submodule; added `CLAUDE.md`; `.githooks/pre-commit`
  now calls `bb pre-commit`.
- README: documents the live-feed flow; fixes the stale "sibling path
  dependencies" note (now git deps).

`fixtures/sample-snapshot.json` stays hand-written — the miniforge tenant
placeholder, whose adapter does not exist yet.

## Strata Affected

- **Fixtures / tooling** — `fixtures/`, `tasks/`, `bb.edn`, `.githooks/`.
- **kernel (test only)** — one assertion value; no production code change.

## Testing Plan

- `bb pre-commit` (cargo fmt + clippy + tests) green.
- `bb regen-fixtures` round-trips against a `thesium-workflows` checkout.
- `cargo run -p minibench-cli -- compare fixtures/experiments` shows the
  real divergence matrix (pass 0.88 vs fail 0.43, ◆).

## Deployment Plan

N/A — dev/demo fixtures and tooling only.

## Related Issues/PRs

- thesium-workflows#309 (dependency, merged).

## Checklist

- [x] Standards gap analysis run against `standards/miniforge/` before push.
- [x] bb-over-shell (740): shell script replaced by a bb task; `.sh` deleted.
- [x] PR doc (721) present.
- [x] Pre-commit gate green; not bypassed.
- [x] Proprietary headers on new files.
- [x] Synthetic fixtures only — no real tenant data.
