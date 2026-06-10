# feat: close the loop — regression diff + baseline + CI gate

## Overview

Addresses the Fable review's two minibench CRITICALs (F1 "nothing runs the
workbench"; F2 "no baseline storage or diffing"). The kernel can now diff a
run set against a committed baseline and report regressions; the CLI exits
non-zero on any; and minibench's **first CI** runs that gate on every PR.
The harness can finally say a run is *worse*, not merely *different*.

## Motivation

The workbench exists to prove claims, but it could only measure them — no
run was automated, no result outlived `/tmp`, no baseline was stored, the
CLI exited 0 regardless of divergence. This closes that loop for the demo
fixtures, establishing the mechanism the real product baselines will use.

## Changes in Detail

- **kernel** (`crates/kernel/src/lib.rs`): `diff(baseline, current) ->
  RegressionReport`. A regression = a state variable whose status got more
  severe (Pass<Warn<Fail<Blocked) or whose score dropped, matched by
  (experiment, variant, state-var). `RegressionReport::is_clean()`. Test:
  self-diff is clean, a worsened cell is flagged.
- **cli** (`crates/cli/src/main.rs`): `minibench diff <baseline-dir>
  <current-dir>` prints regressions and exits `3` when any are found
  (`REGRESSION_EXIT_CODE`).
- **baseline** (`fixtures/baseline/`): a frozen known-good copy of the
  experiment fixtures — the diff target.
- **bb.edn**: `regression-gate` task (`minibench diff fixtures/baseline
  fixtures/experiments`).
- **CI** (`.github/workflows/ci.yml`, minibench's first): `cargo test` the
  gate path (kernel + cli + contract), then `bb regression-gate`. Mints a
  `ci-bot-read` App token and rewrites github.com SSH→HTTPS-with-token so
  cargo can fetch the private workbench-contract git dep (the #307 pattern).
  The data-plane crate is excluded — it pulls thesium-app-foundation, whose
  own `standards/miniforge` submodule cargo inits via libgit2 (bypassing the
  token); it stays covered by the local `bb pre-commit`.

## Strata Affected

- `kernel` (domain — the diff), `cli` (adapter — the subcommand), CI + bb
  tooling, `fixtures/baseline`.

## Testing Plan

- `bb pre-commit`: cargo fmt + clippy + tests (incl. the diff test) + swift
  build — green.
- Manual: `minibench diff fixtures/baseline fixtures/experiments` → "no
  regressions vs baseline", exit 0; a worsened copy → "regressions: 1 …
  pass 0.88 -> fail 0.10", exit 3.
- CI runs the gate on this PR (validates the App-token fetch + the gate).

## Deployment Plan

N/A — local/CI tooling.

## Related Issues/PRs

- Fable review (`core-review-etl-validation-2026-06-09.md`) minibench F1+F2.
- Follow-up (separate, thesium-workflows): populate the contract's
  `StateEvaluation.regression` field from the adapters; point the experiment
  scripts at the production adapter, not the reference one (Fable F5).
- Follow-up (thesium-app-foundation): set `update = none` on its
  `standards/miniforge` submodule so minibench's full workspace (incl. the
  data-plane) builds in CI without the libgit2 submodule-auth gap.

## Checklist

- [x] Standards gap analysis vs `standards/miniforge/` before push.
- [x] Named constants (epsilon, exit code); stratified (pure kernel diff).
- [x] bb-over-shell: the gate is a bb task; CI calls it.
- [x] PR doc (721); proprietary headers on new files.
- [x] App token via env, never interpolated into a shell command line.
- [x] Pre-commit gate green; not bypassed.
