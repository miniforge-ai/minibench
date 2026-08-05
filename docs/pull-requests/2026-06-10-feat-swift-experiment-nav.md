<!--
  Title: Minibench
  Author: Christopher Lester (christopher@miniforge.ai)
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# feat: SwiftUI shell slices 1–2 — comparison matrix + experiment navigation

## Overview

The Minibench shell (`app/`, Miniforge UX — it's a Miniforge product, not
Thesium). **Slice 1:** a window rendering the kernel's `ComparisonMatrix`
over HTTP from the data-plane. **Slice 2:** a two-pane shell — a sidebar
listing experiments grouped by tenant, detail rendering the selected one's
matrix — plus richer fixtures (a second experiment) and the data-plane
endpoints that group snapshots by experiment.

## Changes in Detail

- **Data-plane:** `GET /v1/experiments` (group loaded snapshots by
  `variant.experiment_id`) + `GET /v1/experiments/:id/matrix`. Two route
  integration tests.
- **Fixtures:** a second experiment — `portfolio.readiness` under two
  diverging variants (healthy vs degraded inputs) → a real 2-row matrix
  (`validation_readiness` fail/pass, `fidelity_gate` warn/pass). The single
  `portfolio-daily.json` is replaced.
- **App:** `NavigationSplitView` — `ExperimentSidebar` (grouped by product,
  selection-bound) → `MatrixView` detail; `AppStore` (`@Observable`) drives
  both panes. `MatrixView` is now a pure render of a passed-in matrix;
  `MatrixStore` removed.

Addresses the "single-row, no interaction" feedback on slice 1.

## Motivation

Proves the data path end to end: SwiftUI ← HTTP ← kernel ← real adapter
output. The headline feature (same task, N variants, divergence surfaced)
now has a native surface, not just a terminal table.

## Base Branch

`main`.

## Depends On

Nothing — builds on the merged live-feed fixtures (#1).

## Layer

- **data-plane** (application) — one added route.
- **app** (new SwiftPM target) — the shell, talking HTTP to the data-plane.

The kernel is untouched and stays domain-neutral; the Rust/Swift seam is
HTTP (the data-plane), not FFI — minibench's chosen transport. (FFI is the
`miniforge-control/app` pattern, deferred unless offline/perf demands it.)

## What This Adds

- **Rust:** `GET /v1/comparison` on the data-plane — composes the foundation's
  five routes + this one, runs `kernel::compare()` over the loaded snapshots,
  returns the `ComparisonMatrix` as JSON (`COMPARISON_ROUTE` const,
  `decoded_snapshots` helper, integration test).
- **Swift (`app/`, SwiftPM, macOS 14):** mirrors `miniforge-control/app`'s
  Miniforge UX — `DesignSystem.swift` (`Tokens`), `Strings.swift`,
  `@Observable` `MatrixStore` (URLSession fetch), `MatrixView` (a `Grid`:
  state-var rows × variant columns, pass/warn/fail cells, the ◆ divergence
  mark), `@main` window. Stratified L0/L1/L2 per `languages/swift` (240).
- **bb tasks:** `serve` (data-plane on the experiment fixtures), `build-app`,
  `run-app`; `bb pre-commit` now also runs `swift build`.

## Strata Affected

- data-plane (one route + a helper + a test).
- app (new target, 7 files).
- bb.edn (tasks), README, PR doc.

## Testing Plan

- `bb pre-commit`: cargo fmt + clippy + tests + `swift build` — green.
- `crates/data-plane/tests/serves_comparison.rs` — the route returns a
  matrix with the expected experiment id + pass/fail divergence.
- Manual: `bb serve` then `bb run-app` → the window renders the live matrix.
  (Visual pass is the operator's; CI/compile is mine.)

## Deployment Plan

N/A — local dev surface. App bundling / signing is a later slice.

## Related Issues/PRs

- minibench#1 (live-feed fixtures, merged) — supplies the experiment data.

## Checklist

- [x] Standards gap analysis vs `standards/miniforge/` before push.
- [x] Swift: stratified, `private` defaults, `guard let` (no force-unwrap),
      strings + tokens centralized.
- [x] bb-over-shell (740): app build/run are bb tasks.
- [x] PR doc (721); proprietary headers on all new files.
- [x] Pre-commit gate green; not bypassed.

## Deferred (noted, not gaps)

- Full i18n (`Localizable.strings` per locale) — `Strings.swift` is the
  extraction point.
- Experiment picker / multi-experiment sidebar, the primitive kit, and the
  per-tenant bespoke view plugins — later slices.
