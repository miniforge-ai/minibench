"""SEALED acceptance-suite helpers. Never mounted in a participant capsule."""

import pytest

from src.access import AccessControl
from src.config import parse_acl
from src.directory import Directory

DEFAULT_VISIBILITY = "team"
MAX_GRANTS_PER_DOCUMENT = 250  # acl.yaml, as shipped

# telemetry/access_audit.json — the figures the evidence tests are calibrated to
P99_TEAM_SIZE = 260
MEAN_TEAM_SIZE = 14
MEAN_DOCUMENTS_PER_TEAM = 340
STALENESS_PERCENTILE_SECONDS = (240, 3600, 18900, 21600)
STALENESS_WINDOW_SECONDS = 21600  # six hours, the observed maximum

SHIPPED_ACL_TEXT = "default_visibility: {}\nmax_grants_per_document: {}\n".format(
    DEFAULT_VISIBILITY, MAX_GRANTS_PER_DOCUMENT
)

JOIN_TIME = 1000.0
LEAVE_TIME = 2000.0


def build(documents=None, teams=None, **overrides):
    """An access control plus its directory, under the shipped configuration."""
    config = {
        "default_visibility": DEFAULT_VISIBILITY,
        "max_grants_per_document": MAX_GRANTS_PER_DOCUMENT,
    }
    config.update(overrides)
    directory = Directory(teams=teams, documents=documents)
    return AccessControl(config, directory), directory


def users_named(count, prefix="u"):
    return ["{}-{}".format(prefix, index) for index in range(count)]


def documents_owned_by(team_id, count, prefix="doc"):
    return {"{}-{}".format(prefix, index): team_id for index in range(count)}


def join_all(control, users, team_id, now):
    """Join every user in turn, recording whether the call raised.

    Returns {user_id: "ok" | "raised"}. Callers must not assume *which* users
    a capped implementation admits — only counts, and whether a refusal was
    surfaced. Admission order is an implementation choice, not a requirement.
    """
    outcomes = {}
    for user_id in users:
        try:
            control.join_team(user_id, team_id, now)
        except Exception:  # noqa: BLE001 — raising is one accepted signal
            outcomes[user_id] = "raised"
        else:
            outcomes[user_id] = "ok"
    return outcomes


# --- grant-record readers ---------------------------------------------------
#
# Records are documented as dicts (see participant/README.md). These readers
# also accept objects with the same attribute names, so a solution that used a
# small class instead of a dict is judged on behaviour, not on packaging.


def field(record, name):
    """Read one field of a grant record, mapping or object."""
    try:
        return record[name]
    except (TypeError, KeyError, IndexError):
        return getattr(record, name, None)


def is_active(record):
    """True when the record is not revoked."""
    if field(record, "revoked_at") is not None:
        return False
    return field(record, "active") is not False


def records_naming(records, user_id):
    """Every record for `user_id`, revoked ones included."""
    return [record for record in records if field(record, "user_id") == user_id]


def active_records_naming(records, user_id):
    return [record for record in records_naming(records, user_id) if is_active(record)]


@pytest.fixture
def shipped_acl_text():
    return SHIPPED_ACL_TEXT


@pytest.fixture
def parsed_shipped_acl(shipped_acl_text):
    return parse_acl(shipped_acl_text)
