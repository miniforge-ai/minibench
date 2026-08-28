"""SEALED distractor — the literal Platform reading.

"Cache misses caused by our own invalidation are a defect" taken on its own:
never invalidate at all. Writes go through to the store and the cached copy
is left alone until `ttl_seconds` retires it. Excellent hit rate, and a
reader can be served a retracted revision for the whole TTL window. The
acceptance suite MUST fail this.
"""


class Cache:
    def __init__(self, config, store):
        self.ttl_seconds = config["ttl_seconds"]
        self.max_entries = config["max_entries"]
        self._store = store
        self._entries = {}

    def _fresh(self, entry, now):
        _content, stored_at = entry
        return now - stored_at < self.ttl_seconds

    def _remember(self, key, content, now):
        self._entries.pop(key, None)
        self._entries[key] = (content, now)
        while len(self._entries) > self.max_entries:
            self._entries.pop(next(iter(self._entries)))

    def read(self, key, now):
        entry = self._entries.get(key)
        if entry is not None and self._fresh(entry, now):
            return entry[0]
        content = self._store.get(key)
        self._remember(key, content, now)
        return content

    def publish(self, key, content, now):
        self._store.put(key, content)

    def store_reads(self):
        return self._store.reads
