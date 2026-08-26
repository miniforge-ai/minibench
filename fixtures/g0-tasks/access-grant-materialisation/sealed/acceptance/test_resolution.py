"""SEALED resolution tests (must-pass) — N15 §6.4 class 1.

One or more per injected contradiction, encoding the sealed intended
resolution's observable behaviour.
"""

from conftest import (
    JOIN_TIME,
    LEAVE_TIME,
    active_records_naming,
    build,
    documents_owned_by,
    field,
    is_active,
    join_all,
    records_naming,
    users_named,
)

from src.access import AccessControl
from src.config import parse_acl
from src.directory import Directory

SMALL_CAP = 5


# --- Contradiction 1: instant onboarding vs. auditable access ---------------


def test_joining_a_team_grants_access_in_the_same_call():
    """No administrator step, no propagation delay: the same instant."""
    documents = documents_owned_by("team-a", 3)
    control, _ = build(documents=documents)
    control.join_team("u-0", "team-a", JOIN_TIME)
    for document_id in sorted(documents):
        assert control.can_access("u-0", document_id, JOIN_TIME) is True, (
            "{} is not readable at the instant of joining".format(document_id)
        )


def test_the_join_writes_a_grant_record():
    control, _ = build(documents={"doc-0": "team-a"})
    control.join_team("u-0", "team-a", JOIN_TIME)
    naming = active_records_naming(control.grants_for("doc-0"), "u-0")
    assert naming, "joining conferred access without recording a grant"
    granted_at = field(naming[0], "granted_at")
    if granted_at is not None:
        assert granted_at == JOIN_TIME, (
            "the grant is stamped {!r}, not the injected join time".format(granted_at)
        )


def test_access_does_not_depend_on_the_directory_after_the_grant():
    """The grant is the source of truth, not the membership lookup.

    Once the grant exists, losing the directory row must not remove access —
    the mirror of the property that gaining a row must not create it.
    """
    control, directory = build(documents={"doc-0": "team-a"})
    control.join_team("u-0", "team-a", JOIN_TIME)
    directory.remove_member("u-0", "team-a")
    assert control.can_access("u-0", "doc-0", JOIN_TIME + 60.0) is True


def test_leaving_a_team_ends_access():
    documents = documents_owned_by("team-a", 3)
    control, _ = build(documents=documents)
    control.join_team("u-0", "team-a", JOIN_TIME)
    control.leave_team("u-0", "team-a", LEAVE_TIME)
    for document_id in sorted(documents):
        assert control.can_access("u-0", document_id, LEAVE_TIME + 60.0) is False


def test_leaving_a_team_revokes_the_grant_record():
    """Revocation is a recorded act, not the absence of a lookup result."""
    control, _ = build(documents={"doc-0": "team-a"})
    control.join_team("u-0", "team-a", JOIN_TIME)
    control.leave_team("u-0", "team-a", LEAVE_TIME)
    records = control.grants_for("doc-0")
    assert not [r for r in records_naming(records, "u-0") if is_active(r)], (
        "an active grant survives the leave"
    )
    for record in records_naming(records, "u-0"):
        revoked_at = field(record, "revoked_at")
        if revoked_at is not None:
            assert revoked_at == LEAVE_TIME, (
                "the revocation is stamped {!r}, not the injected leave time"
                .format(revoked_at)
            )


def test_one_user_leaving_does_not_disturb_the_others():
    documents = documents_owned_by("team-a", 2)
    control, _ = build(documents=documents)
    users = users_named(4)
    for user_id in users:
        control.join_team(user_id, "team-a", JOIN_TIME)
    control.leave_team(users[1], "team-a", LEAVE_TIME)
    for user_id in users:
        expected = user_id != users[1]
        for document_id in sorted(documents):
            assert (
                control.can_access(user_id, document_id, LEAVE_TIME + 60.0) is expected
            )


# --- Contradiction 2: the cap becomes load-bearing --------------------------


def test_the_two_key_configuration_drives_the_new_scheme(shipped_acl_text):
    """No new required key: the shipped file must be sufficient on its own."""
    directory = Directory(documents=documents_owned_by("team-a", 2))
    control = AccessControl(parse_acl(shipped_acl_text), directory)
    control.join_team("u-0", "team-a", JOIN_TIME)
    assert control.can_access("u-0", "doc-0", JOIN_TIME) is True
    assert active_records_naming(control.grants_for("doc-0"), "u-0")


def test_grants_are_created_up_to_the_cap():
    """The cap must not fire early: a team that fits is fully served."""
    control, _ = build(
        documents={"doc-0": "team-a"}, max_grants_per_document=SMALL_CAP
    )
    users = users_named(SMALL_CAP)
    outcomes = join_all(control, users, "team-a", JOIN_TIME)
    assert all(outcome == "ok" for outcome in outcomes.values()), outcomes
    for user_id in users:
        assert control.can_access(user_id, "doc-0", JOIN_TIME) is True


def test_reaching_the_cap_is_surfaced_not_silent():
    """A join that returns normally must have left something behind.

    Raising is one acceptable signal; recording the refusal on the document
    is another. Returning as though the grant were made, while the user
    cannot open the document and nothing names them, is not.
    """
    control, _ = build(
        documents={"doc-0": "team-a"}, max_grants_per_document=SMALL_CAP
    )
    users = users_named(SMALL_CAP + 4)
    outcomes = join_all(control, users, "team-a", JOIN_TIME)
    records = control.grants_for("doc-0")
    for user_id in users:
        if outcomes[user_id] == "raised":
            continue
        assert records_naming(records, user_id), (
            "join_team returned normally for {} but left no record and no "
            "signal; the cap dropped the grant silently".format(user_id)
        )


def test_the_cap_is_not_ignored():
    """`max_grants_per_document` stays authoritative once grants are real.

    Either the document holds no more than the cap allows, or the overflow
    was surfaced — by raising, or by a record that names a refused user.
    """
    control, _ = build(
        documents={"doc-0": "team-a"}, max_grants_per_document=SMALL_CAP
    )
    users = users_named(SMALL_CAP + 4)
    outcomes = join_all(control, users, "team-a", JOIN_TIME)
    records = control.grants_for("doc-0")
    with_access = [u for u in users if control.can_access(u, "doc-0", JOIN_TIME)]
    if len(with_access) <= SMALL_CAP:
        return
    raised = any(outcome == "raised" for outcome in outcomes.values())
    refusals_recorded = any(
        not is_active(record)
        or not control.can_access(field(record, "user_id"), "doc-0", JOIN_TIME)
        for record in records
    )
    assert raised or refusals_recorded, (
        "{} users hold access to a document capped at {}, and nothing "
        "reported the overflow".format(len(with_access), SMALL_CAP)
    )
