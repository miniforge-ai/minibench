<!--
  Title: Minibench G0 task fixtures
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# G0 task fixtures

The task packages Gate G0 (N15 §8.4) runs on. **Every task in this directory
is synthetic** — the systems, customers, stakeholders, and telemetry are
invented for benchmark use and describe no real product.

Each task poses the same shape of problem: three stakeholder documents that
were written independently and contradict each other, a seeded repository
that satisfies none of them, telemetry that resolves at least one conflict
the documents cannot, and a hidden acceptance suite that scores the result.

## Layout

```
<task-id>/
├── task.json            # contradictions, hard constraints, evidence, suite designation
├── participant/         # everything mounted in the run capsule
│   ├── README.md        # the brief
│   ├── stakeholders/    # the contradictory requirement documents
│   ├── telemetry/       # the evidence
│   ├── src/             # the seeded implementation
│   ├── tests/           # the suite that ships with the service
│   └── *.yaml           # the customer-editable configuration
└── sealed/              # NEVER mounted in a participant capsule
    ├── resolutions.md   # the intended resolutions
    ├── acceptance/      # the four-class hidden suite
    ├── reference/       # the author's solution (solvability proof)
    ├── distractors/     # solutions the suite must reject
    └── coverage_checklist.md
```

`decision_record_rubric.md` is shared across tasks: the decision record is
the same deliverable everywhere, and a per-task rubric would let scoring
drift in a way that breaks cross-task comparison.

## The sealed boundary

N15 §6.2 requires participants to be unable to read sealed material. The
harness mounts `participant/` only. Nothing in `sealed/` may be reachable
from a run, and the acceptance suite is executed exactly once after the run
terminates — no condition ever receives feedback from it (N15 §3). Oracle-
guided repair would hand ground truth to whichever condition loops hardest.

## The four test classes (N15 §6.4)

1. `test_resolution.py` — **must-pass.** One or more per injected
   contradiction, encoding the sealed resolution's observable behaviour.
2. `test_invariants.py` — **must-pass.** One per mechanically checkable hard
   constraint. A failure here is a constraint violation.
3. `test_regression.py` — **must-pass.** The seeded repository's own suite
   still passes, and its case count was not reduced — so a run cannot score
   success by deleting the tests it was given.
4. `test_evidence.py` — scored, not must-pass. Behaviour that is only correct
   if the run consulted the telemetry.

`g0.run.success` is 1 only when every must-pass test is green and zero hard
constraints are violated.

## Calibration

```bash
python3 fixtures/g0-tasks/validate.py            # all tasks
python3 fixtures/g0-tasks/validate.py <task-id>  # one
```

The validator proves three things per task, and a task is not eligible for
gate use until all three hold:

1. **Solvability** (N15 §6.3.1) — the author's reference solution passes the
   full suite. No LLM is involved. If the reference cannot pass, no run
   result from the task means anything.
2. **Non-triviality** — the seeded repository fails the must-pass suite. A
   task whose starting state already passes carries no signal.
3. **Distractor rejection** (N15 §6.4) — each distractor implements the
   literal reading of one stakeholder document and must fail. A suite that
   green-lights the literal reading is measuring compliance, not
   reconciliation.

The remaining calibration requirement — that baselines at both budget tiers
neither saturate nor floor (N15 §6.3.2) — needs actual runs and is recorded
per task as `calibration.discrimination`.
