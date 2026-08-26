"""SEALED distractor — the literal Product reading.

Onboarding is instant and needs no administrator: joining a team writes the
membership row, and access is worked out from that row when the document is
opened. To answer Security it *renders* grant records on demand from the
current membership, so `grants_for` looks populated and every allowed access
appears to name a grant.

It is still implicit access. The records are a view of the lookup, not a
record of a decision: they exist for anyone the directory happens to list, so
a membership row that outlived its membership confers access, and nothing
recorded when the access began or ended. The acceptance suite MUST fail this.
"""


class AccessControl:
    def __init__(self, config, directory):
        self.default_visibility = config["default_visibility"]
        self.max_grants_per_document = config["max_grants_per_document"]
        self._directory = directory

    def join_team(self, user_id, team_id, now):
        self._directory.add_member(user_id, team_id)

    def leave_team(self, user_id, team_id, now):
        self._directory.remove_member(user_id, team_id)

    def can_access(self, user_id, document_id, now):
        team_id = self._directory.team_for_document(document_id)
        if team_id is None or self.default_visibility != "team":
            return False
        return self._directory.is_member(user_id, team_id)

    def grants_for(self, document_id):
        team_id = self._directory.team_for_document(document_id)
        if team_id is None or self.default_visibility != "team":
            return ()
        return tuple(
            {
                "user_id": user_id,
                "document_id": document_id,
                "team_id": team_id,
                "granted_at": None,
                "revoked_at": None,
            }
            for user_id in self._directory.members(team_id)
        )
