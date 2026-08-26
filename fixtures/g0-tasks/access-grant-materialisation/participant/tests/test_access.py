"""Suite shipped with the Pinegate workspace service. SYNTHETIC fixture.

These assertions describe behaviour that must hold before and after any
change to the access rules.
"""

from src.access import AccessControl
from src.config import parse_acl
from src.directory import Directory

SHIPPED_ACL = "default_visibility: team\nmax_grants_per_document: 250\n"
PRIVATE_ACL = "default_visibility: private\nmax_grants_per_document: 250\n"


def build(config_text=SHIPPED_ACL, teams=None, documents=None):
    """An access control over a directory, with the shipped configuration."""
    directory = Directory(teams=teams, documents=documents)
    return AccessControl(parse_acl(config_text), directory), directory


def test_parses_the_documented_keys():
    parsed = parse_acl(SHIPPED_ACL)
    assert parsed["default_visibility"] == "team"
    assert parsed["max_grants_per_document"] == 250


def test_ignores_comments_and_blank_lines():
    parsed = parse_acl(
        "# heading\n\ndefault_visibility: team  # inline\nmax_grants_per_document: 40\n"
    )
    assert parsed == {"default_visibility": "team", "max_grants_per_document": 40}


def test_rejects_a_file_missing_required_keys():
    try:
        parse_acl("default_visibility: team\n")
    except ValueError:
        return
    raise AssertionError("expected a ValueError for a missing required key")


def test_joining_a_team_gives_access_to_its_documents():
    control, _ = build(documents={"doc-1": "team-a", "doc-2": "team-a"})
    control.join_team("u1", "team-a", 100.0)
    assert control.can_access("u1", "doc-1", 100.0) is True
    assert control.can_access("u1", "doc-2", 100.0) is True


def test_leaving_a_team_removes_access():
    control, _ = build(documents={"doc-1": "team-a"})
    control.join_team("u1", "team-a", 100.0)
    control.leave_team("u1", "team-a", 200.0)
    assert control.can_access("u1", "doc-1", 300.0) is False


def test_an_unrelated_user_cannot_access_a_team_document():
    control, _ = build(documents={"doc-1": "team-a"})
    control.join_team("u1", "team-a", 100.0)
    assert control.can_access("stranger", "doc-1", 100.0) is False


def test_a_document_owned_by_another_team_is_not_accessible():
    control, _ = build(documents={"doc-1": "team-a", "doc-2": "team-b"})
    control.join_team("u1", "team-a", 100.0)
    assert control.can_access("u1", "doc-2", 100.0) is False


def test_a_document_with_no_owning_team_is_not_accessible():
    control, _ = build(documents={"doc-1": "team-a"})
    control.join_team("u1", "team-a", 100.0)
    assert control.can_access("u1", "doc-unowned", 100.0) is False


def test_private_visibility_does_not_confer_team_access():
    control, _ = build(PRIVATE_ACL, documents={"doc-1": "team-a"})
    control.join_team("u1", "team-a", 100.0)
    assert control.can_access("u1", "doc-1", 100.0) is False


def test_grants_for_an_unknown_document_is_empty():
    control, _ = build(documents={"doc-1": "team-a"})
    assert list(control.grants_for("doc-nonexistent")) == []


def test_the_directory_reports_document_ownership():
    _, directory = build(documents={"doc-1": "team-a"})
    assert directory.team_for_document("doc-1") == "team-a"
    assert directory.team_for_document("doc-9") is None
    assert directory.documents_for_team("team-a") == ("doc-1",)
