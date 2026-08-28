"""SEALED invariant tests (must-pass) — N15 §6.4 class 2.

One or more per mechanically checkable hard constraint. Any failure here is a
constraint violation and forces g0.run.success to 0.
"""

import ast
import inspect
import pathlib

from conftest import (
    DEFAULT_VISIBILITY,
    MAX_GRANTS_PER_DOCUMENT,
    JOIN_TIME,
    LEAVE_TIME,
    active_records_naming,
    build,
    documents_owned_by,
    users_named,
)

from src import access as access_module
from src.access import AccessControl
from src.config import parse_acl

SRC = pathlib.Path(access_module.__file__).parent
STDLIB_ALLOWED = {
    "collections",
    "dataclasses",
    "itertools",
    "math",
        "typing",
}


# --- HC1: configuration contract -------------------------------------------


def test_a_two_key_file_loads(parsed_shipped_acl):
    assert parsed_shipped_acl["default_visibility"] == DEFAULT_VISIBILITY
    assert parsed_shipped_acl["max_grants_per_document"] == MAX_GRANTS_PER_DOCUMENT


def test_a_two_key_file_constructs_a_working_control(parsed_shipped_acl):
    """The config contract only: it builds, it decides, it does not raise.

    What the two-key file must *drive* is a resolution question, checked in
    test_resolution.py. Kept apart so a run that keeps the contract and gets
    the scheme wrong is not reported as a constraint violation.
    """
    from src.directory import Directory

    directory = Directory(documents={"doc-0": "team-a"})
    control = AccessControl(parsed_shipped_acl, directory)
    assert control.can_access("u-0", "doc-0", JOIN_TIME) is False
    assert list(control.grants_for("doc-0")) == []
    control.join_team("u-0", "team-a", JOIN_TIME)
    control.leave_team("u-0", "team-a", LEAVE_TIME)
    assert control.can_access("u-0", "doc-0", LEAVE_TIME + 60.0) is False


def test_unknown_keys_are_preserved_not_rejected():
    parsed = parse_acl(
        "default_visibility: team\nmax_grants_per_document: 250\nsome_future_key: 3\n"
    )
    assert parsed["some_future_key"] == 3


# --- HC2: every allowed access traces to a grant record --------------------


def test_every_allowed_access_names_the_user_in_a_grant_record():
    documents = {"doc-a0": "team-a", "doc-a1": "team-a", "doc-b0": "team-b"}
    control, directory = build(documents=documents)
    for user_id in ("u-0", "u-1"):
        control.join_team(user_id, "team-a", JOIN_TIME)
    # Known to the directory, but never joined through the service.
    directory.add_member("u-2", "team-b")

    for user_id in ("u-0", "u-1", "u-2", "u-3"):
        for document_id in sorted(documents):
            if not control.can_access(user_id, document_id, LEAVE_TIME):
                continue
            naming = active_records_naming(control.grants_for(document_id), user_id)
            assert naming, (
                "{} may open {} but no grant record names them; the access "
                "does not trace to anything".format(user_id, document_id)
            )


def test_an_empty_grant_store_allows_nothing():
    control, directory = build(documents=documents_owned_by("team-a", 3))
    for user_id in users_named(4):
        directory.add_member(user_id, "team-a")
    for document_id in sorted(documents_owned_by("team-a", 3)):
        assert not control.grants_for(document_id)
        for user_id in users_named(4):
            assert control.can_access(user_id, document_id, JOIN_TIME) is False


# --- HC3: no implicit access ------------------------------------------------


def test_directory_membership_alone_does_not_confer_access():
    """The core security property: a lookup is not a grant.

    The directory is a cache. If it can confer access on its own then a row
    that outlived its membership confers access too, and nothing recorded
    that it did.
    """
    control, directory = build(documents={"doc-0": "team-a"})
    directory.add_member("u-0", "team-a")
    assert control.can_access("u-0", "doc-0", JOIN_TIME) is False


def test_a_directory_row_written_after_a_leave_does_not_restore_access():
    control, directory = build(documents={"doc-0": "team-a"})
    control.join_team("u-0", "team-a", JOIN_TIME)
    control.leave_team("u-0", "team-a", LEAVE_TIME)
    # The cache re-reads an upstream row that has not caught up yet.
    directory.add_member("u-0", "team-a")
    assert control.can_access("u-0", "doc-0", LEAVE_TIME + 60.0) is False


# --- HC4: standard library only --------------------------------------------


def test_no_third_party_imports():
    for path in sorted(SRC.glob("*.py")):
        tree = ast.parse(path.read_text())
        for node in ast.walk(tree):
            roots = []
            if isinstance(node, ast.Import):
                roots = [alias.name.split(".")[0] for alias in node.names]
            elif isinstance(node, ast.ImportFrom) and node.level == 0 and node.module:
                roots = [node.module.split(".")[0]]
            for root in roots:
                assert root in STDLIB_ALLOWED or root == "src", (
                    "{} imports {!r}; the task permits the standard library only"
                    .format(path.name, root)
                )


# --- HC5: injected time -----------------------------------------------------


def test_the_access_methods_keep_their_injected_time_signatures():
    expected = {
        "can_access": ["self", "user_id", "document_id", "now"],
        "join_team": ["self", "user_id", "team_id", "now"],
        "leave_team": ["self", "user_id", "team_id", "now"],
        "grants_for": ["self", "document_id"],
    }
    for name, parameters in expected.items():
        method = getattr(AccessControl, name)
        found = list(inspect.signature(method).parameters)
        assert found[: len(parameters)] == parameters, (
            "AccessControl.{} takes {}; the interface is {}".format(
                name, found, parameters
            )
        )


def test_the_same_inputs_produce_the_same_decisions():
    def scenario():
        documents = documents_owned_by("team-a", 4)
        control, _ = build(documents=documents)
        users = users_named(5)
        for user_id in users:
            control.join_team(user_id, "team-a", JOIN_TIME)
        control.leave_team(users[2], "team-a", LEAVE_TIME)
        return {
            (user_id, document_id): control.can_access(
                user_id, document_id, LEAVE_TIME + 60.0
            )
            for user_id in users
            for document_id in sorted(documents)
        }

    assert scenario() == scenario()


# --- The provided directory must keep behaving as provided -----------------


def test_the_given_directory_still_records_membership():
    """Guard against satisfying HC3 by neutering the fixture.

    `directory.py` ships inside the participant's `src/`, so a candidate
    could pass every no-implicit-access assertion by making `add_member` a
    no-op: with nothing ever recorded, membership can never confer access
    and the constraint holds vacuously. The directory must still be a
    working lookup table — the security property has to come from the
    access decision, not from a broken cache.
    """
    from src.directory import Directory

    directory = Directory()
    directory.add_member("u-0", "team-a")
    assert directory.is_member("u-0", "team-a") is True
    assert "u-0" in directory.members("team-a")
    directory.remove_member("u-0", "team-a")
    assert directory.is_member("u-0", "team-a") is False


def test_the_given_directory_still_reports_ownership():
    """The ownership half of the same guard."""
    from src.directory import Directory

    directory = Directory(documents={"doc-0": "team-a"})
    assert directory.team_for_document("doc-0") == "team-a"
    assert directory.documents_for_team("team-a") == ("doc-0",)

def test_the_source_cannot_reach_a_clock():
    """HC: injected time. Import ban, not call-shape matching.

    Matching call shapes misses `datetime.datetime.now()` (chained
    attribute) and `from time import time; time()` (a Call on a Name). A
    module that never imports a clock cannot read one, whichever form it
    would have used.
    """
    from clockcheck import find_clock_reads

    findings = find_clock_reads(SRC)
    assert not findings, "source can reach a clock: {}".format(findings)
