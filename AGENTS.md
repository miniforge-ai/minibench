<!--
  Title: Minibench
  Author: Christopher Lester (christopher@miniforge.ai)
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# AGENTS.md

`minibench` — the workbench app shell. Hosts (data plane), summarizes
(kernel), and renders (Swift, later) the `workbench_snapshot/v1` state
that product adapters emit.

## What this repo is (and is not)

- **Is:** the host. The foundation-riding data plane, the generic kernel,
  and (later) the Swift UI + view-plugin host.
- **Is not:** the contract (that's the `workbench-contract` repo, which
  this depends on) and **not** any product's adapter (those live in the
  product repos and emit snapshots to this host).

## Boundaries — the dependency arrow only points inward

- Consumes `workbench-contract` (typed snapshot) and
  `miniforge-app-foundation-{contracts,data-plane}` (envelope + router),
  all three from the public `miniforge-app-foundation` seam repo.
- **Never** depends on a product domain crate (`risk-core`,
  `theseus-engine`, miniforge `supervisory-state`). The kernel reads only
  the contract — that's what keeps it tenant-agnostic. A test that needs
  product data uses a `workbench-contract` fixture, not a product crate.
- New user-facing strings go in a `strings.rs` per the Rust standard, not
  inline.

## Standards

Follows the Miniforge engineering standards, vendored as a public
submodule at `standards/miniforge/`; load
`standards/miniforge/index.mdc` first. Relevant: 006 named-constants,
Rust `strings.rs` rule, 715 pre-commit-discipline, 716 tests-with-code,
810 header-copyright (the Apache-2.0 header — this repo is Apache-2.0).

Minibench is Miniforge infrastructure and open source. The Thesium
products that plug into it are proprietary and live in private repos;
nothing here may take on their license, vendor their standards, or link
their domain crates.

## Adding to the kernel

Kernel ops take a `&WorkbenchSnapshotV1` (and, for diffs, a baseline) and
return plain data the shell renders. Keep them pure and tenant-agnostic;
anything that needs product semantics belongs in that product's adapter,
not here.
