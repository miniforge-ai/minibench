"""Suite shipped with the Quillon ingest endpoint. SYNTHETIC fixture.

These assertions describe behaviour that must hold before and after any
change to the endpoint.
"""

from src.config import parse_config
from src.errors import ValidationError
from src.ingest import Ingestor


def config(schema_version=3, reject_unknown_fields=False, **extra):
    base = {
        "schema_version": schema_version,
        "reject_unknown_fields": reject_unknown_fields,
    }
    base.update(extra)
    return base


def record(**overrides):
    base = {
        "device_id": "d-1",
        "metric": "queue_depth",
        "value": 12,
        "region": "eu-west",
    }
    base.update(overrides)
    return base


def test_parses_the_documented_keys():
    parsed = parse_config("schema_version: 3\nreject_unknown_fields: false\n")
    assert parsed["schema_version"] == 3
    assert parsed["reject_unknown_fields"] is False


def test_ignores_comments_and_blank_lines():
    parsed = parse_config(
        "# heading\n\nschema_version: 2  # inline\nreject_unknown_fields: true\n"
    )
    assert parsed == {"schema_version": 2, "reject_unknown_fields": True}


def test_rejects_a_config_missing_required_keys():
    try:
        parse_config("schema_version: 3\n")
    except ValueError:
        return
    raise AssertionError("expected a ValueError for a missing required key")


def test_accepts_a_record_from_the_current_client():
    ingestor = Ingestor(config())
    stored = ingestor.ingest(record(), "v3")
    assert stored["device_id"] == "d-1"
    assert stored["region"] == "eu-west"


def test_stored_returns_the_accepted_records_in_arrival_order():
    ingestor = Ingestor(config())
    ingestor.ingest(record(device_id="d-1"), "v3")
    ingestor.ingest(record(device_id="d-2"), "v3")
    assert [entry["device_id"] for entry in ingestor.stored()] == ["d-1", "d-2"]


def test_rejects_a_record_missing_a_base_field():
    ingestor = Ingestor(config())
    payload = record()
    del payload["device_id"]
    try:
        ingestor.ingest(payload, "v3")
    except ValidationError:
        return
    raise AssertionError("expected a ValidationError for a missing base field")


def test_rejects_unknown_fields_when_configured_to():
    ingestor = Ingestor(config(reject_unknown_fields=True))
    try:
        ingestor.ingest(record(nonsense="x"), "v3")
    except ValidationError:
        return
    raise AssertionError("expected a ValidationError for an unknown field")


def test_keeps_unknown_fields_when_not_configured_to_reject():
    ingestor = Ingestor(config(reject_unknown_fields=False))
    stored = ingestor.ingest(record(trace_id="t-9"), "v3")
    assert stored["trace_id"] == "t-9"


def test_an_accepted_record_is_retrievable_unchanged_in_its_sent_fields():
    ingestor = Ingestor(config())
    payload = record(device_id="d-5", value=41)
    ingestor.ingest(dict(payload), "v3")
    entry = ingestor.stored()[0]
    for key, value in payload.items():
        assert entry[key] == value
