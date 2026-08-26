"""SEALED acceptance-suite helpers. Never mounted in a participant capsule."""

import pytest

from src.cache import Cache
from src.config import parse_cache_config
from src.store import DocumentStore

TTL_SECONDS = 300
MAX_ENTRIES = 5000

# Derived from telemetry/publish_patterns.json.
NEVER_REPUBLISHED_FRACTION = 0.942
HOT_DOCUMENT_FRACTION = 0.008
HOT_SHARE_OF_REPUBLISHES = 0.71
MEAN_REPUBLISHES_PER_HOT_DOCUMENT_PER_HOUR = 12

# A corpus small enough to run in-process, split in the telemetry's proportions.
WORKLOAD_DOCUMENTS = 1000
WORKLOAD_MAX_ENTRIES = 2000  # larger than the corpus: eviction is not the subject
COLD_DOCUMENTS = round(WORKLOAD_DOCUMENTS * NEVER_REPUBLISHED_FRACTION)  # 942
HOT_DOCUMENTS = round(WORKLOAD_DOCUMENTS * HOT_DOCUMENT_FRACTION)  # 8
OCCASIONAL_DOCUMENTS = WORKLOAD_DOCUMENTS - COLD_DOCUMENTS - HOT_DOCUMENTS  # 50


def revision(key, number):
    """The synthetic content of `key` at revision `number`."""
    return "{} revision {}".format(key, number)


def corpus(keys, number=1):
    return {key: revision(key, number) for key in keys}


def build(documents=None, **overrides):
    """Return (cache, store) under the shipped configuration unless overridden.

    The store is returned as well because some checks need to write behind the
    cache's back, the way a peer node or a restore job would.
    """
    config = {"ttl_seconds": TTL_SECONDS, "max_entries": MAX_ENTRIES}
    config.update(overrides)
    store = DocumentStore(dict(documents or {}))
    return Cache(config, store), store


def build_from_text(text, documents=None):
    """Construct a cache straight from a configuration file's contents."""
    store = DocumentStore(dict(documents or {}))
    return Cache(parse_cache_config(text), store), store


def keys(prefix, count):
    return ["{}-{:04d}".format(prefix, index) for index in range(count)]


def warm(cache, key_list, now=0.0):
    """Read every key once so the cache holds it."""
    for key in key_list:
        cache.read(key, now)


def workload_keys():
    """The synthetic corpus, split cold / occasional / hot per the telemetry."""
    return (
        keys("cold", COLD_DOCUMENTS),
        keys("occasional", OCCASIONAL_DOCUMENTS),
        keys("hot", HOT_DOCUMENTS),
    )


def all_workload_keys():
    cold, occasional, hot = workload_keys()
    return cold + occasional + hot


def run_publish_workload(cache, rounds=5, now=0.0):
    """Read the whole corpus each round, republishing the hot set as we go.

    The mix follows `telemetry/publish_patterns.json`: 94.2% of documents are
    never republished, the 0.8% hot set carries the republishes, and one of
    the remainder is republished per round.

    Publishes are spread through the read order rather than batched before or
    after it, so the measured hit rate is a property of the invalidation
    scheme rather than of where the writes happened to land.

    Returns (reads, hits). Works against any object with the cache API, so a
    comparison scheme can be measured on the same workload.
    """
    cold, occasional, hot = workload_keys()
    every = cold + occasional + hot
    for key in every:
        cache.read(key, now)

    reads = 0
    hits = 0
    stride = len(every) // (len(hot) + 1)
    for round_index in range(rounds):
        targets = hot + [occasional[round_index % len(occasional)]]
        published = 0
        for position, key in enumerate(every):
            if position and position % stride == 0 and published < len(targets):
                target = targets[published]
                cache.publish(target, revision(target, round_index + 2), now)
                published += 1
            before = cache.store_reads()
            cache.read(key, now)
            reads += 1
            if cache.store_reads() == before:
                hits += 1
    return reads, hits


@pytest.fixture
def two_key_config_text():
    return "ttl_seconds: {}\nmax_entries: {}\n".format(TTL_SECONDS, MAX_ENTRIES)


@pytest.fixture
def parsed_two_key(two_key_config_text):
    return parse_cache_config(two_key_config_text)
