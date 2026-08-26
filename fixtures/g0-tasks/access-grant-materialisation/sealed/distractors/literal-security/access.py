"""SEALED distractor — the literal Security reading.

Access comes from explicit grant records and nothing else. Every allowed
access traces to a record, revocation is a recorded act at a recorded time,
and a stale membership row confers nothing. Security is fully satisfied.

The records are created by an administrator calling `grant_document`.
Joining a team therefore grants nothing at all, which is exactly the
onboarding failure Product filed against. The acceptance suite MUST fail
this.
"""


class AccessControl:
    def __init__(self, config, directory):
        self.default_visibility = config["default_visibility"]
        self.max_grants_per_document = config["max_grants_per_document"]
        self._directory = directory
        self._memberships = {}
        self._grants = {}

    # --- membership: recorded, but not a source of access -------------------

    def join_team(self, user_id, team_id, now):
        self._memberships.setdefault(team_id, set()).add(user_id)
        return ()

    def leave_team(self, user_id, team_id, now):
        self._memberships.get(team_id, set()).discard(user_id)
        revoked = []
        for records in self._grants.values():
            for record in records:
                if (
                    record["user_id"] == user_id
                    and record["team_id"] == team_id
                    and record["revoked_at"] is None
                ):
                    record["revoked_at"] = now
                    revoked.append(dict(record))
        return tuple(revoked)

    # --- the administrator step ---------------------------------------------

    def grant_document(self, user_id, document_id, now):
        """Provision one grant. Called by an administrator, out of band."""
        records = self._grants.setdefault(document_id, [])
        active = [r for r in records if r["revoked_at"] is None]
        if len(active) >= self.max_grants_per_document:
            raise ValueError("document {!r} is at its grant cap".format(document_id))
        record = {
            "user_id": user_id,
            "document_id": document_id,
            "team_id": self._directory.team_for_document(document_id),
            "granted_at": now,
            "revoked_at": None,
        }
        records.append(record)
        return dict(record)

    # --- access decision ----------------------------------------------------

    def can_access(self, user_id, document_id, now):
        for record in self._grants.get(document_id, ()):
            if record["user_id"] != user_id:
                continue
            if record["granted_at"] > now:
                continue
            if record["revoked_at"] is not None and now >= record["revoked_at"]:
                continue
            return True
        return False

    def grants_for(self, document_id):
        return tuple(dict(record) for record in self._grants.get(document_id, ()))
