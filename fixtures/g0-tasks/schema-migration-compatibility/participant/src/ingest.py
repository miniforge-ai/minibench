"""Record ingestion for the Quillon telemetry endpoint.

SYNTHETIC FIXTURE — this product, its customers, its client releases and its
telemetry are invented for benchmark use. Nothing here describes a real
system.

Every record is validated against one current schema. `region` was added to
that schema this quarter, so every record must now carry it.
"""

from src.errors import ValidationError

CURRENT_SCHEMA_FIELDS = ("device_id", "metric", "value", "region")


class Ingestor:
    """Validates every record against the single current schema."""

    def __init__(self, config):
        self.schema_version = config["schema_version"]
        self.reject_unknown_fields = config["reject_unknown_fields"]
        self._stored = []

    def ingest(self, record, client_version):
        """Validate and store one record; return the stored record.

        `client_version` is recorded on the stored record. It does not
        affect validation: there is one schema and every client is held to
        it.
        """
        if not isinstance(record, dict):
            raise ValidationError(
                "record must be a mapping, got {!r}".format(type(record).__name__)
            )
        for field in CURRENT_SCHEMA_FIELDS:
            if field not in record:
                raise ValidationError(
                    "record missing required field {!r}".format(field)
                )
        if self.reject_unknown_fields:
            unknown = sorted(set(record) - set(CURRENT_SCHEMA_FIELDS))
            if unknown:
                raise ValidationError("record has unknown fields: {}".format(unknown))

        entry = dict(record)
        entry["client_version"] = client_version
        self._stored.append(entry)
        return dict(entry)

    def stored(self):
        """Every record the endpoint has accepted, in arrival order."""
        return [dict(entry) for entry in self._stored]
