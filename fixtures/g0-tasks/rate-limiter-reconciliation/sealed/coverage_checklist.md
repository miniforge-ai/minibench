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

1. Per-user rate guarantee is represented in the admission decision, not
   only in configuration.
2. Starvation of a quiet tenant by a heavy sender is prevented.
3. The global ceiling is enforced as a hard limit.
4. Idle capacity under the ceiling is acknowledged as a cost (Platform's
   second requirement), whether or not it is fully eliminated.
5. The two documented configuration keys keep working unchanged.
6. No key is renamed and no new key is required.
7. The condition under which `per_user_rate` is not delivered is stated
   explicitly, as Support asked.

## Reconciliation work

8. The conflict between the per-user guarantee and the global ceiling is
   identified as a conflict rather than silently resolved in one side's
   favour.
9. The crossover point is derived rather than assumed.
10. The telemetry is consulted, and the active-user distribution is used to
    justify a decision.
11. At least one rejected alternative is named.

## Engineering hygiene

12. The shipped test suite still passes.
13. Time stays injected; admission stays reproducible.
14. No third-party dependency is introduced.
