"""Team membership and document ownership lookup.

SYNTHETIC FIXTURE — this service, its customers, and its telemetry are
invented for benchmark use. Nothing here describes a real system.

The directory is a *cached view* of the upstream identity service. It is
refreshed asynchronously, so it can lag the upstream service: a membership
row can survive for a while after the membership itself has ended. It is a
lookup table, not a record of decisions — nothing in it is authoritative.
"""


class Directory:
    """Dict-backed membership and ownership lookup.

    `teams` maps a team id to the user ids the cache currently believes are
    members. `documents` maps a document id to the team that owns it.
    """

    def __init__(self, teams=None, documents=None):
        self._teams = {team: set(members) for team, members in (teams or {}).items()}
        self._documents = dict(documents or {})

    # --- membership ---------------------------------------------------------

    def add_member(self, user_id, team_id):
        """Record `user_id` as a member of `team_id` in the cached view."""
        self._teams.setdefault(team_id, set()).add(user_id)

    def remove_member(self, user_id, team_id):
        """Drop `user_id` from `team_id` in the cached view."""
        self._teams.get(team_id, set()).discard(user_id)

    def is_member(self, user_id, team_id):
        """True when the cached view currently lists `user_id` in `team_id`."""
        return user_id in self._teams.get(team_id, set())

    def members(self, team_id):
        """Members of `team_id` in the cached view, in a stable order."""
        return tuple(sorted(self._teams.get(team_id, set())))

    def teams(self):
        """Every team id the cached view knows about, in a stable order."""
        return tuple(sorted(self._teams))

    # --- ownership ----------------------------------------------------------

    def add_document(self, document_id, team_id):
        """Record `team_id` as the owner of `document_id`."""
        self._documents[document_id] = team_id

    def team_for_document(self, document_id):
        """The team owning `document_id`, or None when nothing owns it."""
        return self._documents.get(document_id)

    def documents_for_team(self, team_id):
        """Documents owned by `team_id`, in a stable order."""
        return tuple(
            sorted(
                document_id
                for document_id, owner in self._documents.items()
                if owner == team_id
            )
        )
