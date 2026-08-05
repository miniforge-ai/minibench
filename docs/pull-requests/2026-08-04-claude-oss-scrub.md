<!--
  Title: Minibench
  Author: Christopher Lester (christopher@miniforge.ai)
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# chore: relicense Apache-2.0 and scrub for open source

## Overview

Minibench is Miniforge infrastructure, not a Thesium product, so it is open source under the same terms as the rest of
Miniforge. This slice relicenses the repository and removes what a public repository must not carry. It does **not**
make the repository buildable from a fresh public clone — that is gated on the private Rust dependencies, which move in
a follow-up (see **What still blocks the flip**).

## Licence

`Proprietary` → `Apache-2.0`, per standard 810 and matching the public `miniforge` repository.

- `LICENSE` — the Apache-2.0 text, copied verbatim from `miniforge`.
- 42 source files — the `All rights reserved.` copyright line becomes the copyright line plus the standard Apache-2.0
  notice, keeping the existing Title / Subtitle / Author block. Comment prefix follows the file type (`//`, `;;`, `#`,
  and an HTML comment for Markdown).
- 13 Markdown files that carried no header at all now carry rule 810's block. Rule 810 applies only to Apache-licensed
  repositories, so it was correctly skipped before and correctly applies now.
- `Cargo.toml` — `license = "Apache-2.0"`.
- `tasks/app.clj` — the `NSHumanReadableCopyright` string baked into `Minibench.app`'s `Info.plist`.
- `CLAUDE.md` — the "Proprietary, not OSS — do NOT apply Apache-2.0 headers" convention is reversed, and says plainly
  that the Thesium products that plug in stay proprietary and that this repo must never take on their licence.

## Private references removed

- **`standards/thesium` submodule** — deleted. It pointed at a private repository, and `CLAUDE.md` tells a fresh clone
  to run `git submodule update --init --recursive`, which would have failed for anyone outside Miniforge. It carried
  `update = none` so a plain clone survived, but a public repository should not reference a repository the public
  cannot read. `AGENTS.md` no longer loads it.
- **`standards/miniforge` submodule URL** — `git@github.com:` → `https://github.com/`. The standards repository is
  public; SSH forced an authenticated fetch for no reason.
- **Registry `notes` and `owner`, career and portfolio tenants** — these named private source files, namespaces, and
  policy knobs: `lens_validation_readiness/domain.clj`, `LensEnginePolicy strong-should-floor`,
  `ai.thesium.career-growth-cli.growth-oracle-eval/evaluate`, `phase-thesium-career`. They now describe what the state
  variable measures. The Miniforge and Miniforge-ETL registries are untouched — that product is already public.

  This does not weaken the gate: `validate` compares `registry_id` / `version` / `product`, and every scored field —
  ids, thresholds, evidence requirements, gate effects — still matches upstream exactly. `notes` and `owner` are free
  text that nothing reads. The README now says so rather than claiming the copies are byte-identical duplicates.

## Open-source project files

`CONTRIBUTING.md`, `SECURITY.md`, and `CODE_OF_CONDUCT.md`. The code of conduct is the `miniforge` one, retitled.
`SECURITY.md` and `CONTRIBUTING.md` are written for this repository:

- The security scope is what Minibench actually exposes — snapshot and registry deserialization, path handling through
  `MINIBENCH_SNAPSHOT_DIR` and the CLI's directory arguments, the loopback bind, and gate bypasses. It states
  explicitly that adapter honesty is the product's responsibility, not Minibench's, since that is the boundary a
  reporter is most likely to get wrong.
- `CONTRIBUTING.md` leads with the two gates, because they are the point of the project rather than incidental CI, and
  with the one architectural rule that decides most review outcomes: the kernel reads only the contract, and anything
  needing product semantics belongs in that product's adapter.

## Honesty fixes for a public audience

- `bb regen-fixtures` needs a `thesium-workflows` checkout, which is closed source. The README now says so, and says
  what follows from it: the fixtures it produces are committed, nothing in the test suite or CI calls `regen`, and
  every gate runs against the committed fixtures with no private access.
- `docs/architecture/` points at a design document in a private repository; it now says that document is closed source
  and that nothing here depends on reading it.
- Two PR docs claimed "Proprietary headers on all new files". The files they added now carry Apache-2.0 headers, so the
  claim was stale; they record the relicensing instead of being quietly rewritten.

## What still blocks the flip

The repository is scrubbed but **not yet buildable from a public clone**. `Cargo.toml` pins two private repositories:

| Dependency | Repository | Used by |
|---|---|---|
| `thesium-app-foundation-{contracts,data-plane}` | `thesium-app-foundation` (private) | `crates/data-plane` only |
| `workbench-contract` | `workbench-contract` (private) | kernel, data-plane, CLI |

Neither contains anything proprietary. `thesium-app-foundation` is three DTOs and a five-route axum router, ~590 lines,
no domain logic and no licence enforcement. `workbench-contract`'s Rust crate is 432 lines of pure types and is
explicitly designed as the third-party adapter seam. What cannot be published is the *rest* of the
`workbench-contract` repository: `bindings/clojure/career_adapter.clj` carries the career Lens v2 scoring bands and
grounding formula, alongside `oracle_adapter.clj`, `example_lens_report.edn`, and `experiments/career_*.clj`.

The agreed resolution is a single public seam repository carrying the workbench contract types and the domain-neutral
envelope and router, named off "thesium", with the career reference adapters relocated to the career repository —
where `career_adapter.clj`'s own docstring already says the production adapter belongs. That is the follow-up; this PR
does not touch the dependency graph.

CI is unchanged for the same reason: it still mints an App token to fetch the private dependencies. Once the seam is
public, the token step and the SSH→HTTPS rewrite are deleted and fork pull requests build with a plain checkout.

## Left for a decision, not scrubbed

`fixtures/inputs/career-lens-*.edn` are synthetic inputs shaped like a career `LensReport` — `:report/lens-id`,
`:verdict/dimension`, `:verdict/level`, `:envelope/evidence-confidence`. No real data, but the *shape* is a private
product's report structure, about six keyword names' worth. Left as-is: they are the input to the flagship
comparison-matrix demo, and removing the career tenant would gut the repository's worked example. Flagged rather than
decided, because how much of the career report shape may appear publicly is a product call.

## Testing

- `cargo fmt --all --check` — no drift.
- `cargo clippy --workspace --all-targets --all-features -D warnings` — clean.
- `cargo test --workspace --all-targets` — 8 targets green.
- `swift build --package-path app` — clean.
- `bb regression-gate` — no regressions vs baseline.
- `bb validate-gate` — 11 snapshots, clean, exit 0. Run after the registry rewrite specifically to confirm the gate
  still reads the same scored fields.
- Secret sweep over the working tree and the full history (`git log --all -p`) for AWS keys, GitHub tokens, OpenAI
  keys, Slack tokens, and PEM private-key blocks — no matches.
- No `.DS_Store` or build output tracked.
