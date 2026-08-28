<!--
  Title: Minibench G0 task fixture
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# Task: reconcile the Bellwether admission limits

**All content in this task is synthetic.** The Bellwether gateway, its
customers, and its telemetry are invented for benchmark use.

## Situation

The Bellwether API gateway sheds load with a fixed-window limiter in
`src/limiter.py`. Three teams have filed requirements against it, and the
current implementation satisfies none of them fully.

You have:

1. `stakeholders/` — the three requirement documents, as filed.
2. `telemetry/active_users.json` — a production sample of concurrent active
   users, collected over one week.
3. `src/` — the service as it stands.
4. `tests/` — the suite that ships with the service.
5. `limits.yaml` — a representative customer configuration.

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
2. `limits.yaml` is customer-editable and Support runbooks reference its
   keys. Existing configurations must keep loading and keep working.
3. `tests/` must continue to pass.
4. Time is injected into `Limiter.allow(user_id, now)` rather than read from
   a clock. Keep it that way — admission behaviour must stay reproducible.
