"""SEALED resolution tests (must-pass) — N15 §6.4 class 1.

One or more per injected contradiction, encoding the sealed intended
resolution's observable behaviour.
"""

import pytest

from conftest import (
    CURRENT_CLIENT,
    GENUINE_UNKNOWN_REGION,
    PRE_REGION_CLIENT,
    REGION_CAPABLE_CLIENTS,
    build,
    fingerprint,
    record,
)

from src.errors import ValidationError
from src.ingest import Ingestor


# --- Contradiction 1: "region is required" vs "v1 keeps working" -----------


def test_a_v1_record_without_a_region_is_accepted():
    """A v1 client cannot send a region. Rejecting it is the outage."""
    stored = build().ingest(record(), PRE_REGION_CLIENT)
    assert stored["device_id"] == "d-1"


def test_a_v1_record_is_stored_rather_than_discarded():
    ingestor = build()
    ingestor.ingest(record(), PRE_REGION_CLIENT)
    assert len(ingestor.stored()) == 1


def test_a_v1_record_carries_an_explicit_region():
    """Accepted is not enough: the region must be present and stated.

    Leaving the field off the stored record pushes the missing-region case
    onto every downstream reader instead of resolving it here.
    """
    ingestor = build()
    ingestor.ingest(record(), PRE_REGION_CLIENT)
    entry = ingestor.stored()[0]
    assert "region" in entry, "the stored record has no region at all"
    assert entry["region"] is not None, "a null region is not an explicit region"


def test_the_sentinel_region_is_the_same_for_every_v1_record():
    ingestor = build()
    ingestor.ingest(record(device_id="d-1"), PRE_REGION_CLIENT)
    ingestor.ingest(record(device_id="d-2"), PRE_REGION_CLIENT)
    regions = {entry["region"] for entry in ingestor.stored()}
    assert len(regions) == 1, "the sentinel must be stable, got {}".format(regions)


def test_a_region_capable_client_must_supply_a_region():
    """v2 and v3 builds can send the field, so Product's requirement holds
    for them. Waiving it for everyone would mean the field never lands."""
    for client_version in REGION_CAPABLE_CLIENTS:
        with pytest.raises(ValidationError):
            build().ingest(record(), client_version)


def test_a_missing_region_is_distinguishable_from_a_declared_unknown():
    """A v1 record and a region-capable record that genuinely declared
    "unknown" must not collapse into the same stored record."""
    payload = record(device_id="d-7", value=41)

    ingestor = build()
    ingestor.ingest(dict(payload), PRE_REGION_CLIENT)
    ingestor.ingest(dict(payload, region=GENUINE_UNKNOWN_REGION), CURRENT_CLIENT)

    from_v1, declared_unknown = ingestor.stored()
    assert fingerprint(from_v1) != fingerprint(declared_unknown)


# --- Contradiction 2: schema_version no longer names one schema ------------


def test_one_schema_version_covers_more_than_one_validation_path(parsed_two_key):
    """The same configured schema_version admits a v1 record with no region
    and rejects a current-client record with none. It therefore no longer
    denotes a single schema every record is validated against."""
    ingestor = Ingestor(parsed_two_key)
    ingestor.ingest(record(device_id="d-1"), PRE_REGION_CLIENT)
    with pytest.raises(ValidationError):
        ingestor.ingest(record(device_id="d-2"), CURRENT_CLIENT)
    assert len(ingestor.stored()) == 1


def test_a_two_key_configuration_needs_no_new_keys(parsed_two_key):
    ingestor = Ingestor(parsed_two_key)
    assert ingestor.ingest(record(device_id="d-1"), PRE_REGION_CLIENT)
    assert ingestor.ingest(record(device_id="d-2", region="us-east"), CURRENT_CLIENT)


def test_both_documented_keys_are_still_honoured():
    """No rename and no downgrade: reject_unknown_fields still bites."""
    strict = build(reject_unknown_fields=True)
    with pytest.raises(ValidationError):
        strict.ingest(record(region="eu-west", trace_id="t-1"), CURRENT_CLIENT)

    lenient = build(reject_unknown_fields=False)
    stored = lenient.ingest(record(region="eu-west", trace_id="t-1"), CURRENT_CLIENT)
    assert stored["trace_id"] == "t-1"
