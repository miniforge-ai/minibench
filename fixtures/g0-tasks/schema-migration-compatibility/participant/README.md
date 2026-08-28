<!--
  Title: Minibench G0 task fixture
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# Task: land the `region` field on the Quillon ingest endpoint

**All content in this task is synthetic.** The Quillon telemetry product,
its customers, its client releases, and its telemetry are invented for
benchmark use.

## Situation

Quillon ingests telemetry records from a fleet of customer-installed
clients. A `region` field was added to the record schema this quarter, and
`src/ingest.py` was changed to require it. Three teams have filed
requirements against that change, and the current implementation satisfies
none of them fully.

You have:

1. `stakeholders/` — the three requirement documents, as filed.
2. `telemetry/client_versions.json` — a production sample of which client
   versions records arrive from, collected over 30 days.
3. `src/` — the endpoint as it stands.
4. `tests/` — the suite that ships with the endpoint.
5. `ingest.yaml` — a representative customer configuration.

## The endpoint

```
class Ingestor:
    def __init__(self, config): ...
    def ingest(self, record, client_version): ...
    def stored(self): ...
```

`ingest` returns the stored record, or raises `ValidationError` from
`src/errors.py`. `stored()` returns the records the endpoint has accepted.
`client_version` is the version string of the client that sent the record —
`"v1"`, `"v2"`, or `"v3"`.

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
2. `ingest.yaml` is customer-editable and Support runbooks reference its
   keys. Existing configurations must keep loading and keep working.
3. `tests/` must continue to pass.
4. `Ingestor.ingest(record, client_version)` keeps its signature. The
   client version is passed in rather than inferred.
5. No wall-clock reads. Nothing in `src/` may consult the current date or
   time — ingestion behaviour must be reproducible from its inputs alone,
   and anything that changes over time must be driven by a value passed in.
