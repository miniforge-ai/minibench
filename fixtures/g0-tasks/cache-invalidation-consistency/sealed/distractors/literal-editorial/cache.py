"""SEALED distractor — the literal Editorial reading.

"A reader must never be served content older than the latest publish" taken
on its own: drop everything on every write, so nothing cached can possibly
predate any publish. Never stale, and it destroys the hit rate — publishing
one hot document evicts the 94.2% of the corpus that is cold and still
correct. The acceptance suite MUST fail this.
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
        self._entries.clear()

    def store_reads(self):
        return self._store.reads
