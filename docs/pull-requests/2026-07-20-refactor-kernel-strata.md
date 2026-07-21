<!--
  Title: Minibench
  Subtitle: PR doc — refactor/kernel-strata
  Author: Christopher Lester
  Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.
-->

# refactor: split kernel lib.rs into stratified modules

## Overview

`crates/kernel/src/lib.rs` (2,454 lines, four bounded operations plus a
1,000-line test module in one file) becomes a thin index over five
stratified modules, each carrying `// ---- Layer N` headings and passing
`stratum-lint-rs` clean. This is the Rust exemplar for the coming
`languages/rust-stratified-modules` standards rule — the Rust analog of
the Clojure Layer-heading convention (210) and Python rule 221.

## Motivation

The kernel was the canonical god-module: summary, comparison, regression
diff, and evidence validation interleaved with shared helpers in one
file. rust.mdc's thin-orchestration-index and no-god-modules guidance
already pointed here; the new linter makes the discipline checkable.

## Changes in Detail

- `lib.rs` — thin index: crate docs, module decls, `pub use` re-exports.
  Public API unchanged (verified by unchanged consumers: data-plane,
  cli, integration test).
- `snapshot.rs` — new shared vocabulary module: variant/experiment
  labels, registry ref keys, and `check_registry`, the
  snapshot-vs-registry guard.
- `summary.rs`, `compare.rs`, `regression.rs`, `evidence.rs` — one
  bounded operation each, items grouped under ascending Layer headings
  (vocabulary → assembly → public API), ≤3 layers per file.
- `tests.rs` — the in-file test module extracted verbatim (public-API
  tests only; imports adjusted).
- Duplication removed in passing: `experiment_id_of` was a literal-using
  copy of `experiment_id` (now one fn in `snapshot.rs`); the registry
  ref/product guard existed three times with three error types (now one
  `check_registry` mapped into `SummaryError` / `CompareError` /
  `ValidateError` at each boundary — error shapes unchanged).

## Testing Plan

No behavior change intended. All 48 workspace tests pass unmodified
(the 38 kernel behavior tests are the spec); fmt clean; clippy
`-D warnings` clean; `stratum-lint-rs crates/kernel/src` exits 0 with
all five modules heading-covered (index and tests files are
intentionally headingless and skipped).

## Deployment Plan

Merges to main; no consumer changes needed.

## Related Issues/PRs

- miniforge-ai/stratum-lint#3 — the Rust linter this passes.
- Next: miniforge-standards rule 236 referencing this crate as exemplar.

## Checklist

- [x] Public API byte-compatible (`pub use` re-exports, consumers untouched)
- [x] All tests green, fmt + clippy clean
- [x] stratum-lint-rs clean, coverage verified non-vacuous
- [x] Proprietary headers on all new files
