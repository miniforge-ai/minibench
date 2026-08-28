"""Request admission control for the Bellwether API gateway.

SYNTHETIC FIXTURE — this service, its customers, and its telemetry are
invented for benchmark use. Nothing here describes a real system.

The limiter decides whether a request is admitted. Time is injected rather
than read from a clock so admission behaviour is reproducible.
"""

WINDOW_SECONDS = 1.0


class Limiter:
    """Fixed-window admission control.

    The current implementation counts admissions globally: every user draws
    from one shared allowance until it is exhausted.
    """

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
        """True when the request is admitted, False when it is shed."""
        self._roll_window(now)
        if self._admitted >= self.global_rate:
            return False
        self._admitted += 1
        return True

    def admitted_in_window(self):
        """Requests admitted in the current window. Used by the ops dashboard."""
        return self._admitted
