"""Read-through document cache for the Meridian publishing platform.

SYNTHETIC FIXTURE — this platform, its customers, and its telemetry are
invented for benchmark use. Nothing here describes a real system.

The cache sits in front of the document store. Time is injected rather than
read from a clock so cache behaviour is reproducible.
"""


class Cache:
    """Read-through cache with a time-to-live.

    The current implementation drops every cached entry whenever anything is
    published: the simplest way to be certain nobody is served superseded
    content.
    """

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
        """Write `content` through to the store."""
        self._store.put(key, content)
        self._entries.clear()

    def store_reads(self):
        """Round-trips made to the document store. The ops latency proxy."""
        return self._store.reads
