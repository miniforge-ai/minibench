"""SEALED acceptance-suite helpers. Never mounted in a participant capsule."""

import pytest

from src.config import parse_config
from src.ingest import Ingestor

SCHEMA_VERSION = 3
REJECT_UNKNOWN_FIELDS = False

PRE_REGION_CLIENT = "v1"
REGION_CAPABLE_CLIENTS = ("v2", "v3")
CURRENT_CLIENT = "v3"

# telemetry/client_versions.json
OBSERVED_V1_SHARE = 0.032
V1_SHARE_CHANGE_PER_WEEK = -0.0015
# Hypothetical residual tail, well below any defensible threshold.
RESIDUAL_V1_SHARE = 0.0005

# The literal string a region-capable client may legitimately send, and the
# obvious choice of sentinel for a record that carried no region at all.
GENUINE_UNKNOWN_REGION = "unknown"


def build(**overrides):
    """An ingestor under the shipped two-key configuration unless overridden."""
    config = {
        "schema_version": SCHEMA_VERSION,
        "reject_unknown_fields": REJECT_UNKNOWN_FIELDS,
    }
    config.update(overrides)
    return Ingestor(config)


def record(**overrides):
    """A base record. `region` is supplied only when a test asks for it."""
    base = {"device_id": "d-1", "metric": "queue_depth", "value": 12}
    base.update(overrides)
    return base


def fingerprint(entry):
    """Order-independent identity for a stored record.

    Stored records are compared by content, never by dict ordering: key
    insertion order is an implementation detail and must not decide whether
    a test passes.
    """
    return tuple(sorted((key, repr(value)) for key, value in entry.items()))


def fingerprints(ingestor):
    return {fingerprint(entry) for entry in ingestor.stored()}


@pytest.fixture
def two_key_config_text():
    return "schema_version: {}\nreject_unknown_fields: false\n".format(SCHEMA_VERSION)


@pytest.fixture
def parsed_two_key(two_key_config_text):
    return parse_config(two_key_config_text)
