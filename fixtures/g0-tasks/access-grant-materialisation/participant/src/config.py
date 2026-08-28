"""Access-control configuration loading.

SYNTHETIC FIXTURE — invented for benchmark use.

`acl.yaml` is customer-editable. The parser is deliberately tiny: the file
format is flat `key: value` pairs, one per line, and Support runbooks
reference the key names directly.
"""

REQUIRED_KEYS = ("default_visibility", "max_grants_per_document")


def parse_acl(text):
    """Parse the flat key/value ACL format into a dict.

    Values that look like integers are converted to ints; every other value
    is kept as a string. Unknown keys are preserved so a newer service can
    read an older file and an older service can read a newer one.
    """
    parsed = {}
    for raw in text.splitlines():
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        if ":" not in line:
            raise ValueError("malformed acl line: {!r}".format(raw))
        key, value = line.split(":", 1)
        key = key.strip()
        value = value.strip()
        if not key:
            raise ValueError("malformed acl line: {!r}".format(raw))
        try:
            parsed[key] = int(value)
        except ValueError:
            parsed[key] = value
    missing = [k for k in REQUIRED_KEYS if k not in parsed]
    if missing:
        raise ValueError("acl file missing required keys: {}".format(missing))
    return parsed
