<!--
  Title: Minibench
  Author: Christopher Lester (christopher@miniforge.ai)
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# Contributing to Minibench

Minibench is the workbench app shell: it hosts, validates, and renders the cross-product state validation that product
adapters emit as `workbench_snapshot/v1`. Contributions are welcome — most usefully, new adapters in your own repo, and
fixes or kernel operations here.

## Quick start

```bash
git clone --recurse-submodules https://github.com/miniforge-ai/minibench.git
cd minibench
cargo test --workspace --all-targets
```

> **The build is not yet open.** `Cargo.toml` still pins two private repositories — `thesium-app-foundation` and
> `workbench-contract` — so `cargo` cannot fetch them without Miniforge access and the commands above will fail at the
> dependency fetch. Neither holds anything proprietary; they are being extracted into a public seam repository, after
> which a plain clone builds. Until that lands, external pull requests will fail CI at the same step. Issues and
> reviews are welcome in the meantime.

The `standards/miniforge` submodule holds the engineering rules this repo follows. It is public, so no credentials are
needed. If you cloned without `--recurse-submodules`:

```bash
git submodule update --init --recursive
```

### Prerequisites

- **Rust** — the toolchain is pinned in `rust-toolchain.toml`; `rustup` will honour it automatically.
- **Babashka** — the build, gate, and fixture tasks live in `bb.edn`, not in shell scripts. Install with
  `brew install borkdude/brew/babashka`, or see the [Babashka install guide](https://github.com/babashka/babashka#installation).
- **Swift** (macOS only) — needed to build the SwiftUI shell in `app/`. Not required to work on the Rust crates.

## The gates

Two gates decide whether a change is accepted, and both run in CI on every pull request.

```bash
bb regression-gate    # no state variable got worse than the committed baseline
bb validate-gate      # every committed snapshot satisfies its registry's evidence requirements
```

These are the point of the project rather than incidental CI: the workbench exists to say a run is *worse*, not merely
*different*, and to refuse a `pass` that cites no evidence. A change that makes a gate vacuous — skipping snapshots,
widening a threshold to fit a result — is a change to the gate's meaning. Say so explicitly in the pull request.

To accept a genuinely better result, update `fixtures/baseline/`. To accept a single reviewed cell without moving the
whole baseline, record a correction:

```bash
cargo run -p minibench-cli -- correct fixtures/corrections --help
```

## Before you open a pull request

Enable the pre-commit hook once per clone:

```bash
git config core.hooksPath .githooks
```

It runs `bb pre-commit` — `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, and `swift build`. Do not
bypass it; fix the cause.

Then:

1. **Audit your diff against `standards/miniforge/`.** Start at `standards/miniforge/index.mdc`. The ones that bite here
   are 001 stratified-design, 006 named-constants, 008 no-dead-code, 230 Rust style (user-facing text belongs in
   `strings.rs`), 740 bb-over-shell, and 810 the Apache-2.0 file header.
2. **Write the PR doc.** Every branch gets `docs/pull-requests/YYYY-MM-DD-branch-name.md` (rule 721). Record what you
   decided and why, not just what changed.
3. **Add tests with the code** (rule 716), and put the Apache-2.0 header on every new file.

## Architecture, in one rule

The kernel reads only `workbench-contract`. It never links a product's domain crate, and it must summarize a portfolio,
career, or Miniforge snapshot identically. Anything that needs product semantics belongs in that product's adapter, in
that product's repo — not here. Dependencies flow `cli` → `data-plane` → `kernel` → `workbench-contract`, downward only.

If a change to the kernel needs product-specific knowledge to make sense, that is the signal it belongs in an adapter.

## Writing an adapter

An adapter lives in your own repository and depends only on the contract types. It projects your product's data into a
`WorkbenchSnapshotV1` and writes it where Minibench can load it. See `fixtures/` for worked examples in three products,
and `docs/architecture/` for how the pieces fit.

## Licensing

Minibench is Apache-2.0. By contributing you agree that your contributions are licensed under the same terms. Put the
Apache-2.0 header on every new file — `standards/miniforge/project/header-copyright.mdc` has the exact block.

## Reporting security issues

Do not open a public issue. See [SECURITY.md](SECURITY.md).
