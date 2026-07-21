# feat: enforce evidence requirements — kernel validate op + CLI gate

## Overview

The registry has declared, per state variable, what an evaluation must be
able to point at to be trusted (`EvidenceRequirements`) since the contract
landed — and nothing has ever enforced it. An evaluator can report `pass`
with zero evidence refs and every downstream view renders it as settled
fact. This slice implements the declared invariant ("no high confidence
without evidence") as a kernel op, `validate(snapshot, registry) ->
ValidationReport`, and a `minibench validate` subcommand that exits
non-zero when any evaluation's evidence falls short.

## Changes in Detail

- **Kernel:** new `validate` op in the style of `summarize_with_registry` /
  `compare_with_registry`: same registry-ref and product verification, a
  `ValidateError` enum for mismatches, and a serializable
  `ValidationReport` of per-evaluation `EvidenceViolation`s. Each
  violation carries a machine-readable `EvidenceViolationKind`
  (snake_case serde, fieldless like `CompareWarning`) plus a human
  message naming the offending ref / entry / counts.
- **Rules enforced per evaluation**, all thresholds from the registry:
  `min_count`, `required_refs`, `must_include_hash`,
  `must_include_source_role`, `freshness_sla_hours`, plus two invariant
  rules: declared requirements with zero refs (`missing_evidence`) and
  `pass` status with zero refs (`pass_without_evidence`, enforced even
  when the state variable declares no requirements).
- **Explicit `min_count: 0` waives the zero-refs invariants.** An author
  writing a literal zero is stating that no evidence is legitimate for
  that variable — the common case being a variable that reads
  `not_applicable` when its source collection is empty. Treating that
  declaration as "requires evidence" would hand the author the exact
  opposite of what they wrote (found in the miniforge ETL adapter's
  `data_quality_pass_rate`, which pairs `min_count 0` with a
  `required_refs` entry). Omitting `min_count` is NOT the same and keeps
  the invariants in force; `required_refs` still constrains the
  non-empty case. Same shape as `must_include_*: Some(false)`.
- **CLI:** `minibench validate <snapshot.json|dir> <registry.json>`.
  The registry argument is required — the requirements are the yardstick.
  A directory target validates every decodable `*.json` snapshot in it,
  skipping non-snapshot files with a warning (so a registry sitting
  beside its snapshots does not abort the gate); zero decodable
  snapshots is an error, not a clean run. Violations exit with a
  dedicated code (4), distinct from the `diff` regression code (3).
- **Tests:** kernel unit tests cover each rule's violation and clean
  case plus the edges: empty requirements are vacuously clean,
  `Some(false)` `must_include_*` waives rather than requires, missing
  `created_at` under an SLA, malformed timestamps, zero-refs collapse,
  unregistered state vars, and wrong-registry rejection.
- **Dependency:** `time` (workspace-pinned, `parsing` feature only) for
  RFC 3339 parsing in the freshness check.

## Design Decisions

- **`required_refs` match against `source_role`.** The fixtures pair
  PascalCase type entries ("LensVerdict", "EvidenceBundle") with
  kebab-case ref roles ("lens-verdict", "evidence-bundle"), while
  `EvidenceRef.id` is an instance id ("career.lens-verdict.
  technical-execution.l3") no registry could name ahead of time. Entries
  therefore match refs' `source_role` with both sides reduced to
  lowercase alphanumerics, and every entry must be matched by at least
  one ref.
- **Missing `created_at` under an SLA is a violation.** A freshness SLA
  without a timestamp is unverifiable; treating it as fresh would make
  the SLA vacuous for exactly the refs most likely to be stale. Same
  strictness for non-RFC-3339 timestamps (`malformed_timestamp`).
- **Zero refs collapse to the invariant violations.** When an evaluation
  carries no refs, every per-rule check is trivially implied, so the
  report shows `missing_evidence` (and `pass_without_evidence` when
  status is `pass`) instead of per-rule noise.
- **Unregistered state vars are a violation, not a hard error** —
  consistent with `summarize`/`compare` tolerating unregistered ids, but
  validation cannot vouch for requirements it cannot look up, so the
  gate still fails.
- **Registry mismatch is a hard error**, matching how the other
  `_with_registry` ops reject a wrong registry.
- **No invented thresholds.** "High confidence" is read as `pass`
  status; the registry has no confidence cutoff and the kernel does not
  make one up.

## Motivation

Validated against the shipped fixtures, the gate finds real gaps: the
portfolio fixtures carry `pass` evaluations with zero evidence refs, and
the career fixtures satisfy only one of the two `required_refs` entries
for `career.lens.report_grounded`. Both were previously invisible.

## Base Branch

`main`.

## Standards Checklist

- [x] Strings centralized in `strings.rs`.
- [x] Kernel remains tenant-agnostic and depends only on `workbench-contract`.
- [x] No magic numbers: every threshold comes from the registry.
- [x] Code change includes violation and clean-case tests per rule.
- [x] Local validation: Rust fmt, tests, clippy, and Swift build.
