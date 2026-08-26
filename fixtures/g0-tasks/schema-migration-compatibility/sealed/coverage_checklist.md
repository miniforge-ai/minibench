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

1. `region` is required of the clients that are able to send it, not merely
   present in the schema.
2. A `v1` record is accepted rather than rejected.
3. An accepted record is stored and retrievable — nothing accepted is
   quietly discarded.
4. A region that did not come from the client is identifiable as such
   (Product's second requirement), whether or not the mechanism matches.
5. The two documented configuration keys keep working unchanged.
6. No key is renamed and no new key is required.
7. What `schema_version` selects now is stated explicitly, as Support asked.
8. A stated, defensible condition for retiring `v1` is given, rather than a
   date somebody picked.

## Reconciliation work

9. The conflict between the required field and the `v1` client population is
   identified as a conflict rather than silently resolved in one side's
   favour.
10. The reason `v1` cannot be made to comply — the field does not exist in
    that build — is engaged with, not treated as a rollout delay.
11. The telemetry is consulted, and a specific figure from it justifies a
    decision.
12. At least one rejected alternative is named.

## Engineering hygiene

13. The shipped test suite still passes.
14. Nothing in `src/` reads a clock; behaviour stays reproducible.
15. No third-party dependency is introduced.
