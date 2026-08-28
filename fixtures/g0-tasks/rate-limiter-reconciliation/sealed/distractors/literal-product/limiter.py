"""SEALED distractor — the literal Product reading.

Implements "every user is guaranteed 10 rps" as an independent per-user
bucket. Fair, and it oversubscribes the ceiling the moment more than 80
users are active. The acceptance suite MUST fail this.
"""

WINDOW_SECONDS = 1.0


class Limiter:
    def __init__(self, config):
        self.global_rate = config["global_rate"]
        self.per_user_rate = config["per_user_rate"]
        self._window_start = None
        self._admitted = 0
        self._per_user = {}

    def _roll_window(self, now):
        if self._window_start is None or now - self._window_start >= WINDOW_SECONDS:
            self._window_start = now
            self._admitted = 0
            self._per_user = {}

    def allow(self, user_id, now):
        self._roll_window(now)
        used = self._per_user.get(user_id, 0)
        if used >= self.per_user_rate:
            return False
        self._per_user[user_id] = used + 1
        self._admitted += 1
        return True

    def admitted_in_window(self):
        return self._admitted
