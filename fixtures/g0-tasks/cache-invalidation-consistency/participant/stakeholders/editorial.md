<!--
  Title: Minibench G0 task fixture — synthetic stakeholder input
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# Editorial requirements — published content is authoritative

Filed by Editorial. Synthetic document, invented for benchmark use.

## Background

When we publish a correction, the correction is the document. A reader who is
served the superseded text after we have published the replacement has been
served something we have formally retracted. Two of our publishing
agreements make corrections legally significant, and a retracted paragraph
sitting in front of readers for minutes is not something we can defend.

## Requirements

1. **A reader must never be served content older than the latest publish for
   that document.** Once `publish` returns, the previous revision is gone as
   far as readers are concerned. There is no acceptable window.
2. **Freshness must not depend on how long a reader waits.** "It will be
   correct within five minutes" is not a correction policy. If the mechanism
   that makes content fresh is a timer, we do not have a guarantee, we have
   an average.

## Note

We are aware Platform cares about the cache hit rate. Requirement 1 is not a
target we are trading against it — a scheme that serves the old revision
some of the time is the same defect at a lower frequency.
