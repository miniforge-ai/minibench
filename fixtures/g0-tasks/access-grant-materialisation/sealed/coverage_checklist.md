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

1. Joining a team confers access in the same action, with no administrator
   step and no propagation delay.
2. Leaving a team ends access to that team's documents.
3. Every allowed access is traceable to a record naming the user and the
   document.
4. Revocation is a recorded act with a time, not the absence of a lookup
   result.
5. What was visible in the past can be answered, not only what is visible
   now (Security's stated compliance question).
6. The two documented configuration keys keep working unchanged.
7. No key is renamed and no new key is required.
8. What happens when `max_grants_per_document` is reached is stated
   explicitly, as Support asked.

## Reconciliation work

9. The conflict between instant onboarding and auditable access is
   identified as a conflict rather than silently resolved in one side's
   favour.
10. The distinction between a recorded grant and a membership lookup is drawn
    explicitly — not merely asserted by naming a function `grants_for`.
11. The telemetry is consulted, and a specific figure from it (the staleness
    window, the 2.1% share, or the p99 team size against the shipped cap)
    justifies a decision.
12. At least one rejected alternative is named.

## Engineering hygiene

13. The shipped test suite still passes.
14. Time stays injected; access decisions stay reproducible.
15. No third-party dependency is introduced.
