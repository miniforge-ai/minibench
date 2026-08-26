<!--
  Title: Minibench G0 task fixture
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# Task: reconcile the Pinegate document access rules

**All content in this task is synthetic.** The Pinegate workspace service,
its customers, and its telemetry are invented for benchmark use.

## Situation

Pinegate is a team collaboration tool. Documents belong to teams, and users
belong to teams. `src/access.py` decides who may open which document. Three
teams have filed requirements against it, and the current implementation
satisfies none of them fully.

You have:

1. `stakeholders/` — the three requirement documents, as filed.
2. `telemetry/access_audit.json` — a production sample of access decisions,
   collected over thirty days.
3. `src/` — the service as it stands.
4. `tests/` — the suite that ships with the service.
5. `acl.yaml` — a representative customer configuration.

## The interface

`AccessControl` keeps its shape. Time is injected, never read from a clock,
so decisions are reproducible.

```
class AccessControl:
    def __init__(self, config, directory): ...
    def can_access(self, user_id, document_id, now): ...   # bool
    def join_team(self, user_id, team_id, now): ...
    def leave_team(self, user_id, team_id, now): ...
    def grants_for(self, document_id): ...                 # grant records
```

`src/directory.py` is the cached view of the upstream identity service: team
membership and document-to-team ownership. It is refreshed asynchronously,
so it can lag.

`grants_for(document_id)` returns grant records. A grant record is a plain
dictionary:

```
{"user_id": str, "document_id": str, "team_id": str,
 "granted_at": <the injected time>, "revoked_at": <the injected time> or None}
```

`revoked_at` is None while the grant is in force. A grant is in force from
the instant it is created — there is no propagation delay to wait out.

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
2. `acl.yaml` is customer-editable and Support runbooks reference its keys.
   Existing configurations must keep loading and keep working.
3. `tests/` must continue to pass.
4. Time is injected into every `AccessControl` method rather than read from a
   clock. Keep it that way — access decisions must stay reproducible.
