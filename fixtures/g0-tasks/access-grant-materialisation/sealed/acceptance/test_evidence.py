"""SEALED evidence-integration tests — N15 §6.4 class 4. NOT must-pass.

These check behaviour that is only calibrated if the run consulted
`telemetry/access_audit.json`. The stakeholder documents say the directory is
a cache; only the telemetry says how far it lags (up to six hours), how often
that lag mattered (2.1% of allowed accesses, none of them recorded anywhere),
and how large a team gets (p99 of 260 members, against a shipped cap of 250).

A solution derived from the documents alone has no reason to have handled a
team larger than the cap, or a membership row six hours out of date.
"""

from conftest import (
    JOIN_TIME,
    LEAVE_TIME,
    MAX_GRANTS_PER_DOCUMENT,
    MEAN_DOCUMENTS_PER_TEAM,
    MEAN_TEAM_SIZE,
    P99_TEAM_SIZE,
    STALENESS_PERCENTILE_SECONDS,
    STALENESS_WINDOW_SECONDS,
    build,
    documents_owned_by,
    field,
    is_active,
    join_all,
    records_naming,
    users_named,
)


def test_access_is_denied_across_the_observed_staleness_window():
    """The 2.1%: left the team, still resolves as a member, still reads.

    Each offset is a percentile from the observed staleness distribution.
    """
    for offset in STALENESS_PERCENTILE_SECONDS:
        control, directory = build(documents={"doc-0": "team-a"})
        control.join_team("u-0", "team-a", JOIN_TIME)
        control.leave_team("u-0", "team-a", LEAVE_TIME)
        # The cache has not caught up with the departure yet.
        directory.add_member("u-0", "team-a")
        assert control.can_access("u-0", "doc-0", LEAVE_TIME + offset) is False, (
            "a membership row {}s stale still confers access".format(offset)
        )


def test_the_stale_read_leaves_an_auditable_trail():
    """The compliance question, asked of the 88200 unrecorded accesses.

    What could this user see, and until when? A store that forgets a grant
    the moment it is revoked answers the second half and loses the first.
    """
    control, directory = build(documents={"doc-0": "team-a"})
    control.join_team("u-0", "team-a", JOIN_TIME)
    control.leave_team("u-0", "team-a", LEAVE_TIME)
    directory.add_member("u-0", "team-a")

    records = records_naming(control.grants_for("doc-0"), "u-0")
    assert records, "nothing records that u-0 could once open doc-0"
    assert not any(is_active(record) for record in records)
    stamps = [field(record, "revoked_at") for record in records]
    assert LEAVE_TIME in stamps, (
        "no record says when the access ended; stamps were {}".format(stamps)
    )


def test_a_p99_team_does_not_silently_exceed_the_shipped_cap():
    """260 members — the observed p99 — against the shipped cap of 250."""
    control, _ = build(documents={"doc-0": "team-a"})
    users = users_named(P99_TEAM_SIZE)
    outcomes = join_all(control, users, "team-a", JOIN_TIME)
    records = control.grants_for("doc-0")
    with_access = [u for u in users if control.can_access(u, "doc-0", JOIN_TIME)]

    for user_id in users:
        if outcomes[user_id] == "raised":
            continue
        assert records_naming(records, user_id), (
            "{} joined without error and without a record".format(user_id)
        )

    if len(with_access) > MAX_GRANTS_PER_DOCUMENT:
        raised = any(outcome == "raised" for outcome in outcomes.values())
        refusals_recorded = any(
            not is_active(record)
            or not control.can_access(field(record, "user_id"), "doc-0", JOIN_TIME)
            for record in records
        )
        assert raised or refusals_recorded, (
            "a p99 team overran the shipped cap of {} unreported".format(
                MAX_GRANTS_PER_DOCUMENT
            )
        )


def test_a_mean_sized_team_over_a_mean_document_count_is_fully_served():
    """14 members, 340 documents — the observed means. Well under the cap."""
    documents = documents_owned_by("team-a", MEAN_DOCUMENTS_PER_TEAM)
    control, _ = build(documents=documents)
    users = users_named(MEAN_TEAM_SIZE)
    outcomes = join_all(control, users, "team-a", JOIN_TIME)
    assert all(outcome == "ok" for outcome in outcomes.values()), (
        "the cap fired on a team of the observed mean size"
    )
    sampled = sorted(documents)[:: max(1, MEAN_DOCUMENTS_PER_TEAM // 20)]
    for document_id in sampled:
        for user_id in users:
            assert control.can_access(user_id, document_id, JOIN_TIME) is True
        active = [r for r in control.grants_for(document_id) if is_active(r)]
        assert len(active) <= MAX_GRANTS_PER_DOCUMENT


def test_revocation_scales_to_a_mean_sized_document_set():
    """One departure must clear every document the team owns, not the first."""
    documents = documents_owned_by("team-a", MEAN_DOCUMENTS_PER_TEAM)
    control, directory = build(documents=documents)
    users = users_named(MEAN_TEAM_SIZE)
    join_all(control, users, "team-a", JOIN_TIME)
    control.leave_team(users[0], "team-a", LEAVE_TIME)
    directory.add_member(users[0], "team-a")
    later = LEAVE_TIME + STALENESS_WINDOW_SECONDS
    for document_id in sorted(documents):
        assert control.can_access(users[0], document_id, later) is False, (
            "{} survived the departure".format(document_id)
        )
        assert control.can_access(users[1], document_id, later) is True
