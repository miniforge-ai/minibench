"""SEALED reference solution — per-key invalidation with a TTL backstop.

Solvability proof for N15 §6.3(1): this passes the full acceptance suite and
no LLM was involved in writing it.

Scheme. A publish invalidates exactly the document it published and nothing
else. Correctness for content this cache wrote comes from that invalidation,
not from the timer: once `publish` returns, the next read of that key goes to
the store. `ttl_seconds` stops being the freshness mechanism and becomes the
backstop for writes the cache never saw — an out-of-band writer, a peer node,
a restore — bounding how long such a change can go unnoticed.

The telemetry is what rules out the global flush: 94.2% of documents are
never republished after first publish, and 61% of reads target them. Flushing
the whole cache to make one hot document fresh evicts an overwhelmingly
cold-but-still-correct set, and every one of those evictions is a store
round-trip the reader pays for.
"""


class Cache:
    """Read-through cache with per-key invalidation and a TTL backstop."""

    def __init__(self, config, store):
        self.ttl_seconds = config["ttl_seconds"]
        self.max_entries = config["max_entries"]
        self._store = store
        self._entries = {}

    def _fresh(self, entry, now):
        _content, stored_at = entry
        return now - stored_at < self.ttl_seconds

    def _remember(self, key, content, now):
        """Cache `content` for `key`, evicting the oldest entry if full."""
        self._entries.pop(key, None)
        self._entries[key] = (content, now)
        while len(self._entries) > self.max_entries:
            oldest = next(iter(self._entries))
            self._entries.pop(oldest)

    def read(self, key, now):
        """Return the content for `key`, fetching it when it is not cached."""
        entry = self._entries.get(key)
        if entry is not None and self._fresh(entry, now):
            return entry[0]
        content = self._store.get(key)
        self._remember(key, content, now)
        return content

    def publish(self, key, content, now):
        """Write `content` through to the store and invalidate that key only.

        Unrelated keys are untouched: this write did not make them wrong, so
        evicting them would buy nothing and cost a round-trip each.
        """
        self._store.put(key, content)
        self._entries.pop(key, None)

    def store_reads(self):
        """Round-trips made to the document store. The ops latency proxy."""
        return self._store.reads
