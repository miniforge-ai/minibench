"""SEALED regression tests (must-pass) — N15 §6.4 class 3.

The seeded repository's own visible suite must still pass. Re-running it
from inside the sealed suite means a candidate cannot score success by
deleting or weakening the tests it was given.
"""

import importlib
import inspect


def test_the_shipped_suite_still_passes():
    module = importlib.import_module("tests.test_dispatcher")
    cases = [
        (name, fn)
        for name, fn in inspect.getmembers(module, inspect.isfunction)
        if name.startswith("test_")
    ]
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
    module = importlib.import_module("tests.test_dispatcher")
    cases = [
        name
        for name, _ in inspect.getmembers(module, inspect.isfunction)
        if name.startswith("test_")
    ]
    assert len(cases) >= 10, "expected the ten shipped cases, found {}".format(
        len(cases)
    )
