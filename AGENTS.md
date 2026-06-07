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
  `thesium-app-foundation-{contracts,data-plane}` (envelope + router).
- **Never** depends on a product domain crate (`risk-core`,
  `theseus-engine`, miniforge `supervisory-state`). The kernel reads only
  the contract — that's what keeps it tenant-agnostic. A test that needs
  product data uses a `workbench-contract` fixture, not a product crate.
- New user-facing strings go in a `strings.rs` per the Rust standard, not
  inline.

## Standards

Follows the miniforge engineering standards (+ Thesium proprietary
header, since it ships with Thesium tenants). Load
`miniforge-standards/index.mdc` first, then `thesium-standards/index.mdc`.
Relevant: 006 named-constants, Rust `strings.rs` rule, 715
pre-commit-discipline, 716 tests-with-code, 810 proprietary header.
Standards submodules to be vendored under `standards/` in a follow-up.

## Adding to the kernel

Kernel ops take a `&WorkbenchSnapshotV1` (and, for diffs, a baseline) and
return plain data the shell renders. Keep them pure and tenant-agnostic;
anything that needs product semantics belongs in that product's adapter,
not here.
