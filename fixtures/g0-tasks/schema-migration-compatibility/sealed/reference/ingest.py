"""SEALED reference solution — per-client-version validation paths.

Solvability proof for N15 §6.3(1): this passes the full acceptance suite and
no LLM was involved in writing it.

Scheme. `schema_version` names the newest schema generation the endpoint
knows, not a single schema every record is held to. A record from client
`vN` is validated against generation `min(N, schema_version)`. Generation 1
has no `region`; generation 2 introduced it as required. So a `v1` client —
which has no way to send a region — validates against generation 1 and is
accepted, while every region-capable client is required to send one.

An accepted record with no client-supplied region is stored with an explicit
sentinel region plus the provenance of that value, so a record whose region
was never sent is never confused with one a client deliberately labelled
"unknown". Nothing is dropped and nothing is invented silently.

Retiring the generation-1 path is gated on the observed `v1` share falling
below a threshold — `can_deprecate_v1(observed_v1_share)` — not on a date.
The telemetry shows `v1` at 3.2% of records declining by 0.15 percentage
points per week, so any fixed date picked today would either break a live
population or sit so far out as to be meaningless.
"""

from src.errors import ValidationError

BASE_FIELDS = ("device_id", "metric", "value")
KNOWN_FIELDS = BASE_FIELDS + ("region",)
SUPPORTED_CLIENT_VERSIONS = ("v1", "v2", "v3")

REGION_REQUIRED_FROM_GENERATION = 2
DEFAULT_SENTINEL_REGION = "unknown"
DEFAULT_V1_DEPRECATION_SHARE_THRESHOLD = 0.005

REGION_FROM_CLIENT = "client"
REGION_NOT_SENT = "not-sent-by-client"


class Ingestor:
    """Validates each record against the schema generation its client knows."""

    def __init__(self, config):
        # The two documented keys stay authoritative and unrenamed.
        self.schema_version = config["schema_version"]
        self.reject_unknown_fields = config["reject_unknown_fields"]
        # Everything new is optional with a default, so a two-key file works.
        self.sentinel_region = config.get("sentinel_region", DEFAULT_SENTINEL_REGION)
        self.v1_deprecation_share_threshold = config.get(
            "v1_deprecation_share_threshold", DEFAULT_V1_DEPRECATION_SHARE_THRESHOLD
        )
        self._stored = []

    def schema_generation_for(self, client_version):
        """The schema generation a client's records are validated against.

        A client cannot be held to a generation it predates, and the
        endpoint will not validate against a generation newer than the
        configured `schema_version`.
        """
        if client_version not in SUPPORTED_CLIENT_VERSIONS:
            raise ValidationError(
                "unsupported client version: {!r}".format(client_version)
            )
        client_generation = int(client_version[1:])
        return min(client_generation, self.schema_version)

    def region_is_required_for(self, client_version):
        """True when the client's generation is expected to send a region."""
        return (
            self.schema_generation_for(client_version)
            >= REGION_REQUIRED_FROM_GENERATION
        )

    def _resolve_region(self, record, client_version):
        """Return (region, provenance) for one record."""
        if "region" in record:
            return record["region"], REGION_FROM_CLIENT
        if self.region_is_required_for(client_version):
            raise ValidationError(
                "client {} must supply 'region'".format(client_version)
            )
        return self.sentinel_region, REGION_NOT_SENT

    def ingest(self, record, client_version):
        """Validate and store one record; return the stored record."""
        if not isinstance(record, dict):
            raise ValidationError(
                "record must be a mapping, got {!r}".format(type(record).__name__)
            )
        # Raises for an unsupported client version before anything is stored.
        self.schema_generation_for(client_version)

        for field in BASE_FIELDS:
            if field not in record:
                raise ValidationError(
                    "record missing required field {!r}".format(field)
                )
        if self.reject_unknown_fields:
            unknown = sorted(set(record) - set(KNOWN_FIELDS))
            if unknown:
                raise ValidationError("record has unknown fields: {}".format(unknown))

        region, provenance = self._resolve_region(record, client_version)

        entry = dict(record)
        entry["region"] = region
        entry["region_source"] = provenance
        entry["client_version"] = client_version
        self._stored.append(entry)
        return dict(entry)

    def stored(self):
        """Every record the endpoint has accepted, in arrival order."""
        return [dict(entry) for entry in self._stored]

    def can_deprecate_v1(self, observed_v1_share):
        """True when the observed `v1` share is small enough to retire it.

        The condition is the measured share, not a date and not a count of
        releases since v1 shipped. Support policy is defensible only against
        a number somebody can go and check.
        """
        return observed_v1_share < self.v1_deprecation_share_threshold
