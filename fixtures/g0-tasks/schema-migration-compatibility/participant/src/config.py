"""Ingest configuration loading.

SYNTHETIC FIXTURE — invented for benchmark use.

`ingest.yaml` is customer-editable. The parser is deliberately tiny: the
file format is flat `key: value` pairs, one per line, and Support runbooks
reference the key names directly.
"""

REQUIRED_KEYS = ("schema_version", "reject_unknown_fields")
BOOLEANS = {"true": True, "false": False}


def _coerce(value):
    """Booleans, then integers, then floats, then the bare string."""
    if value.lower() in BOOLEANS:
        return BOOLEANS[value.lower()]
    try:
        return int(value)
    except ValueError:
        pass
    try:
        return float(value)
    except ValueError:
        return value


def parse_config(text):
    """Parse the flat key/value ingest format into a dict.

    Unknown keys are preserved so a newer service can read an older file and
    an older service can read a newer one.
    """
    parsed = {}
    for raw in text.splitlines():
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        if ":" not in line:
            raise ValueError("malformed config line: {!r}".format(raw))
        key, value = line.split(":", 1)
        parsed[key.strip()] = _coerce(value.strip())
    missing = [key for key in REQUIRED_KEYS if key not in parsed]
    if missing:
        raise ValueError("config file missing required keys: {}".format(missing))
    return parsed
