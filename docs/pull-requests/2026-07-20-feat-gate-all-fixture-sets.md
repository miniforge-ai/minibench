# feat: gate every fixture set; not_applicable and min_count-0 semantics

## Overview

The validate gate (PR #18) covered `fixtures/baseline` +
`fixtures/experiments` and tracked two fixture sets it could not yet
gate: `fixtures/sample-snapshot.json` (its `pass` evaluation missed
both required ref types) and `fixtures/miniforge-etl/` (its
`not_applicable` evaluation carried zero refs against a
declared-but-zero-count requirement, which the zero-refs collapse
flagged as `missing_evidence`). This slice resolves both — one was a
fixture bug, the other a semantics bug — and extends the gate to every
committed snapshot.

## Semantics decision

Two explicit outs in `validate`, both narrow:

- **`not_applicable` is exempt from `missing_evidence`.** The gate's
  invariant is "no confident claim without evidence"; a variable that
  does not apply claims nothing, so demanding refs there pushes
  adapters toward fabricating evidence — the exact dishonesty PR #18
  removed. Refs *present* on a `not_applicable` evaluation still get
  every per-rule check. `unknown` stays strict: it means the evaluator
  failed, and that gap should surface.
- **`min_count: 0` is a waiver, not a demand** — excluded from
  `declares_requirements`, mirroring the existing `must_include_*:
  false` waiver. It does not neuter `required_refs`: a scored
  evaluation with zero refs against a declared ref type still flags.

Under these semantics the miniforge-etl registry's
`data_quality_pass_rate` declaration reads coherently: evidence is
optional overall (`min_count` 0, honest `not_applicable` when no
data-quality rules are configured), but a scored evaluation must cite a
`data-quality-report`.

## Changes in Detail

- **Kernel:** `validate_evaluation` skips the `missing_evidence`
  collapse for `not_applicable`; `declares_requirements` counts
  `min_count` only when positive. Four new tests pin the exemption, the
  still-checked-refs path, the waiver, and the waiver-vs-required_refs
  boundary.
- **Fixture:** `fixtures/sample-snapshot.json`'s
  `machine_authoritative` evaluation now cites `workflow-run` and
  `machine-snapshot` refs (plus the original `supervisory-projection`),
  all hashed — honest against the miniforge registry's
  `required_refs: ["WorkflowRun", "MachineSnapshot"]` +
  `must_include_hash`. The miniforge-etl fixtures are untouched: they
  were already honest; the validator was wrong.
- **Registries:** `fixtures/registries/miniforge-state-vars.json` —
  pinned copy of `workbench-contract/fixtures/miniforge/registry.json`.
  `fixtures/miniforge-etl/registry.json` MOVED to
  `fixtures/registries/miniforge-etl-state-vars.json` (single copy on
  the gate's `<registry_id>.json` convention; the adapter integration
  test now includes it from there).
- **Gate:** `snapshot-dirs` in tasks/validate.clj adds `fixtures`
  (top-level `*.json` — catches the sample snapshot and any future
  stray) and `fixtures/miniforge-etl/variants`. Coverage is now every
  committed snapshot in the repo.
- **Docs:** README evidence-validation-gate section documents the two
  outs and the widened coverage.

## Testing

- `cargo test` — all crates green (42 kernel tests, including the four
  new validate cases).
- `cargo clippy --all-targets` — clean; `cargo fmt` — no drift.
- `bb validate-gate` — 11 snapshots checked (1 sample + 8
  baseline/experiments + 2 ETL variants), clean, exit 0.
- Pre-fix `sample-snapshot.json` reproduces the reported violations
  under the new semantics: `missing_required_ref` ×2 (WorkflowRun,
  MachineSnapshot), exit 4 — the fixture fix, not the semantics change,
  is what clears it.
- `bb regression-gate` — no regressions vs baseline.

## Base Branch

`feat/evidence-honest-fixtures` (PR #18) — this slice extends that
gate; merge #18 first.
