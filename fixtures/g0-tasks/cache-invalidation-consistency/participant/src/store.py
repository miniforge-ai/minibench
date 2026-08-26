"""Document store client for the Meridian publishing platform.

SYNTHETIC FIXTURE — this platform, its customers, and its telemetry are
invented for benchmark use. Nothing here describes a real system.

In production the store is a network service and every `get` is a round-trip.
Here it is a dict that counts those round-trips, because that count is the
latency proxy the platform requirements are written against.
"""


class DocumentStore:
    """A dict-backed stand-in for the document store.

    `reads` counts round-trips made by callers. Nothing in the task should
    reset it — the ops dashboard reads it to compute the cache hit rate.
    """

    def __init__(self, documents=None):
        self._documents = dict(documents or {})
        self.reads = 0

    def get(self, key):
        """Fetch the current content for `key`. Costs one round-trip."""
        self.reads += 1
        return self._documents.get(key)

    def put(self, key, content):
        """Write `content` for `key`. Writes are not counted as reads."""
        self._documents[key] = content
