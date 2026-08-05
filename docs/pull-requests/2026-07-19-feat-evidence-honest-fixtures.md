<!--
  Title: Minibench
  Author: Christopher Lester (christopher@miniforge.ai)
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# feat: registry-honest fixtures + evidence validation gate in CI

## Overview

The evidence-validate slice (PR #16) shipped the `validate` op and CLI
but deliberately left the gate off: run against the shipped fixtures it
found real gaps — portfolio evaluations passing with zero evidence
refs, and the career grounding evaluation citing only one of the two
required ref types. The fix belonged upstream in the thesium-workflows
adapters; that fix is thesium-workflows PR #490
(career-workbench-adapter cites the portfolio claims backing each
verdict; portfolio-workbench-adapter cites the readiness record,
per-metric signal-quality records, and hashed input artifacts). This
slice closes the loop: regenerate the fixtures from the fixed adapters,
commit the registries the snapshots name, and turn the gate on in CI.

## Changes in Detail

- **Inputs:** `fixtures/inputs/career-lens-{opus,haiku}.edn` now carry
  `:verdict/claims` — the synthetic portfolio claims each verdict cites.
  The haiku variant cites one claim from both verdicts, exercising the
  adapter's claim dedupe. Portfolio inputs are unchanged; their evidence
  (readiness record, source hashes) derives from the artifacts
  themselves.
- **Fixtures:** `fixtures/{baseline,experiments}/*.json` regenerated via
  `bb regen-fixtures` against the fixed adapters. Scores and statuses
  are unchanged (opus 0.88 pass, haiku 0.43 fail, portfolio 1.0/0.75) —
  only the evidence refs are new — so the regression gate stays clean.
- **Registries:** `fixtures/registries/{career,portfolio}-state-vars.json`
  — pinned copies of the product registries from
  `workbench-contract/fixtures/`. `validate` verifies
  registry_id/version/product against each snapshot's `registry_ref`,
  so an upstream registry bump fails loudly here instead of validating
  against a stale yardstick.
- **Gate:** `bb validate-gate` (tasks/validate.clj) discovers every
  snapshot under `fixtures/baseline` + `fixtures/experiments`, pairs it
  with `fixtures/registries/<registry_id>.json` from its own
  `registry_ref`, and runs `minibench validate`. A snapshot with no
  committed registry copy is a gate failure, not a silent skip. CI runs
  it after the regression gate.
- **Regen:** `bb regen-fixtures` passes `--variant-inputs` (new CLI flag
  from thesium-workflows PR #490) — each variant's input IS that
  variant's measured artifact, so the snapshots must not carry the
  identical-inputs `source_hashes` comparability claim. Without the
  flag, regenerated snapshots hash their differing inputs and the
  kernel rejects the pair as `MixedSourceHashes` (a latent regen
  incompatibility since provenance emission landed upstream; any regen
  would have hit it).
- **Kernel tests:** the five validate tests that parametrized off the
  opus fixture's literal ref count / timestamps now derive expectations
  from the fixture (`evidence_refs.len()`, `evaluated_at`), so the next
  regen doesn't break them; the missing-required-ref case names
  `EvidenceBundle`, which no career snapshot cites.

## Scope

`fixtures/sample-snapshot.json` (hand-written miniforge stand-in; its
`pass` evaluation misses both required ref types) and
`fixtures/miniforge-etl/` (its `not_applicable` evaluation carries a
declared-but-zero-count requirement, which `validate` currently flags)
sit outside the gated directories. Gating them needs either fixture
backfill or a semantics decision on `not_applicable`/`min_count 0` in
the validator — tracked separately rather than silently skipped:
the gate prints exactly which directories it covers.

## Testing

- `bb validate-gate` — 8 snapshots checked, clean, exit 0.
- Pre-fix fixtures reproduce the reported violations under the same
  gate (career `missing_required_ref` Claim; portfolio
  `pass_without_evidence` + `missing_evidence` ×2, exit 4).
- Missing-registry path exercised: removing a registry copy fails the
  gate with a named snapshot list, exit 1.
- `bb regression-gate` — no regressions vs baseline.

## Base Branch

`fable-evidence-validate` (PR #16) — this slice needs the `validate`
CLI; merge #16 first.

🤖 Generated with [Claude Code](https://claude.com/claude-code)
