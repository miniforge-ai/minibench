<!--
  Title: Minibench
  Author: Christopher Lester (christopher@miniforge.ai)
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# chore: depend on the public seam, drop the CI token

## Overview

The last thing blocking Minibench's flip to public. PR #22 relicensed the repository but left `Cargo.toml` pinning two
private repositories, so a public clone could not build. Both dependencies now come from
[`miniforge-app-foundation`](https://github.com/miniforge-ai/miniforge-app-foundation) — the extracted public seam —
and CI no longer needs a token.

## The dependency change

| Before | After |
|---|---|
| `thesium-app-foundation-contracts` — `ssh://git@github.com/miniforge-ai/thesium-app-foundation.git` | `miniforge-app-foundation-contracts` — `https://github.com/miniforge-ai/miniforge-app-foundation.git` |
| `thesium-app-foundation-data-plane` — same private repo | `miniforge-app-foundation-data-plane` — same public repo |
| `workbench-contract` — `ssh://git@github.com/miniforge-ai/workbench-contract.git` | `workbench-contract` — same public repo |

Three git dependencies across two private repositories become three crates in one public repository, pinned to a single
rev. The crates are renamed off "thesium" because the Thesium products are closed source and stay that way; a public
repository carrying their name advertises a private business line for no benefit. Import sites changed in exactly one
file, `crates/data-plane/src/lib.rs`.

Nothing extracted was proprietary: three DTOs, a 200-line router, and 432 lines of pure types, with no domain logic and
no licence enforcement. What could not be published — the career reference adapters and their scoring bands — stayed
behind in the private `workbench-contract` repository.

## `.cargo/config.toml` deleted

It existed only to set `net.git-fetch-with-cli = true` so libgit2 would use the system SSH agent for the private
fetches. With the dependencies public and fetched over HTTPS, there is nothing to authenticate and the file is dead
configuration.

## CI

The App-token step and the SSH→HTTPS rewrite are gone, along with the `MINIFORGE_CI_BOT_APPID` / `MINIFORGE_CI_BOT_KEY`
secrets they consumed. No `secrets.` reference remains in the workflow. Fork pull requests now get the same signal as
branch pull requests, which was the point of opening the repository — previously they would have failed at the
dependency fetch with a confusing auth error, since GitHub does not pass secrets to fork workflows.

`~/.cargo/git` is cacheable again. It was excluded because those checkouts' `.git/config` held the rewritten remote URL
with the ephemeral token embedded, which must not be persisted in a cache artifact. With no token, there is nothing
secret to leak, and the git dependencies stop re-fetching on every run.

## Verification

The important check is that the build no longer needs credentials, so `.cargo/config.toml` was deleted *before*
building rather than after — a stale cargo config would have masked exactly the failure this PR is meant to eliminate.

| Check | Result |
|---|---|
| `cargo build --workspace` with no `.cargo/config.toml` | resolves and compiles all three seam crates over HTTPS |
| `cargo fmt --all --check` | no drift (the rename reordered imports; applied) |
| `cargo clippy --workspace --all-targets --all-features -D warnings` | clean |
| `cargo test --workspace --all-targets` | 8 targets green |
| `swift build --package-path app` | clean |
| `bb regression-gate` | no regressions; corrected expectation applied |
| `bb validate-gate` | 11 snapshots, exit 0 |
| `grep -c thesium Cargo.lock` | 0 |

The first draft of this PR carried a caveat: the seam repository was still private, so the local fetch had used the
developer's own credentials and proved nothing about a credential-free build. CI was red for exactly that reason, and
the PR sat as a draft until it was resolved.

`miniforge-app-foundation` is now public, and the claim is verified rather than asserted:

- An anonymous clone succeeds — `GIT_TERMINAL_PROMPT=0` with global and system git config suppressed, so no credential
  helper, no `insteadOf` rewrite, and no stored token could have assisted it — and lands on
  `7ce4a41932a2ea0036d8741fe69a661c270cd6af`, the exact rev this workspace pins.
- CI passes on this branch with no token step, no `.cargo/config.toml`, and no SSH rewrite: checkout, build, test,
  regression gate, and validation gate all green.

## After this merge

Nothing in Minibench blocks the flip. Remaining, outside this repository:

1. Flip Minibench to public. (`miniforge-app-foundation` is already public.)
2. Delete the now-unused `MINIFORGE_CI_BOT_APPID` / `MINIFORGE_CI_BOT_KEY` secrets from this repository's settings if
   no other workflow uses them.
3. Relocate the career reference adapters out of the private `workbench-contract` repository into the career repository
   — where `career_adapter.clj`'s own docstring already says the production adapter belongs — and retire that repo's
   now-duplicated Rust crate.
4. Migrate `risk-data-plane` and `thesium-career/thesium-data-plane` onto the public foundation crates and retire
   `thesium-app-foundation`. Both use path dependencies into a vendored checkout or submodule rather than git revs, so
   neither is broken by this change and both can move on their own schedule.
5. Move the Clojure Malli mirror into the seam repository. It stayed put deliberately: `workbench-adapter-kit` git-deps
   the private repo's `bindings/clojure`, and copying it now would fork a live dependency.
