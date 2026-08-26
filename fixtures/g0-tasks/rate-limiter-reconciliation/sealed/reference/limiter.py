"""SEALED reference solution — hierarchical fair-share admission control.

Solvability proof for N15 §6.3(1): this passes the full acceptance suite and
no LLM was involved in writing it.

Scheme. Each user is entitled to `min(per_user_rate, global_rate // active)`,
where `active` is the number of users seen this window but never fewer than
`expected_active_users`. Reserving for users who have not arrived yet is what
stops the first heavy sender in a window from consuming the whole allowance:
without it, admission order alone decides who gets served.

`expected_active_users` is optional and defaults to
`global_rate // per_user_rate`, so a two-key configuration keeps working.
"""

WINDOW_SECONDS = 1.0
MINIMUM_ENTITLEMENT = 1


class Limiter:
    """Fixed-window admission with a degrading per-user entitlement."""

    def __init__(self, config):
        self.global_rate = config["global_rate"]
        self.per_user_rate = config["per_user_rate"]
        default_expected = max(1, self.global_rate // max(1, self.per_user_rate))
        self.expected_active_users = config.get("expected_active_users", default_expected)
        self._window_start = None
        self._admitted = 0
        self._per_user = {}

    def _roll_window(self, now):
        if self._window_start is None or now - self._window_start >= WINDOW_SECONDS:
            self._window_start = now
            self._admitted = 0
            self._per_user = {}

    def _active_count(self, user_id):
        seen = len(self._per_user) + (0 if user_id in self._per_user else 1)
        return max(seen, self.expected_active_users)

    def entitlement(self, user_id):
        """The rate `user_id` is entitled to given who is active right now.

        The published per_user_rate while the ceiling has room for everyone,
        an equal share of the ceiling once it does not. Never below
        MINIMUM_ENTITLEMENT, so extreme fan-out does not lock a user out.
        """
        active = self._active_count(user_id)
        share = self.global_rate // max(1, active)
        return max(MINIMUM_ENTITLEMENT, min(self.per_user_rate, share))

    def _reserved_for_others(self, user_id, entitlement):
        """Allowance still owed to other users, seen and unseen.

        Seen users are owed the part of their entitlement they have not used.
        Unseen users — up to expected_active_users — are owed a full
        entitlement each, because they may still arrive in this window.
        """
        owed_to_seen = sum(
            max(0, entitlement - used)
            for other, used in self._per_user.items()
            if other != user_id
        )
        seen = len(self._per_user) + (0 if user_id in self._per_user else 1)
        unseen = max(0, self.expected_active_users - seen)
        return owed_to_seen + unseen * entitlement

    def allow(self, user_id, now):
        """True when the request is admitted, False when it is shed."""
        self._roll_window(now)

        # The global ceiling is absolute — checked before anything else.
        if self._admitted >= self.global_rate:
            return False

        used = self._per_user.get(user_id, 0)
        entitlement = self.entitlement(user_id)

        if used < entitlement:
            # Under entitlement with room in the window: admitting is
            # mandatory, not discretionary.
            self._admit(user_id, used)
            return True

        # Past entitlement. Borrow only what nobody else is owed, so a heavy
        # sender cannot displace a quiet tenant's floor.
        if self._admitted + self._reserved_for_others(user_id, entitlement) < self.global_rate:
            self._admit(user_id, used)
            return True
        return False

    def _admit(self, user_id, used):
        self._per_user[user_id] = used + 1
        self._admitted += 1

    def admitted_in_window(self):
        """Requests admitted in the current window. Used by the ops dashboard."""
        return self._admitted
