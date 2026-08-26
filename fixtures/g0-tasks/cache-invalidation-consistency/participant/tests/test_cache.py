"""Suite shipped with the Meridian document cache. SYNTHETIC fixture.

These assertions describe behaviour that must hold before and after any
change to the cache.
"""

from src.cache import Cache
from src.config import parse_cache_config
from src.store import DocumentStore


def build(documents=None, ttl_seconds=300, max_entries=5000, **extra):
    """A cache and its store, under the shipped configuration by default."""
    config = {"ttl_seconds": ttl_seconds, "max_entries": max_entries}
    config.update(extra)
    store = DocumentStore(dict(documents or {}))
    return Cache(config, store), store


def test_parses_the_documented_keys():
    parsed = parse_cache_config("ttl_seconds: 300\nmax_entries: 5000\n")
    assert parsed["ttl_seconds"] == 300
    assert parsed["max_entries"] == 5000


def test_ignores_comments_and_blank_lines():
    parsed = parse_cache_config(
        "# heading\n\nttl_seconds: 60  # inline\nmax_entries: 100\n"
    )
    assert parsed == {"ttl_seconds": 60, "max_entries": 100}


def test_rejects_a_file_missing_required_keys():
    try:
        parse_cache_config("ttl_seconds: 300\n")
    except ValueError:
        return
    raise AssertionError("expected a ValueError for a missing required key")


def test_a_miss_fetches_from_the_store():
    cache, _store = build({"doc-1": "doc-1 revision 1"})
    assert cache.read("doc-1", 0.0) == "doc-1 revision 1"
    assert cache.store_reads() == 1


def test_a_repeat_read_is_served_from_the_cache():
    cache, _store = build({"doc-1": "doc-1 revision 1"})
    cache.read("doc-1", 0.0)
    before = cache.store_reads()
    assert cache.read("doc-1", 0.0) == "doc-1 revision 1"
    assert cache.store_reads() == before


def test_an_entry_older_than_the_ttl_is_refetched():
    cache, store = build({"doc-1": "doc-1 revision 1"}, ttl_seconds=60)
    cache.read("doc-1", 0.0)
    store.put("doc-1", "doc-1 revision 2")
    assert cache.read("doc-1", 60.0) == "doc-1 revision 2"


def test_publish_writes_through_to_the_store():
    cache, store = build({"doc-1": "doc-1 revision 1"})
    cache.publish("doc-1", "doc-1 revision 2", 0.0)
    assert store.get("doc-1") == "doc-1 revision 2"


def test_a_read_after_publish_returns_the_published_content():
    cache, _store = build({"doc-1": "doc-1 revision 1"})
    cache.read("doc-1", 0.0)
    cache.publish("doc-1", "doc-1 revision 2", 0.0)
    assert cache.read("doc-1", 0.0) == "doc-1 revision 2"


def test_the_cache_holds_at_most_max_entries():
    keys = ["doc-{:03d}".format(index) for index in range(25)]
    cache, _store = build({key: key + " revision 1" for key in keys}, max_entries=10)
    for key in keys:
        cache.read(key, 0.0)
    before = cache.store_reads()
    for key in keys:
        cache.read(key, 0.0)
    refetched = cache.store_reads() - before
    assert refetched >= len(keys) - 10, "more than max_entries entries were retained"
