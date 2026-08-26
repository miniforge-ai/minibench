"""SEALED regression tests (must-pass) — N15 §6.4 class 3.

The seeded repository's own visible suite must still pass. Re-running it
from inside the sealed suite means a candidate cannot score success by
deleting or weakening the tests it was given.
"""

import importlib
import inspect

SHIPPED_CASE_COUNT = 9


def _shipped_cases():
    module = importlib.import_module("tests.test_ingest")
    return [
        (name, fn)
        for name, fn in inspect.getmembers(module, inspect.isfunction)
        if name.startswith("test_")
    ]


def test_the_shipped_suite_still_passes():
    cases = _shipped_cases()
    assert cases, "the shipped suite is missing or has no tests"
    failures = []
    for name, fn in cases:
        try:
            fn()
        except Exception as exc:  # noqa: BLE001 - reporting, not handling
            failures.append("{}: {}".format(name, exc))
    assert not failures, "shipped tests failing: {}".format(failures)


def test_the_shipped_suite_was_not_hollowed_out():
    """Guard the count as well as the outcome.

    Deleting cases would otherwise pass the check above trivially.
    """
    cases = _shipped_cases()
    assert len(cases) >= SHIPPED_CASE_COUNT, (
        "expected the {} shipped cases, found {}".format(
            SHIPPED_CASE_COUNT, len(cases)
        )
    )
