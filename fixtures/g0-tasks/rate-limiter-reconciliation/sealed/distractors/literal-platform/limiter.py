"""SEALED distractor — the literal Platform reading.

Holds the ceiling absolutely and conserves work perfectly: first come,
first served until the allowance is gone. No per-user floor at all, so a
heavy sender starves everyone. The acceptance suite MUST fail this.
"""

WINDOW_SECONDS = 1.0


class Limiter:
    def __init__(self, config):
        self.global_rate = config["global_rate"]
        self.per_user_rate = config["per_user_rate"]
        self._window_start = None
        self._admitted = 0

    def _roll_window(self, now):
        if self._window_start is None or now - self._window_start >= WINDOW_SECONDS:
            self._window_start = now
            self._admitted = 0

    def allow(self, user_id, now):
        self._roll_window(now)
        if self._admitted >= self.global_rate:
            return False
        self._admitted += 1
        return True

    def admitted_in_window(self):
        return self._admitted
