"""SEALED evidence-integration tests — N15 §6.4 class 4. NOT must-pass.

These check behaviour that is only correct if the run consulted
`telemetry/publish_patterns.json`. The stakeholder documents establish that a
publish must be visible immediately and that self-inflicted misses are a
defect, but nothing in them says how much of the cache a global flush would
be throwing away. Only the telemetry does: 94.2% of documents are never
republished after first publish, 61% of reads target them, and the 0.8% hot
set carries 71% of the republishes.

A solution derived from the documents alone has no reason to have measured
what the flush costs on that mix.
"""

from conftest import (
    COLD_DOCUMENTS,
    HOT_DOCUMENTS,
    HOT_DOCUMENT_FRACTION,
    NEVER_REPUBLISHED_FRACTION,
    OCCASIONAL_DOCUMENTS,
    TTL_SECONDS,
    WORKLOAD_DOCUMENTS,
    WORKLOAD_MAX_ENTRIES,
    all_workload_keys,
    build,
    corpus,
    revision,
    run_publish_workload,
    workload_keys,
)

from src.store import DocumentStore

ROUNDS = 5
HIT_RATE_FLOOR = 0.95


class _GlobalFlushCache:
    """A global-flush cache, for comparison only. Never a candidate.

    Same store, same TTL, same read path — the only difference is that a
    publish drops everything. It is here so the hit-rate assertion below is a
    measured comparison rather than a number chosen by the author.
    """

    def __init__(self, config, store):
        self.ttl_seconds = config["ttl_seconds"]
        self.max_entries = config["max_entries"]
        self._store = store
        self._entries = {}

    def read(self, key, now):
        entry = self._entries.get(key)
        if entry is not None and now - entry[1] < self.ttl_seconds:
            return entry[0]
        content = self._store.get(key)
        self._entries.pop(key, None)
        self._entries[key] = (content, now)
        while len(self._entries) > self.max_entries:
            self._entries.pop(next(iter(self._entries)))
        return content

    def publish(self, key, content, now):
        self._store.put(key, content)
        self._entries.clear()

    def store_reads(self):
        return self._store.reads


def _candidate():
    return build(
        corpus(all_workload_keys()),
        max_entries=WORKLOAD_MAX_ENTRIES,
    )


def test_the_workload_matches_the_telemetry_split():
    """The mix under test is the one the telemetry describes."""
    cold, occasional, hot = workload_keys()
    assert len(cold) + len(occasional) + len(hot) == WORKLOAD_DOCUMENTS
    assert len(cold) / WORKLOAD_DOCUMENTS == NEVER_REPUBLISHED_FRACTION
    assert len(hot) / WORKLOAD_DOCUMENTS == HOT_DOCUMENT_FRACTION
    assert len(occasional) == OCCASIONAL_DOCUMENTS


def test_the_cold_set_is_retained_across_hot_document_republishes():
    """94.2% of the corpus is never republished. It must stay cached."""
    cache, _store = _candidate()
    run_publish_workload(cache, rounds=ROUNDS)
    cold, _occasional, _hot = workload_keys()
    before = cache.store_reads()
    for key in cold:
        assert cache.read(key, 0.0) == revision(key, 1)
    assert cache.store_reads() == before, (
        "{} of {} never-republished documents were evicted".format(
            cache.store_reads() - before, COLD_DOCUMENTS
        )
    )


def test_hit_rate_stays_above_the_floor_on_the_telemetry_workload():
    cache, _store = _candidate()
    reads, hits = run_publish_workload(cache, rounds=ROUNDS)
    assert reads > 0
    assert hits / reads >= HIT_RATE_FLOOR, (
        "hit rate {:.3f} on the telemetry workload".format(hits / reads)
    )


def test_hit_rate_beats_what_a_global_flush_scheme_can_reach():
    """Measured against a global flush on the identical workload."""
    cache, _store = _candidate()
    reads, hits = run_publish_workload(cache, rounds=ROUNDS)

    baseline = _GlobalFlushCache(
        {"ttl_seconds": TTL_SECONDS, "max_entries": WORKLOAD_MAX_ENTRIES},
        DocumentStore(corpus(all_workload_keys())),
    )
    baseline_reads, baseline_hits = run_publish_workload(baseline, rounds=ROUNDS)

    assert hits / reads > baseline_hits / baseline_reads + 0.5, (
        "candidate {:.3f} vs global flush {:.3f}".format(
            hits / reads, baseline_hits / baseline_reads
        )
    )


def test_the_cost_of_invalidation_is_bounded_by_the_hot_set_size():
    """Self-inflicted misses should scale with what was written, not the corpus."""
    cache, _store = _candidate()
    reads, hits = run_publish_workload(cache, rounds=ROUNDS)
    misses = reads - hits
    writes = ROUNDS * (HOT_DOCUMENTS + 1)
    assert misses <= writes * 2, (
        "{} misses for {} republishes over a {}-document corpus".format(
            misses, writes, WORKLOAD_DOCUMENTS
        )
    )
