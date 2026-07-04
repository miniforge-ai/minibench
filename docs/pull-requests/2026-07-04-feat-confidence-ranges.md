# feat: expose confidence ranges in comparison rows

## Overview

Adds row-level confidence range fields to the kernel `ComparisonMatrix` and
renders them in the CLI and Swift matrix. This addresses the Fable review note
that status divergence should not hide evaluator uncertainty: a pass/fail split
with weak confidence should look different from the same split with strong,
uniform confidence.

## Changes in Detail

- **Kernel:** `ComparisonRow` now includes `confidence_min`, `confidence_max`,
  and `confidence_spread`, derived from present cells across variants and
  replicates.
- **CLI:** `minibench compare` prints a compact `confidence` column, either as
  one value or a min-max range.
- **Swift app:** `MatrixView` shows the same confidence range. The decoder uses
  defaults when older payloads do not include the new fields.
- **Tests:** kernel regression coverage verifies that confidence ranges are
  reported across variant cells.

## Motivation

Fable's methods review called out confidence as carried but not meaningfully
used in comparison. This slice makes confidence visible without introducing a
tenant-specific confidence policy or arbitrary pass/fail weighting.

## Base Branch

`main`.

## Standards Checklist

- [x] Strings centralized in `strings.rs` / `Strings.swift`.
- [x] Kernel remains tenant-agnostic and depends only on `workbench-contract`.
- [x] Code change includes a focused regression test.
- [x] Swift payload change is backward-compatible.
- [x] Local validation: Rust fmt, tests, clippy, and Swift build.
