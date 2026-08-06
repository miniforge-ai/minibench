<!--
  Title: Minibench
  Author: Christopher Lester (christopher@miniforge.ai)
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# fix: restore the Apache-2.0 appendix template

> **This document was rewritten after the change merged.** Its first version claimed GitHub could not detect the
> licence and that this change fixed it. That diagnosis was wrong. The change is still correct, for a smaller reason.
> What follows is the corrected account; the original is in this file's git history.

## What this change does

`LICENSE` had the Apache-2.0 appendix's placeholder line replaced with a filled-in project block:

```diff
-   Title: Minibench
-   Subtitle: workbench app shell — data plane + generic kernel + CLI
-   Author: Christopher Lester
-   Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai)
+   Copyright [yyyy] [name of copyright owner]
```

The appendix is part of the licence *template*. It instructs a reader on how to apply the licence to their own work,
and `Copyright [yyyy] [name of copyright owner]` is a placeholder that is meant to stay a placeholder — substituting
real values makes the instructions read as though they were about this project rather than about the reader's. After
this change the file matches a canonical Apache-2.0 text exactly, verified by a whitespace-normalised diff against
`fnv-1.0.7/LICENSE-APACHE`.

Nothing is lost. The real attribution is unchanged where it belongs: the Apache-2.0 header on all 42 source files,
`license = "Apache-2.0"` in `Cargo.toml`, and the README's licence section.

## The wrong diagnosis, and why it was wrong

The change was opened on the premise that GitHub reported the licence as undetected and that the modified appendix was
the cause. Both halves were false.

The evidence for "undetected" was `gh repo view --json licenseInfo`, which returns `null`. That reads GitHub's GraphQL
`licenseInfo` field. The REST equivalent, `gh api repos/OWNER/REPO -q .license.spdx_id`, returns `Apache-2.0` — and
did so before this change.

| Repository | REST `.license.spdx_id` | GraphQL `licenseInfo.spdxId` |
|---|---|---|
| `minibench` | `Apache-2.0` | `null` |
| `miniforge-app-foundation` | `Apache-2.0` | `null` |
| `miniforge` | `Apache-2.0` | `null` |

The decisive case is the third row. `miniforge` still carries the modified appendix and was never touched by this work,
and GitHub detects its licence anyway. So the appendix modification never blocked detection, and the GraphQL `null` is a
property of that field across these repositories rather than a signal about any LICENSE file.

The error was accepting a single tool's output as the measurement without first checking it against a control. Running
the same query against a repository whose licence is known-detected would have shown the instrument was at fault in
under a minute.

## Lesson for the next licence or metadata check

Use `gh api repos/OWNER/REPO -q .license.spdx_id`, not `gh repo view --json licenseInfo`. When a check reports a
problem, run it against a known-good control before acting on it.
