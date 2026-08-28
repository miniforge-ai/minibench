"""Cache configuration loading.

SYNTHETIC FIXTURE — invented for benchmark use.

`cache.yaml` is customer-editable. The parser is deliberately tiny: the file
format is flat `key: value` pairs, one per line, and Support runbooks
reference the key names directly.
"""

REQUIRED_KEYS = ("ttl_seconds", "max_entries")


def parse_cache_config(text):
    """Parse the flat key/value cache format into a dict of ints.

    Unknown keys are preserved so a newer service can read an older file and
    an older service can read a newer one.
    """
    parsed = {}
    for raw in text.splitlines():
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        if ":" not in line:
            raise ValueError("malformed cache config line: {!r}".format(raw))
        key, value = line.split(":", 1)
        key = key.strip()
        value = value.strip()
        try:
            parsed[key] = int(value)
        except ValueError:
            raise ValueError("cache setting {!r} must be an integer".format(key))
    missing = [k for k in REQUIRED_KEYS if k not in parsed]
    if missing:
        raise ValueError("cache config missing required keys: {}".format(missing))
    return parsed
