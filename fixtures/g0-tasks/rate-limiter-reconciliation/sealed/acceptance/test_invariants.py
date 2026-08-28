"""SEALED invariant tests (must-pass) — N15 §6.4 class 2.

One per mechanically checkable hard constraint. Any failure here is a
constraint violation and forces g0.run.success to 0.
"""

import ast
import inspect
import pathlib

from conftest import GLOBAL_RATE, OBSERVED_PEAK_ACTIVE, PER_USER_RATE, build, interleaved

from src import limiter as limiter_module
from src.config import parse_limits

SRC = pathlib.Path(limiter_module.__file__).parent
STDLIB_ALLOWED = {"math", "collections", "typing", "dataclasses", "itertools"}


# --- HC1: configuration contract -------------------------------------------


def test_a_two_key_file_loads(parsed_two_key):
    assert parsed_two_key["global_rate"] == GLOBAL_RATE
    assert parsed_two_key["per_user_rate"] == PER_USER_RATE


def test_a_two_key_file_constructs_a_working_limiter(parsed_two_key):
    from src.limiter import Limiter

    limiter = Limiter(parsed_two_key)
    assert limiter.allow("u1", 0.0) is True


def test_unknown_keys_are_preserved_not_rejected():
    parsed = parse_limits(
        "global_rate: 800\nper_user_rate: 10\nsome_future_key: 3\n"
    )
    assert parsed["some_future_key"] == 3


# --- HC2: the global ceiling is absolute -----------------------------------


def test_ceiling_holds_for_a_single_sender():
    limiter = build()
    for _ in range(GLOBAL_RATE * 2):
        limiter.allow("u1", 0.0)
    assert limiter.admitted_in_window() <= GLOBAL_RATE


def test_ceiling_holds_at_the_observed_peak():
    users = ["u{}".format(i) for i in range(OBSERVED_PEAK_ACTIVE)]
    limiter = build()
    interleaved(limiter, users, PER_USER_RATE * 2)
    assert limiter.admitted_in_window() <= GLOBAL_RATE


def test_ceiling_holds_under_extreme_fan_out():
    users = ["u{}".format(i) for i in range(GLOBAL_RATE * 2)]
    limiter = build()
    interleaved(limiter, users, 2)
    assert limiter.admitted_in_window() <= GLOBAL_RATE


# --- HC3: no rejection below entitlement while the window has room ---------


def test_a_user_under_entitlement_is_never_rejected_with_room_left():
    limiter = build()
    for index in range(50):
        user = "u{}".format(index)
        for _ in range(PER_USER_RATE):
            admitted = limiter.allow(user, 0.0)
            assert admitted is True, "rejected below entitlement with room left"
    assert limiter.admitted_in_window() <= GLOBAL_RATE


# --- HC4: standard library only --------------------------------------------


def test_no_third_party_imports():
    for path in SRC.glob("*.py"):
        tree = ast.parse(path.read_text())
        for node in ast.walk(tree):
            roots = []
            if isinstance(node, ast.Import):
                roots = [alias.name.split(".")[0] for alias in node.names]
            elif isinstance(node, ast.ImportFrom) and node.level == 0 and node.module:
                roots = [node.module.split(".")[0]]
            for root in roots:
                assert root in STDLIB_ALLOWED or root == "src", (
                    "{} imports {!r}; the task permits the standard library only"
                    .format(path.name, root)
                )


# --- HC5: injected time -----------------------------------------------------


def test_allow_keeps_its_injected_time_signature():
    from src.limiter import Limiter

    parameters = list(inspect.signature(Limiter.allow).parameters)
    assert parameters[:3] == ["self", "user_id", "now"]


def test_the_same_inputs_produce_the_same_admissions():
    users = ["u{}".format(i) for i in range(30)]
    first = interleaved(build(), users, PER_USER_RATE * 2)
    second = interleaved(build(), users, PER_USER_RATE * 2)
    assert first == second

def test_the_source_cannot_reach_a_clock():
    """HC: injected time. Import ban, not call-shape matching.

    Matching call shapes misses `datetime.datetime.now()` (chained
    attribute) and `from time import time; time()` (a Call on a Name). A
    module that never imports a clock cannot read one, whichever form it
    would have used.
    """
    from clockcheck import find_clock_reads

    findings = find_clock_reads(SRC)
    assert not findings, "source can reach a clock: {}".format(findings)
