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

## Merging main — and a broken main

Merging `origin/main` surfaced that **main did not compile**. PR #20
(`refactor/kernel-strata`) split the kernel's monolithic `lib.rs` into
stratified modules but did not carry over the 296 lines PR #17
(`fable-correction-loop`) had added to that same file. `minibench-cli`
still imports `CorrectedDiffReport`, `CorrectionKey`, `CorrectionSet`,
`CorrectionV1`, and `diff_with_corrections`, and on main those resolve
to nothing (`E0432`). The two PRs were green independently and the
conflict was semantic, not textual, so the merge landed silently broken.

Restored as `crates/kernel/src/corrections.rs` in the module shape #20
established — Layer 0 (records, keys, validation) then Layer 1
(`diff_with_corrections`) — with `severity` and
`SCORE_REGRESSION_EPSILON` widened to `pub(crate)` so the module can
reuse `regression.rs`'s comparison rules rather than restate them, and
the 200 lines of correction tests returned to `tests.rs`.

`fixtures/corrections/` is a separate, still-open gap. PR #17 deliberately
shipped no correction — a committed correction is a recorded human
judgment, and none had been made — but its doc described the directory as
shipping "empty", which git cannot do. So the directory did not exist at
all, the `fs/exists?` guard in `bb regression-gate` never fired, and the
corrections path was exercised only by unit tests, never by CI.

> **Amendment (2026-08-04).** Closed by
> `2026-08-04-feat-commit-corrections-fixture.md`: a human-recorded
> correction now ships under `fixtures/corrections/`, so the guard fires
> and CI runs `diff_with_corrections` end-to-end. The paragraph above
> originally read that PR #17's doc "describes a committed example
> correction"; it does not — it states the opposite. Corrected in place
> because the follow-up work was scoped from that misreading.

### Conflict resolution

- **`bb.edn`** — keep both gates: main's corrections-aware
  `regression-gate` and this branch's `validate-gate`.
- **kernel** — take main's module layout, re-apply this branch's
  semantics into it: the `not_applicable` exemption and the positive-
  `min_count` rule land in `evidence.rs`, the `MissingEvidence` doc note
  in `violations.rs`.

### Fixture-coupled tests

Five of main's validate tests asserted against the pre-registry-honest
`opus-semantic.json`: two evidence refs, `evaluated_at`
`2026-06-10T16:07:46Z`. This branch's fixture carries four refs, and
`bb regen-fixtures` restamps `evaluated_at` with the wall clock — so
`min_count: Some(3)` was satisfied, a `"Claim"` required-ref now
matched, the per-ref violation counts were short by two, and the
freshness dates were ~40 days stale. The counts and the SLA instant now
derive from the fixture, so the next regen cannot break them.

## Testing

- `cargo test --workspace --all-targets` — 8 targets green, 49 kernel
  tests (the four new validate cases plus the restored correction ones).
- `cargo clippy --workspace --all-targets --all-features -D warnings` —
  clean; `cargo fmt` — no drift.
- `swift build --package-path app` — clean.
- `bb validate-gate` — 11 snapshots checked (1 sample + 8
  baseline/experiments + 2 ETL variants), clean, exit 0.
- Pre-fix `sample-snapshot.json` reproduces the reported violations
  under the new semantics: `missing_required_ref` ×2 (WorkflowRun,
  MachineSnapshot), exit 4 — the fixture fix, not the semantics change,
  is what clears it.
- `bb regression-gate` — no regressions vs baseline.

## Base Branch

`main`. PR #18 (`feat/evidence-honest-fixtures`), which this slice
extends, is already merged.
