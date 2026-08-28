"""Document access control for the Pinegate workspace service.

SYNTHETIC FIXTURE — this service, its customers, and its telemetry are
invented for benchmark use. Nothing here describes a real system.

The access decision is reproducible: the calling time is injected into every
method rather than read from a clock.

Grant records, where a scheme produces them, are plain dictionaries:

    {"user_id": str, "document_id": str, "team_id": str,
     "granted_at": <injected time>, "revoked_at": <injected time> or None}

`revoked_at` is None while the grant is in force.
"""


class AccessControl:
    """Access decisions for team-owned documents.

    The current implementation derives access at access time: a user can open
    a document when the directory reports them as a member of the team that
    owns it. Nothing is written down when access begins or ends.
    """

    def __init__(self, config, directory):
        self.default_visibility = config["default_visibility"]
        self.max_grants_per_document = config["max_grants_per_document"]
        self._directory = directory

    def join_team(self, user_id, team_id, now):
        """Record `user_id` as a member of `team_id`."""
        self._directory.add_member(user_id, team_id)

    def leave_team(self, user_id, team_id, now):
        """Drop `user_id` from `team_id`."""
        self._directory.remove_member(user_id, team_id)

    def can_access(self, user_id, document_id, now):
        """True when `user_id` may open `document_id`."""
        team_id = self._directory.team_for_document(document_id)
        if team_id is None:
            return False
        if self.default_visibility != "team":
            return False
        return self._directory.is_member(user_id, team_id)

    def grants_for(self, document_id):
        """Grant records covering `document_id`.

        There are none: access is derived from the directory at access time,
        so there is nothing recorded to return.
        """
        return []
