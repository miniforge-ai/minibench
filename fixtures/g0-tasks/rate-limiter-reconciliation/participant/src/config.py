"""Limit configuration loading.

SYNTHETIC FIXTURE — invented for benchmark use.

`limits.yaml` is customer-editable. The parser is deliberately tiny: the
file format is flat `key: value` pairs, one per line, and Support runbooks
reference the key names directly.
"""

REQUIRED_KEYS = ("global_rate", "per_user_rate")


def parse_limits(text):
    """Parse the flat key/value limits format into a dict of ints.

    Unknown keys are preserved so a newer service can read an older file and
    an older service can read a newer one.
    """
    parsed = {}
    for raw in text.splitlines():
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        if ":" not in line:
            raise ValueError("malformed limits line: {!r}".format(raw))
        key, value = line.split(":", 1)
        key = key.strip()
        value = value.strip()
        try:
            parsed[key] = int(value)
        except ValueError:
            raise ValueError("limit {!r} must be an integer".format(key))
    missing = [k for k in REQUIRED_KEYS if k not in parsed]
    if missing:
        raise ValueError("limits file missing required keys: {}".format(missing))
    return parsed
