<!--
  Title: Minibench G0 task fixture
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# Task: reconcile the Meridian document cache

**All content in this task is synthetic.** The Meridian publishing platform,
its customers, and its telemetry are invented for benchmark use.

## Situation

Meridian serves documents to readers through a read-through cache in
`src/cache.py`, backed by the document store in `src/store.py`. Three teams
have filed requirements against the cache, and the current implementation
satisfies none of them fully.

You have:

1. `stakeholders/` — the three requirement documents, as filed.
2. `telemetry/publish_patterns.json` — a production sample of how often
   documents are republished after their first publish.
3. `src/` — the service as it stands.
4. `tests/` — the suite that ships with the service.
5. `cache.yaml` — a representative customer configuration.

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
2. `cache.yaml` is customer-editable and Support runbooks reference its
   keys. Existing configurations must keep loading and keep working.
3. `tests/` must continue to pass.
4. Time is injected into `Cache.read(key, now)` and
   `Cache.publish(key, content, now)` rather than read from a clock. Keep it
   that way — cache behaviour must stay reproducible.
5. `Cache.store_reads()` reports round-trips to the document store. It is the
   latency proxy the Platform requirements are written against, so leave it
   reporting the truth.
