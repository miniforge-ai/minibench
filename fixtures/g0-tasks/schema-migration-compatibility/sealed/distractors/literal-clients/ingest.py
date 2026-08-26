"""SEALED distractor — the literal Clients reading.

Makes `region` optional for every client, because that is what it takes for
a v1 record to be accepted, and fills an absent region with the same
placeholder a client may legitimately send. v1 keeps working. Two things are
lost: no client is ever required to send a region, so Product's requirement
is not represented anywhere in the endpoint; and a record whose region was
never sent is stored identically to one a client explicitly declared as
"unknown", so nothing downstream can tell them apart. The acceptance suite
MUST fail this.
"""

from src.errors import ValidationError

BASE_FIELDS = ("device_id", "metric", "value")
KNOWN_FIELDS = BASE_FIELDS + ("region",)
PLACEHOLDER_REGION = "unknown"


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
        for field in BASE_FIELDS:
            if field not in record:
                raise ValidationError(
                    "record missing required field {!r}".format(field)
                )
        if self.reject_unknown_fields:
            unknown = sorted(set(record) - set(KNOWN_FIELDS))
            if unknown:
                raise ValidationError("record has unknown fields: {}".format(unknown))

        entry = dict(record)
        entry.setdefault("region", PLACEHOLDER_REGION)
        self._stored.append(entry)
        return dict(entry)

    def stored(self):
        return [dict(entry) for entry in self._stored]
