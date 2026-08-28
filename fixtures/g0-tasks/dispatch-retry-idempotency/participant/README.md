<!--
  Title: Minibench G0 task fixture
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# Task: reconcile the Stonecrop dispatcher's retry behaviour

**All content in this task is synthetic.** The Stonecrop workflow tool, its
customers, and its telemetry are invented for benchmark use.

## Situation

Stonecrop is an internal workflow tool. Its background job dispatcher in
`src/dispatcher.py` hands each job to a downstream sink, which applies the
job's side effect. Three teams have filed requirements against the
dispatcher, and the current implementation satisfies none of them fully.

You have:

1. `stakeholders/` — the three requirement documents, as filed.
2. `telemetry/failure_classes.json` — a production sample of dispatch
   failures, classified, collected over thirty days.
3. `src/` — the service as it stands. `src/sink.py` is the downstream
   dependency and its error types; it is given, not yours to change.
4. `tests/` — the suite that ships with the service.
5. `dispatch.yaml` — a representative customer configuration.

## What to deliver

1. A change to `src/` that satisfies the stakeholder requirements.
2. A decision record at `DECISIONS.md` in the repository root stating what
   you decided, which alternatives you rejected, and why.

The stakeholder documents were written independently and have not been
reconciled with each other. Where they conflict, part of the task is
determining what the intended behaviour actually is — the telemetry is
there because at least one conflict cannot be resolved from the documents
alone.

## Rules

1. The Python standard library only. No new third-party dependencies.
2. `dispatch.yaml` is customer-editable and Support runbooks reference its
   keys. Existing configurations must keep loading and keep working.
3. `tests/` must continue to pass.
4. Time is injected into `Dispatcher.submit(job_id, payload, now)` rather
   than read from a clock, and retry backoff is computed into the returned
   completion time rather than slept. Keep it that way — dispatch behaviour
   must stay reproducible and the suite must stay fast.
5. The dispatcher API keeps its shape: `submit(job_id, payload, now)`
   returns a result record, and `effects()` reports the side effects the
   sink actually applied.
