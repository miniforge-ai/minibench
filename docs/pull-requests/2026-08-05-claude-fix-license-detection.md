<!--
  Title: Minibench
  Author: Christopher Lester (christopher@miniforge.ai)
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# fix: restore the Apache-2.0 appendix so GitHub detects the licence

## The symptom

With the repository public, `gh repo view --json licenseInfo` reports **`not detected`**. GitHub shows no licence badge,
and the licence is invisible to the dependency and compliance scanners that read that field. For a repository whose
purpose is to be built against by outside adapter authors, an undetectable licence reads as an unanswered question about
whether the code may be used at all.

## The cause

`LICENSE` was copied from the public `miniforge` repository, whose copy had the appendix's placeholder line replaced
with a filled-in project notice. Minibench inherited that, and PR #22 then retitled the block for Minibench rather than
removing it.

The appendix is part of the licence *template*: it instructs a reader on how to apply the licence to their own work, and
its `Copyright [yyyy] [name of copyright owner]` line is a placeholder that is meant to stay a placeholder. Substituting
real values turns four lines of template into four lines of foreign content, and GitHub's `licensee` — which normalises
and hashes the text against known licence bodies — stops matching.

Removed:

```
   Title: Minibench
   Subtitle: workbench app shell — data plane + generic kernel + CLI
   Author: Christopher Lester
   Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai)
```

Restored:

```
   Copyright [yyyy] [name of copyright owner]
```

Nothing is lost. The actual copyright attribution lives where it belongs and is unchanged: the Apache-2.0 header on all
42 source files, the `license = "Apache-2.0"` field in `Cargo.toml`, and the README's licence section.

## On the earlier review

Copilot flagged this on PR #22 and offered two options — keep the standard placeholder boilerplate, or update the
filled-in fields to match Minibench. I took the second. The first was correct, and this restores it.

## Verification

The appendix now matches a canonical Apache-2.0 text verbatim, modulo the leading indentation the rest of this file
uses (diffed with whitespace normalised against `fnv-1.0.7/LICENSE-APACHE`). The terms body was already unmodified.

Detection is a GitHub-side computation on the default branch, so it can only be confirmed after merge.

## Same defect elsewhere

`miniforge-app-foundation` has an identical block, fixed in the same way alongside this.

The public `miniforge` repository is the origin of the pattern and is also reported as `not detected`. It is outside this
repository's scope; flagged for a separate fix.
