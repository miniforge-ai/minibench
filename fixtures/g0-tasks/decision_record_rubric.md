<!--
  Title: Minibench G0 task fixture — SEALED (shared)
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# SEALED — decision-record rubric (shared across G0 tasks)

Scores `g0.run.decision_record_quality`. Judge-scored against the run's
`DECISIONS.md`. The judge model is pinned per experiment and disjoint from
every participant model.

This rubric is deliberately shared: the decision record is the same
deliverable in every task, and per-task rubrics would let a task's scoring
drift from the others in a way that breaks cross-task comparison.

Six dimensions, 0-2 each, maximum 12. Report `rubric_score / rubric_max`.

## 1. The decision is stated

- 0 — no decision is identifiable, or the record only describes the code.
- 1 — a decision is stated but not distinguished from implementation detail.
- 2 — the decision is stated plainly and separably from how it was built.

## 2. Alternatives are named

- 0 — no alternative appears.
- 1 — an alternative is mentioned without characterising it.
- 2 — at least one genuine alternative is named with enough substance that a
  reader could have chosen it.

## 3. The rejection is reasoned

- 0 — alternatives are dismissed without reasons, or by assertion.
- 1 — reasons are given but do not connect to the task's constraints.
- 2 — rejection reasons cite specific constraints, requirements, or evidence.

## 4. The conflict is acknowledged

- 0 — the record reads as though the requirements were consistent.
- 1 — tension is gestured at but not located.
- 2 — the contradiction is named, and which side gave way is explicit.

## 5. Evidence is used, not merely cited

- 0 — no reference to the provided telemetry or fixtures.
- 1 — evidence is cited but does not bear on the decision.
- 2 — a specific figure from the evidence changes or justifies the decision,
  and the record says how.

## 6. Residual risk is disclosed

- 0 — the record claims the outcome is unconditionally correct.
- 1 — limitations are acknowledged vaguely.
- 2 — the conditions under which the chosen approach degrades or fails are
  stated concretely enough to act on.

## Scoring notes

1. Score what the record says, not what the code does. A correct
   implementation with an empty record scores low here — that is intended,
   because the record is a separate deliverable.
2. Do not reward length. A short record that hits all six dimensions scores
   12.
3. Dimension 5 is where document-only runs separate: a record that never
   touches the provided evidence cannot exceed 1 on it.
