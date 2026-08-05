<!--
  Title: Minibench
  Author: Christopher Lester (christopher@miniforge.ai)
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# feat: human-correction loop — labeled expectations over the frozen baseline

## Overview

First slice of the workbench's human-correction loop: a reviewer records a
labeled correction for one comparison cell (what it SHOULD read, and why),
and that correction — not the raw frozen baseline — becomes the expectation
the regression gate enforces for that (experiment, variant, state-var).
Until now the only way to accept a changed result was wholesale baseline
replacement, with no provenance for WHY a change was accepted.

## What this is a slice of

The original workbench design's correction loop: human reviews a cell →
records a correction with a rationale → the labeled expectation drives the
gate. This PR ships the record type, the corrections-aware diff, the
`correct` CLI, and the gate wiring. Later slices: registry refinement from
accumulated corrections, and a correction-queue UI in the Swift shell.
`CorrectionV1` is a candidate for promotion into workbench-contract once a
second consumer (the shell, or the workflows adapters) needs it; it stays
kernel-local until then.

## Changes in Detail

- **kernel** (`crates/kernel/src/lib.rs`):
  - `CorrectionV1` — serde snake_case record: experiment_id /
    variant_label / state_var_id / expected_status (contract
    `StateStatus`, so only the closed status vocabulary parses) /
    expected_score (optional) / rationale (required, non-empty — a
    correction without a why is refused) / corrected_by (required,
    non-empty) / corrected_at (RFC 3339) / snapshot_id (optional).
  - `CorrectionSet::new` — validates every record; duplicate cell keys
    are an error (`CorrectionError::DuplicateKey`), never a silent
    last-wins.
  - `diff_with_corrections(baseline, current, corrections) ->
    CorrectedDiffReport` — every cell is judged against an expectation
    that defaults to the baseline cell and is overridden by a correction
    where one exists. Score drops reuse `SCORE_REGRESSION_EPSILON`. A
    correction with no `expected_score` overrides only the status; the
    baseline score remains the floor. A correction matching a cell absent
    from baseline still judges the current cell (the correction IS the
    expectation). A correction matching nothing in baseline or current
    surfaces in `report.stale` (warning), never vanishes. `report.applied`
    plus the per-regression `corrected` flag record which cells were
    judged against a human label.
- **cli** (`crates/cli/src/main.rs`, `strings.rs`):
  - `minibench correct <corrections-dir> --experiment … --variant …
    --state-var … --status … --rationale … --by … [--score …]
    [--snapshot-id …]` — validates, stamps `corrected_at` (RFC 3339 UTC
    via the `time` crate, the one new dependency), and writes
    `<dir>/<sanitized experiment>__<variant>__<state-var>.json`. The name
    derives from the cell key, so re-correcting the same cell overwrites
    rather than duplicates. Prints what was written.
  - `minibench diff <baseline> <current> [--corrections <dir>]` — loads
    and validates the corrections dir (empty dir is an error, mirroring
    `load_nonempty`), prints stale-correction warnings, applied
    provenance lines, and regressions with expected status/score and a
    `(corrected)` mark; exits `REGRESSION_EXIT_CODE` on regression.
- **gate** (`bb.edn` `regression-gate`): passes
  `--corrections fixtures/corrections` when that directory exists.
- **no committed corrections**: `fixtures/corrections/` ships empty
  (gate wiring is a no-op until it exists). A committed correction is a
  recorded human judgment with attribution; none ships until a human has
  actually made one. The demonstration lives in a kernel test that
  constructs the correction in-test against the committed baseline.

> **Amendment (2026-08-04).** Two corrections to the bullet above, both
> settled by `2026-08-04-feat-commit-corrections-fixture.md`:
>
> - "ships empty" was never literally true. Git does not track empty
>   directories, so `fixtures/corrections/` did not ship at all, and the
>   `fs/exists?` guard in `regression-gate` never fired. The consequence
>   was larger than "no-op": between this PR and 2026-08-04, CI never
>   executed `diff_with_corrections`. The corrected-diff path was covered
>   by kernel unit tests only, never end-to-end through the CLI.
> - The withholding rule — no committed correction until a human has made
>   one — has now been satisfied rather than reversed. The example this
>   doc describes in-test (a 0.40 grounding floor for
>   `career.lens.acme-l4-eval [haiku+mechanical]`) was recorded by a
>   named human through `minibench correct` and committed, so the gate
>   exercises the corrections path on every CI run.

## Strata Affected

- `kernel` (domain — correction records + corrections-aware diff), `cli`
  (adapter — `correct` subcommand, `--corrections` flag), bb gate wiring.

## Testing Plan

- Kernel: correction flips a would-be regression into accepted; correction
  raises the bar above the raw baseline (score floor 0.95 fails a passing
  0.88); status-only correction keeps the baseline score floor; stale
  correction warns without failing; duplicate keys are a load error;
  empty rationale is refused; an in-test correction against the committed
  baseline changes a diff outcome (0.41: raw regression, corrected clean).
- CLI: CLI-written file round-trips through the loader; re-correcting the
  same cell overwrites (one file); empty corrections dir refused; status
  parsing accepts only the contract vocabulary.
- Manual: `correct` wrote a scratch correction file that round-tripped
  through `diff --corrections` on a worsened copy — 0.41 exits 3 raw /
  0 corrected, 0.30 exits 3 with the `(corrected)` provenance mark;
  `bb regression-gate` green with the corrections wiring.
- `bb pre-commit` (fmt + clippy -D warnings + tests + swift build) green.

## Deployment Plan

N/A — local/CI tooling.

## Related Issues/PRs

- Builds on `2026-06-10-feat-regression-diff.md` (baseline + `diff` +
  `regression-gate`).
- Follow-up slices: registry refinement from corrections; correction
  queue in the Swift shell; `CorrectionV1` promotion to
  workbench-contract on a second consumer.

## Checklist

- [x] Standards gap analysis vs `standards/miniforge/` before push.
- [x] No new magic numbers (reuses `SCORE_REGRESSION_EPSILON`); CLI
  strings centralized in `strings.rs`.
- [x] Stratified: pure kernel judgment, fs/argv in the CLI adapter.
- [x] bb-over-shell: gate wiring stays a bb task.
- [x] PR doc (721); proprietary headers unchanged/new files headered.
- [x] Pre-commit gate green; not bypassed.
