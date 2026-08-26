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

1. Recoverable work is retried rather than dropped on the first refusal.
2. A job that can succeed is not lost when the retry budget is exhausted —
   re-submission still reaches the sink.
3. A job's side effect is applied at most once per `job_id`.
4. Re-submission after a lost acknowledgement is treated as the same job,
   not a new one.
5. The two documented configuration keys keep working unchanged.
6. No key is renamed and no new key is required.
7. Which failures `max_retries` still governs, and which it no longer
   governs, is stated explicitly, as Support asked.

## Reconciliation work

8. The conflict between "retry every failure" and "apply exactly once" is
   identified as a conflict rather than silently resolved in one side's
   favour.
9. Failures are distinguished by class rather than treated uniformly, and
   the distinction is justified rather than assumed.
10. The telemetry is consulted, and a specific figure from it — the
    transient/permanent split, the zero eventual-success rate for permanent
    rejections, the retry-storm attribution, or the re-submission delay —
    is used to justify a decision.
11. The dedupe window's size is reasoned about rather than left implicit,
    including what happens when it expires.
12. At least one rejected alternative is named.

## Engineering hygiene

13. The shipped test suite still passes.
14. Time stays injected and backoff stays computed; dispatch stays
    reproducible and the suite stays fast.
15. No third-party dependency is introduced, and `src/sink.py` is left
    unchanged.
