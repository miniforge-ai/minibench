"""SEALED distractor — the literal Product reading.

Implements "every record must carry a region" as one required field on one
schema, held against every client. Product's requirement is satisfied
exactly, the dashboards reconcile, and every record a v1 client sends is
rejected — which is the seeded reading carried forward without noticing that
v1 builds cannot send the field at all. The acceptance suite MUST fail this.
"""

from src.errors import ValidationError

REQUIRED_FIELDS = ("device_id", "metric", "value", "region")


class Ingestor:
    def __init__(self, config):
        self.schema_version = config["schema_version"]
        self.reject_unknown_fields = config["reject_unknown_fields"]
        self._stored = []

    def ingest(self, record, client_version):
        if not isinstance(record, dict):
            raise ValidationError(
                "record must be a mapping, got {!r}".format(type(record).__name__)
            )
        for field in REQUIRED_FIELDS:
            if field not in record:
                raise ValidationError(
                    "record missing required field {!r}".format(field)
                )
        if self.reject_unknown_fields:
            unknown = sorted(set(record) - set(REQUIRED_FIELDS))
            if unknown:
                raise ValidationError("record has unknown fields: {}".format(unknown))

        entry = dict(record)
        entry["client_version"] = client_version
        self._stored.append(entry)
        return dict(entry)

    def stored(self):
        return [dict(entry) for entry in self._stored]
