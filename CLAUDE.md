# CLAUDE.md

This repository vendors the Miniforge engineering rules as a git submodule
at `standards/miniforge/`. Load them before any task.

## Entry points

1. [`standards/miniforge/index.mdc`](standards/miniforge/index.mdc) — Miniforge engineering rules (load first)
2. [`standards/miniforge/CLAUDE.md`](standards/miniforge/CLAUDE.md) — rules quick-reference
3. [README.md](README.md) — repo purpose, layout, run
4. [AGENTS.md](AGENTS.md) — product / architecture context

## What this repo is

Minibench is the workbench **app shell** — infrastructure that hosts and
renders the cross-product state validation tenants emit as
`workbench_snapshot/v1`. A Rust workspace (`kernel` / `data-plane` / `cli`),
later a SwiftUI shell. It never links a product's domain crate.

## Conventions (the ones that bite)

- **Proprietary, not OSS** — proprietary file headers (Title / Subtitle /
  Author / Copyright, all rights reserved), `license = "Proprietary"`. Do
  NOT apply Apache-2.0 headers (`project/header-copyright` is skipped here).
- **Rust** per `languages/rust` (230) + `project/rust-miniforge-shape`
  (835): edition 2024, `unsafe_code = "forbid"`, clippy `all = "deny"`,
  user-facing text in `strings.rs`, typed state, **pure kernel** (Domain
  has no I/O and never imports up).
- **Stratified** (`foundations/stratified-design`, 001): `cli` →
  `data-plane` → `kernel` → `workbench-contract`. Dependencies flow
  downward only; no layer reaches up.
- **Babashka over shell** (`workflows/bb-over-shell`, 740): build / dev /
  regen tasks live in `bb.edn`, not `scripts/*.sh`.
- **Pre-commit gate** is `bb pre-commit` (fmt + clippy + tests). Enable
  with `git config core.hooksPath .githooks`. Never bypass — fix the cause.
- **PR docs** (`workflows/pr-documentation`, 721): every feature branch gets
  `docs/pull-requests/YYYY-MM-DD-branch.md`.

## Before pushing any PR

Run a **standards gap analysis**: audit the diff against the vendored
`standards/miniforge/` rules and fix the gaps *before* push — not after a
reviewer finds them. This is a standing requirement for this repo and all
Miniforge repos.
