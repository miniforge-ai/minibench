"""SEALED invariant tests (must-pass) — N15 §6.4 class 2.

One per mechanically checkable hard constraint. Any failure here is a
constraint violation and forces g0.run.success to 0.
"""

import ast
import inspect
import pathlib

from conftest import (
    CURRENT_CLIENT,
    GENUINE_UNKNOWN_REGION,
    PRE_REGION_CLIENT,
    SCHEMA_VERSION,
    build,
    fingerprint,
    fingerprints,
    record,
)

from src import ingest as ingest_module
from src.config import parse_config
from src.ingest import Ingestor

SRC = pathlib.Path(ingest_module.__file__).parent
STDLIB_ALLOWED = {"math", "collections", "typing", "dataclasses", "itertools", "json"}
CLOCK_MODULES = {"time", "datetime", "calendar", "zoneinfo"}
CLOCK_READS = {
    "time.time",
    "time.monotonic",
    "time.time_ns",
    "time.localtime",
    "datetime.now",
    "datetime.today",
    "datetime.utcnow",
    "date.today",
}


def _source_trees():
    for path in sorted(SRC.glob("*.py")):
        yield path, ast.parse(path.read_text())


def _import_roots(tree):
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                yield alias.name.split(".")[0]
        elif isinstance(node, ast.ImportFrom) and node.level == 0 and node.module:
            yield node.module.split(".")[0]


# --- HC1: configuration contract -------------------------------------------


def test_a_two_key_file_loads(parsed_two_key):
    assert parsed_two_key["schema_version"] == SCHEMA_VERSION
    assert parsed_two_key["reject_unknown_fields"] is False


def test_a_two_key_file_constructs_a_working_ingestor(parsed_two_key):
    ingestor = Ingestor(parsed_two_key)
    stored = ingestor.ingest(record(region="eu-west"), CURRENT_CLIENT)
    assert stored["region"] == "eu-west"


def test_unknown_config_keys_are_preserved_not_rejected():
    parsed = parse_config(
        "schema_version: 3\nreject_unknown_fields: false\nsome_future_key: 7\n"
    )
    assert parsed["some_future_key"] == 7


# --- HC2: no v1 breakage ---------------------------------------------------


def test_a_valid_v1_record_is_accepted():
    assert build().ingest(record(), PRE_REGION_CLIENT) is not None


def test_every_record_in_a_v1_batch_is_accepted():
    ingestor = build()
    for index in range(50):
        ingestor.ingest(record(device_id="d-{}".format(index)), PRE_REGION_CLIENT)
    assert len(ingestor.stored()) == 50


def test_v1_acceptance_does_not_depend_on_arrival_order():
    """Whether a v1 record is accepted must not depend on what preceded it."""
    forward = build()
    forward.ingest(record(device_id="d-a", region="eu-west"), CURRENT_CLIENT)
    forward.ingest(record(device_id="d-b"), PRE_REGION_CLIENT)

    reverse = build()
    reverse.ingest(record(device_id="d-b"), PRE_REGION_CLIENT)
    reverse.ingest(record(device_id="d-a", region="eu-west"), CURRENT_CLIENT)

    assert fingerprints(forward) == fingerprints(reverse)


# --- HC3: no silent data loss ----------------------------------------------


def test_every_accepted_record_is_retrievable():
    ingestor = build()
    sent = [
        (record(device_id="d-1"), PRE_REGION_CLIENT),
        (record(device_id="d-2", region="eu-west"), "v2"),
        (record(device_id="d-3", region="us-east"), "v3"),
    ]
    for payload, client_version in sent:
        ingestor.ingest(dict(payload), client_version)
    stored = ingestor.stored()
    assert len(stored) == len(sent)
    assert {entry["device_id"] for entry in stored} == {"d-1", "d-2", "d-3"}


def test_fields_the_client_sent_are_preserved_exactly():
    ingestor = build(reject_unknown_fields=False)
    payload = record(device_id="d-9", value=41, region="ap-south", trace_id="t-3")
    ingestor.ingest(dict(payload), CURRENT_CLIENT)
    entry = ingestor.stored()[0]
    for key, value in payload.items():
        assert entry[key] == value, "field {!r} was not preserved".format(key)


def test_the_region_is_never_silently_dropped():
    ingestor = build()
    ingestor.ingest(record(), PRE_REGION_CLIENT)
    assert "region" in ingestor.stored()[0]


def test_a_missing_region_is_not_stored_as_a_declared_one():
    """The store must keep the two cases apart, whatever mechanism it uses."""
    payload = record(device_id="d-7", value=41)
    ingestor = build()
    ingestor.ingest(dict(payload), PRE_REGION_CLIENT)
    ingestor.ingest(dict(payload, region=GENUINE_UNKNOWN_REGION), CURRENT_CLIENT)
    from_v1, declared_unknown = ingestor.stored()
    assert fingerprint(from_v1) != fingerprint(declared_unknown)


# --- HC4: standard library only --------------------------------------------


def test_no_third_party_imports():
    for path, tree in _source_trees():
        for root in _import_roots(tree):
            assert root in STDLIB_ALLOWED or root == "src", (
                "{} imports {!r}; the task permits the standard library only"
                .format(path.name, root)
            )


# --- HC5: no clock reads ---------------------------------------------------


def test_no_clock_module_is_imported():
    for path, tree in _source_trees():
        for root in _import_roots(tree):
            assert root not in CLOCK_MODULES, (
                "{} imports {!r}; deprecation must be driven by observed data, "
                "not by a date".format(path.name, root)
            )


def test_no_clock_is_read():
    for path, tree in _source_trees():
        for node in ast.walk(tree):
            if isinstance(node, ast.Attribute) and isinstance(node.value, ast.Name):
                called = "{}.{}".format(node.value.id, node.attr)
                assert called not in CLOCK_READS, (
                    "{} reads a clock; ingestion must stay reproducible"
                    .format(path.name)
                )


def test_ingest_keeps_its_injected_client_version_signature():
    parameters = list(inspect.signature(Ingestor.ingest).parameters)
    assert parameters[:3] == ["self", "record", "client_version"]


def test_the_same_inputs_produce_the_same_stored_records():
    payloads = [
        record(device_id="d-{}".format(index), value=index, region="eu-west")
        for index in range(8)
    ]

    def run():
        ingestor = build()
        for payload in payloads:
            ingestor.ingest(dict(payload), CURRENT_CLIENT)
        return [fingerprint(entry) for entry in ingestor.stored()]

    assert run() == run()
