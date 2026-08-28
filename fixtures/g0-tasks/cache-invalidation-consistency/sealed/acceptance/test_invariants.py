"""SEALED invariant tests (must-pass) — N15 §6.4 class 2.

One per mechanically checkable hard constraint. Any failure here is a
constraint violation and forces g0.run.success to 0.
"""

import ast
import inspect
import pathlib

from conftest import MAX_ENTRIES, TTL_SECONDS, build, corpus, keys, revision, warm

from src import cache as cache_module
from src.config import parse_cache_config

SRC = pathlib.Path(cache_module.__file__).parent
STDLIB_ALLOWED = {
    "collections",
    "dataclasses",
    "functools",
    "heapq",
    "itertools",
    "math",
        "typing",
}
CLOCK_CALLS = {
    "time.time",
    "time.monotonic",
    "time.perf_counter",
    "time.time_ns",
    "datetime.now",
    "datetime.utcnow",
}


# --- HC1: configuration contract -------------------------------------------


def test_a_two_key_file_loads(parsed_two_key):
    assert parsed_two_key["ttl_seconds"] == TTL_SECONDS
    assert parsed_two_key["max_entries"] == MAX_ENTRIES


def test_a_two_key_file_constructs_a_working_cache(parsed_two_key):
    from src.cache import Cache
    from src.store import DocumentStore

    cache = Cache(parsed_two_key, DocumentStore({"doc-1": revision("doc-1", 1)}))
    assert cache.read("doc-1", 0.0) == revision("doc-1", 1)


def test_unknown_keys_are_preserved_not_rejected():
    parsed = parse_cache_config(
        "ttl_seconds: 300\nmax_entries: 5000\nsome_future_key: 3\n"
    )
    assert parsed["some_future_key"] == 3


# --- HC2: a read after a publish is never pre-publish content ---------------


def test_a_read_after_a_publish_never_returns_the_previous_revision():
    cache, _store = build(corpus(["doc-1"]))
    cache.read("doc-1", 0.0)
    cache.publish("doc-1", revision("doc-1", 2), 0.0)
    assert cache.read("doc-1", 0.0) == revision("doc-1", 2)


def test_freshness_holds_for_a_document_that_was_never_read_first():
    """Ordering: publish before any read must be just as correct."""
    cache, _store = build(corpus(["doc-1"]))
    cache.publish("doc-1", revision("doc-1", 2), 0.0)
    assert cache.read("doc-1", 0.0) == revision("doc-1", 2)


def test_freshness_holds_across_a_run_of_publishes():
    cache, _store = build(corpus(["doc-1"]))
    for number in range(2, 12):
        cache.publish("doc-1", revision("doc-1", number), 0.0)
        assert cache.read("doc-1", 0.0) == revision("doc-1", number)
        assert cache.read("doc-1", 0.0) == revision("doc-1", number)


def test_freshness_holds_well_inside_the_ttl_window():
    """The TTL must not be what makes a published correction visible."""
    cache, _store = build(corpus(["doc-1"]), ttl_seconds=10 ** 6)
    cache.read("doc-1", 0.0)
    cache.publish("doc-1", revision("doc-1", 2), 1.0)
    assert cache.read("doc-1", 1.0) == revision("doc-1", 2)


def test_freshness_holds_for_every_document_in_a_batch():
    document_keys = keys("doc", 40)
    cache, _store = build(corpus(document_keys))
    warm(cache, document_keys)
    for key in document_keys:
        cache.publish(key, revision(key, 2), 0.0)
    for key in document_keys:
        assert cache.read(key, 0.0) == revision(key, 2)


# --- HC3: publishing one document must not evict unrelated documents -------
#
# store_reads() is the instrument the rest of this section measures with, so
# it is checked first: a counter that under-reports would make every
# no-eviction assertion below vacuous.


def test_store_reads_counts_every_round_trip():
    document_keys = keys("doc", 12)
    cache, _store = build(corpus(document_keys))
    assert cache.store_reads() == 0
    for expected, key in enumerate(document_keys, start=1):
        cache.read(key, 0.0)
        assert cache.store_reads() == expected, (
            "store_reads() under-reports: {} round-trips reported as {}".format(
                expected, cache.store_reads()
            )
        )


def test_publishing_one_document_leaves_the_others_cached():
    document_keys = keys("doc", 50)
    cache, _store = build(corpus(document_keys))
    warm(cache, document_keys)
    before = cache.store_reads()
    cache.publish(document_keys[0], revision(document_keys[0], 2), 0.0)
    for key in document_keys[1:]:
        assert cache.read(key, 0.0) == revision(key, 1)
    assert cache.store_reads() == before, (
        "publishing one document forced store round-trips for unrelated ones"
    )


def test_no_global_flush_wherever_the_written_document_sits():
    """Ordering: the victim's position in the key order must not matter."""
    document_keys = keys("doc", 30)
    for victim_index in (0, 15, 29):
        cache, _store = build(corpus(document_keys))
        warm(cache, document_keys)
        before = cache.store_reads()
        victim = document_keys[victim_index]
        cache.publish(victim, revision(victim, 2), 0.0)
        for key in document_keys:
            if key != victim:
                cache.read(key, 0.0)
        assert cache.store_reads() == before, (
            "unrelated documents were evicted when publishing at index {}".format(
                victim_index
            )
        )


def test_publishing_an_uncached_document_evicts_nothing():
    document_keys = keys("doc", 20)
    cache, _store = build(corpus(document_keys + ["fresh-doc"]))
    warm(cache, document_keys)
    before = cache.store_reads()
    cache.publish("fresh-doc", revision("fresh-doc", 1), 0.0)
    for key in document_keys:
        cache.read(key, 0.0)
    assert cache.store_reads() == before


# --- HC4: standard library only --------------------------------------------


def test_no_third_party_imports():
    for path in SRC.glob("*.py"):
        tree = ast.parse(path.read_text())
        for node in ast.walk(tree):
            roots = []
            if isinstance(node, ast.Import):
                roots = [alias.name.split(".")[0] for alias in node.names]
            elif isinstance(node, ast.ImportFrom) and node.level == 0 and node.module:
                roots = [node.module.split(".")[0]]
            for root in roots:
                assert root in STDLIB_ALLOWED or root == "src", (
                    "{} imports {!r}; the task permits the standard library only"
                    .format(path.name, root)
                )


# --- HC5: injected time -----------------------------------------------------


def test_read_and_publish_keep_their_injected_time_signatures():
    from src.cache import Cache

    read_parameters = list(inspect.signature(Cache.read).parameters)
    publish_parameters = list(inspect.signature(Cache.publish).parameters)
    assert read_parameters[:3] == ["self", "key", "now"]
    assert publish_parameters[:4] == ["self", "key", "content", "now"]


def test_the_same_inputs_produce_the_same_reads():
    document_keys = keys("doc", 30)

    def trace():
        cache, _store = build(corpus(document_keys))
        seen = []
        for round_index in range(3):
            for key in document_keys:
                seen.append(cache.read(key, 0.0))
            cache.publish(document_keys[0], revision(document_keys[0], round_index + 2), 0.0)
        return seen, cache.store_reads()

    assert trace() == trace()


# --- HC6: max_entries still bounds the cache --------------------------------


def test_max_entries_is_respected():
    document_keys = keys("doc", 25)
    cache, _store = build(corpus(document_keys), max_entries=10)
    warm(cache, document_keys)
    before = cache.store_reads()
    warm(cache, document_keys)
    refetched = cache.store_reads() - before
    assert refetched >= len(document_keys) - 10, (
        "more than max_entries entries were retained: only {} of {} refetched"
        .format(refetched, len(document_keys))
    )


def test_max_entries_of_one_still_serves_correct_content():
    document_keys = keys("doc", 5)
    cache, _store = build(corpus(document_keys), max_entries=1)
    for key in document_keys:
        assert cache.read(key, 0.0) == revision(key, 1)
    cache.publish(document_keys[0], revision(document_keys[0], 2), 0.0)
    assert cache.read(document_keys[0], 0.0) == revision(document_keys[0], 2)

def test_the_source_cannot_reach_a_clock():
    """HC: injected time. Import ban, not call-shape matching.

    Matching call shapes misses `datetime.datetime.now()` (chained
    attribute) and `from time import time; time()` (a Call on a Name). A
    module that never imports a clock cannot read one, whichever form it
    would have used.
    """
    from clockcheck import find_clock_reads

    findings = find_clock_reads(SRC)
    assert not findings, "source can reach a clock: {}".format(findings)
