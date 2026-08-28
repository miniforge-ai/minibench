<!--
  Title: Minibench G0 task fixture — SEALED
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# SEALED — coverage checklist

Scores `g0.run.coverage`: the fraction of these items the run addressed.
Judge-scored, judge model pinned per experiment and disjoint from every
participant model. Coverage is a diagnostic, not a gate input.

Score each item addressed / not addressed. "Addressed" means the run's
artifact or decision record demonstrably engages with the item — not that it
reached the sealed answer.

## Requirements traceable to a stakeholder document

1. A published revision is visible to the next reader without waiting for a
   timer.
2. Correctness on write is handled by the write path, not by shortening the
   staleness window.
3. The cache still serves cached documents without a store round-trip.
4. Misses caused by the service's own invalidation are treated as a cost to
   be minimised, not an unavoidable consequence.
5. The two documented configuration keys keep working unchanged.
6. No key is renamed and no new key is required.
7. `max_entries` still bounds the cache.
8. The new meaning of `ttl_seconds` is stated explicitly, as Support asked.

## Reconciliation work

9. The conflict between never-stale and the hit rate is identified as a
   conflict rather than silently resolved in one side's favour.
10. The scope of invalidation is reasoned about — what a write actually made
    wrong — rather than chosen by default.
11. The telemetry is consulted, and the republish distribution is used to
    justify a decision.
12. The residual staleness window for writes the cache never saw is
    acknowledged.
13. At least one rejected alternative is named.

## Engineering hygiene

14. The shipped test suite still passes.
15. Time stays injected; cache behaviour stays reproducible.
16. No third-party dependency is introduced.
