# feat: flag registry-meaningful score spread

## Overview

Adds registry-aware meaningful score spread detection to the comparison matrix.
Rows can now distinguish "statuses match and spread is small" from "statuses
match, but the score spread is large enough to cross the registry's own
threshold-band width."

## Changes in Detail

- **Kernel:** adds `ComparisonRow::meaningful_score_spread` and
  `compare_with_registry(snapshots, registry)`.
- **Threshold logic:** derives the meaningful spread floor from adjacent
  finite registry threshold cutoffs for the specific state variable.
- **CLI:** `minibench compare <dir> [registry.json]` loads an optional registry
  and prints a `meaningful` marker column.
- **Swift app:** decodes `meaningfulScoreSpread` with a default of `false` and
  renders a `meaningful` signal badge when present.
- **Tests:** covers no-registry behavior, registry-aware same-status spread,
  and registry mismatch rejection.

## Motivation

Fable called out that same-status rows with large score spread were invisible.
The registry already defines status-band cutoffs, so this slice uses that
existing contract data rather than inventing a tenant-specific threshold in
the minibench kernel.

## Base Branch

`main`.

## Standards Checklist

- [x] New strings are centralized in `strings.rs`, `Strings.swift`, and docs.
- [x] Kernel remains tenant-agnostic and consumes only `workbench-contract`.
- [x] Registry mismatch is rejected rather than silently ignored.
- [x] Code change includes focused tests.
- [x] Swift payload change is backward-compatible.
- [x] Local validation: Rust fmt, tests, clippy, and Swift build.
