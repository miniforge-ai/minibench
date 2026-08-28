"""SEALED resolution tests (must-pass) — N15 §6.4 class 1.

One or more per injected contradiction, encoding the sealed intended
resolution's observable behaviour.
"""

from conftest import (
    MAX_ENTRIES,
    TTL_SECONDS,
    build,
    build_from_text,
    corpus,
    keys,
    revision,
    warm,
)

# --- Contradiction 1: never-stale versus the hit rate -----------------------


def test_a_publish_invalidates_the_document_that_was_published():
    """The written document is the one the write made wrong."""
    cache, _store = build(corpus(["doc-1"]))
    cache.read("doc-1", 0.0)
    cache.publish("doc-1", revision("doc-1", 2), 0.0)
    assert cache.read("doc-1", 0.0) == revision("doc-1", 2)


def test_a_publish_invalidates_nothing_else():
    """Invalidation is per key: everything else is still correct."""
    document_keys = keys("doc", 100)
    cache, _store = build(corpus(document_keys))
    warm(cache, document_keys)
    before = cache.store_reads()
    cache.publish(document_keys[7], revision(document_keys[7], 2), 0.0)
    for key in document_keys:
        if key != document_keys[7]:
            assert cache.read(key, 0.0) == revision(key, 1)
    assert cache.store_reads() == before, (
        "a single publish cost {} store round-trips on unrelated documents"
        .format(cache.store_reads() - before)
    )


def test_the_cold_set_survives_a_burst_of_hot_document_republishes():
    """Repeated writes to a small hot set must not clear the rest.

    This is the shape the telemetry describes: a handful of documents
    republished over and over while the bulk of the corpus sits unchanged.
    """
    cold = keys("cold", 200)
    hot = keys("hot", 5)
    cache, _store = build(corpus(cold + hot))
    warm(cache, cold + hot)
    before = cache.store_reads()
    for round_index in range(3):
        for key in hot:
            cache.publish(key, revision(key, round_index + 2), 0.0)
    for key in cold:
        assert cache.read(key, 0.0) == revision(key, 1)
    assert cache.store_reads() == before, (
        "the cold set was evicted by writes to the hot set"
    )


def test_freshness_is_not_bought_by_switching_the_cache_off():
    """Reading through to the store on every read is not a resolution."""
    document_keys = keys("doc", 20)
    cache, _store = build(corpus(document_keys))
    warm(cache, document_keys)
    before = cache.store_reads()
    for _ in range(5):
        warm(cache, document_keys)
    assert cache.store_reads() == before, (
        "repeat reads went to the store; the cache is not caching"
    )


def test_freshness_does_not_wait_for_the_ttl():
    """With a very long TTL a timer-based scheme is visibly wrong."""
    cache, _store = build(corpus(["doc-1"]), ttl_seconds=10 ** 6)
    cache.read("doc-1", 0.0)
    cache.publish("doc-1", revision("doc-1", 2), 0.0)
    assert cache.read("doc-1", 0.0) == revision("doc-1", 2)
    assert cache.read("doc-1", 500.0) == revision("doc-1", 2)


def test_both_halves_hold_at_once():
    """Neither requirement may be satisfied at the other's expense."""
    cold = keys("cold", 60)
    hot = "hot-0000"
    cache, _store = build(corpus(cold + [hot]))
    warm(cache, cold + [hot])
    before = cache.store_reads()
    cache.publish(hot, revision(hot, 2), 0.0)
    assert cache.read(hot, 0.0) == revision(hot, 2)  # never stale
    reads_for_the_written_key = cache.store_reads() - before
    for key in cold:
        assert cache.read(key, 0.0) == revision(key, 1)
    assert cache.store_reads() - before == reads_for_the_written_key  # hit rate kept


# --- Contradiction 2: ttl_seconds is no longer the freshness mechanism ------


def test_a_two_key_configuration_still_produces_correct_behaviour():
    """No new required key: the shipped two-key file must be sufficient."""
    document_keys = keys("doc", 30)
    text = "ttl_seconds: {}\nmax_entries: {}\n".format(TTL_SECONDS, MAX_ENTRIES)
    cache, _store = build_from_text(text, corpus(document_keys))
    warm(cache, document_keys)
    before = cache.store_reads()
    cache.publish(document_keys[0], revision(document_keys[0], 2), 0.0)
    assert cache.read(document_keys[0], 0.0) == revision(document_keys[0], 2)
    for key in document_keys[1:]:
        assert cache.read(key, 0.0) == revision(key, 1)
    assert cache.store_reads() - before <= 1


def test_ttl_seconds_still_expires_entries_as_a_backstop():
    """The key keeps its authority: it bounds writes the cache never saw.

    `store.put` here stands for a writer that did not go through this cache —
    a peer node, a restore, an admin edit. Invalidation cannot cover those, so
    `ttl_seconds` must still retire the entry.
    """
    cache, store = build(corpus(["doc-1"]), ttl_seconds=60)
    assert cache.read("doc-1", 0.0) == revision("doc-1", 1)
    store.put("doc-1", revision("doc-1", 2))

    before = cache.store_reads()
    assert cache.read("doc-1", 59.0) == revision("doc-1", 1)
    assert cache.store_reads() == before, (
        "an unexpired entry was refetched; ttl_seconds no longer caches"
    )

    assert cache.read("doc-1", 60.0) == revision("doc-1", 2)
    assert cache.store_reads() > before, (
        "an entry older than ttl_seconds was not refetched; the backstop is gone"
    )


def test_a_customer_tuned_ttl_is_still_honoured():
    """Customers raise ttl_seconds for slow-moving archives. It must apply."""
    cache, store = build(corpus(["doc-1"]), ttl_seconds=900)
    cache.read("doc-1", 0.0)
    store.put("doc-1", revision("doc-1", 2))
    assert cache.read("doc-1", 899.0) == revision("doc-1", 1)
    assert cache.read("doc-1", 900.0) == revision("doc-1", 2)


def test_no_configuration_key_was_renamed():
    from src.config import REQUIRED_KEYS

    assert "ttl_seconds" in REQUIRED_KEYS
    assert "max_entries" in REQUIRED_KEYS


def test_any_new_configuration_input_is_optional():
    """A dict holding only the two documented keys must construct a cache."""
    from src.cache import Cache
    from src.store import DocumentStore

    config = {"ttl_seconds": TTL_SECONDS, "max_entries": MAX_ENTRIES}
    cache = Cache(config, DocumentStore(corpus(["doc-1"])))
    cache.publish("doc-1", revision("doc-1", 2), 0.0)
    assert cache.read("doc-1", 0.0) == revision("doc-1", 2)
