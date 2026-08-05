<!--
  Title: Minibench
  Author: Christopher Lester (christopher@miniforge.ai)
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# CLAUDE.md

This repository vendors the Miniforge engineering rules as a git submodule
at `standards/miniforge/`. Load them before any task. A fresh clone needs
`git submodule update --init --recursive` for that directory to exist; the
submodule is public, so this works without credentials.

## Entry points

1. [`standards/miniforge/index.mdc`](standards/miniforge/index.mdc) — Miniforge engineering rules (load first)
2. [`standards/miniforge/CLAUDE.md`](standards/miniforge/CLAUDE.md) — rules quick-reference
3. [README.md](README.md) — repo purpose, layout, run
4. [AGENTS.md](AGENTS.md) — product / architecture context

## What this repo is

Minibench is the workbench **app shell** — infrastructure that hosts and
renders the cross-product state validation tenants emit as
`workbench_snapshot/v1`. A Rust workspace (`kernel` / `data-plane` / `cli`)
plus a SwiftUI shell (`app/`, Miniforge UX — minibench is a Miniforge
product, not Thesium). It never links a product's domain crate.

## Conventions (the ones that bite)

- **Apache-2.0, open source** — every file carries the Apache-2.0 header
  (Title / Subtitle / Author / Copyright, then the standard notice), and
  `license = "Apache-2.0"`. `project/header-copyright` (810) applies here;
  it did not while this repo was proprietary. Minibench is Miniforge
  infrastructure, not a Thesium product — the Thesium apps stay
  proprietary and this repo must never take on their license or link
  their domain crates.
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
