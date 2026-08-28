"""SEALED reference solution — grants materialised at membership change.

Solvability proof for N15 §6.3(1): this passes the full acceptance suite and
no LLM was involved in writing it.

Scheme. Joining a team is the act that creates access, and it creates it by
writing grant records — one per document the team owns — at the injected join
time. Leaving revokes those records at the injected leave time. `can_access`
consults only the grant store; the directory is never read at access time, so
a membership row that outlives the membership confers nothing.

That keeps Product's instant, admin-free onboarding (the grants exist before
`join_team` returns) and Security's auditable trace (every allowed access
names a record), which the seeded access-time lookup could only do one of.

`max_grants_per_document` caps the active grants a single document may carry.
A join that would breach the cap raises before writing anything, so a partial
materialisation is impossible and the caller cannot mistake a refusal for a
success.
"""


class GrantLimitExceeded(ValueError):
    """A join would push a document past `max_grants_per_document`.

    Raised instead of trimming the grant list: a grant that quietly does not
    exist is the failure Support asked us to make impossible.
    """


class AccessControl:
    """Access decided by explicit grant records, materialised on join."""

    def __init__(self, config, directory):
        self.default_visibility = config["default_visibility"]
        self.max_grants_per_document = config["max_grants_per_document"]
        self._directory = directory
        self._grants = {}

    # --- grant store --------------------------------------------------------

    def _active_records(self, document_id):
        return [
            record
            for record in self._grants.get(document_id, ())
            if record["revoked_at"] is None
        ]

    def _has_active_grant(self, user_id, document_id):
        return any(
            record["user_id"] == user_id
            for record in self._active_records(document_id)
        )

    def _effective_grant(self, user_id, document_id, now):
        """The record conferring access to `user_id` at `now`, or None.

        A record is effective from `granted_at` until `revoked_at`. Bounding
        it at both ends is what lets the store answer "who could see this
        document last March" rather than only "who can see it today".
        """
        for record in self._grants.get(document_id, ()):
            if record["user_id"] != user_id:
                continue
            if record["granted_at"] > now:
                continue
            if record["revoked_at"] is not None and now >= record["revoked_at"]:
                continue
            return record
        return None

    # --- membership changes -------------------------------------------------

    def join_team(self, user_id, team_id, now):
        """Materialise grants for every document the team owns.

        Returns the records created. Raises GrantLimitExceeded, before
        creating anything, when any target document is at its cap.
        """
        if self.default_visibility != "team":
            # Regulated workspaces grant per document, never per team.
            return ()

        targets = [
            document_id
            for document_id in self._directory.documents_for_team(team_id)
            if not self._has_active_grant(user_id, document_id)
        ]
        full = [
            document_id
            for document_id in targets
            if len(self._active_records(document_id)) >= self.max_grants_per_document
        ]
        if full:
            raise GrantLimitExceeded(
                "{} document(s) already hold max_grants_per_document={} grants; "
                "no grant was created for {!r} joining {!r}. First: {}".format(
                    len(full), self.max_grants_per_document, user_id, team_id,
                    ", ".join(full[:3]),
                )
            )

        created = []
        for document_id in targets:
            record = {
                "user_id": user_id,
                "document_id": document_id,
                "team_id": team_id,
                "granted_at": now,
                "revoked_at": None,
            }
            self._grants.setdefault(document_id, []).append(record)
            created.append(dict(record))
        return tuple(created)

    def leave_team(self, user_id, team_id, now):
        """Revoke every grant this user holds by way of `team_id`."""
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

    # --- access decision ----------------------------------------------------

    def can_access(self, user_id, document_id, now):
        """True when a grant record covers `user_id` on `document_id` at `now`.

        The directory is deliberately not consulted: a stale membership row
        must not confer access, and a fresh one must not be needed.
        """
        return self._effective_grant(user_id, document_id, now) is not None

    def grants_for(self, document_id):
        """Every grant record for `document_id`, revoked ones included.

        Revoked records are kept: the audit question is about what was
        visible then, not what is visible now.
        """
        return tuple(dict(record) for record in self._grants.get(document_id, ()))
