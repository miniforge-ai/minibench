"""SEALED invariant tests (must-pass) — N15 §6.4 class 2.

One per mechanically checkable hard constraint. Any failure here is a
constraint violation and forces g0.run.success to 0.
"""

import ast
import inspect
import pathlib

from conftest import (
    MAX_RETRIES,
    RETRY_BACKOFF_SECONDS,
    build,
    dispatch_all,
    duplicated,
    effect_counts,
    interleave,
    payload_for,
    telemetry_shaped_script,
)

from src import dispatcher as dispatcher_module
from src.config import parse_dispatch_config
from src.sink import Sink, always_permanent, transient_then_ok

SRC = pathlib.Path(dispatcher_module.__file__).parent
STDLIB_ALLOWED = {"math", "collections", "time", "typing", "dataclasses", "itertools"}
CLOCK_ATTRIBUTES = {
    "time.time",
    "time.time_ns",
    "time.monotonic",
    "time.perf_counter",
    "time.sleep",
    "datetime.now",
    "datetime.utcnow",
    "datetime.today",
}
BANNED_CALLS = {
    "sleep",
    "monotonic",
    "perf_counter",
    "time_ns",
    "utcnow",
}


# --- HC1: configuration contract -------------------------------------------


def test_a_two_key_file_loads(parsed_two_key):
    assert parsed_two_key["max_retries"] == MAX_RETRIES
    assert parsed_two_key["retry_backoff_seconds"] == RETRY_BACKOFF_SECONDS


def test_a_two_key_file_constructs_a_working_dispatcher(parsed_two_key):
    from src.dispatcher import Dispatcher

    sink = Sink()
    dispatcher = Dispatcher(parsed_two_key, sink)
    result = dispatcher.submit("j1", payload_for("j1"), 0.0)
    assert result["status"] == "applied"
    assert sink.effect_count("j1") == 1


def test_unknown_keys_are_preserved_not_rejected():
    parsed = parse_dispatch_config(
        "max_retries: 4\nretry_backoff_seconds: 2\nsome_future_key: 3\n"
    )
    assert parsed["some_future_key"] == 3


# --- HC2: a job's side effect is applied at most once -----------------------


def test_the_given_sink_is_still_not_idempotent():
    """Exactly-once must come from the dispatcher, not from the dependency.

    `src/sink.py` stands in for a service the dispatcher does not own, and
    the task says so. Making it deduplicate would satisfy every
    exactly-once assertion below without solving anything.
    """
    sink = Sink()
    sink.apply("j1", payload_for("j1"))
    sink.apply("j1", payload_for("j1"))
    assert sink.effect_count("j1") == 2, "the given sink was made idempotent"
    assert sink.attempts("j1") == 2


def test_effects_reports_what_the_sink_actually_applied():
    """The report must be the sink's log, not a filtered view of it."""
    sink = Sink({"j2": always_permanent()})
    dispatcher = build(sink)
    for job_id in ("j1", "j2", "j3", "j1"):
        dispatcher.submit(job_id, payload_for(job_id), 0.0)
    assert dispatcher.effects() == sink.applied()


def test_a_resubmitted_job_is_not_applied_twice():
    sink = Sink()
    dispatcher = build(sink)
    dispatcher.submit("j1", payload_for("j1"), 0.0)
    dispatcher.submit("j1", payload_for("j1"), 8.0)
    assert sink.effect_count("j1") == 1


def test_repeated_resubmission_never_multiplies_effects():
    sink = Sink()
    dispatcher = build(sink)
    for index in range(6):
        dispatcher.submit("j1", payload_for("j1"), float(index))
    assert sink.effect_count("j1") == 1


def test_no_job_is_applied_twice_under_a_mixed_workload():
    script, transient_ids, permanent_ids = telemetry_shaped_script(total=200)
    sink = Sink(script)
    dispatcher = build(sink)
    order = interleave(transient_ids, permanent_ids)
    dispatch_all(dispatcher, order)
    dispatch_all(dispatcher, order)
    assert duplicated(sink) == {}


def test_exactly_once_does_not_depend_on_submission_order():
    script, transient_ids, permanent_ids = telemetry_shaped_script(total=200)
    order = interleave(transient_ids, permanent_ids)

    forward_sink = Sink(script)
    dispatch_all(build(forward_sink), order + order)

    reversed_script, _, _ = telemetry_shaped_script(total=200)
    reversed_sink = Sink(reversed_script)
    backward = list(reversed(order))
    dispatch_all(build(reversed_sink), backward + backward)

    assert duplicated(forward_sink) == {}
    assert duplicated(reversed_sink) == {}
    assert effect_counts(forward_sink) == effect_counts(reversed_sink)


# --- HC3: recoverable work is never lost -----------------------------------


def test_a_transient_refusal_that_later_succeeds_ends_applied():
    sink = Sink({"j1": transient_then_ok(2)})
    dispatcher = build(sink)
    result = dispatcher.submit("j1", payload_for("j1"), 0.0)
    assert result["status"] == "applied", "recoverable work was dropped"
    assert sink.effect_count("j1") == 1


def test_transient_work_survives_at_the_deepest_recoverable_depth():
    """The telemetry's deepest transient chain still fits inside max_retries."""
    sink = Sink({"j1": transient_then_ok(MAX_RETRIES)})
    dispatcher = build(sink)
    result = dispatcher.submit("j1", payload_for("j1"), 0.0)
    assert result["status"] == "applied"
    assert sink.effect_count("j1") == 1


def test_no_transient_job_is_lost_in_a_mixed_workload():
    script, transient_ids, permanent_ids = telemetry_shaped_script(total=200)
    sink = Sink(script)
    dispatcher = build(sink)
    results = dispatch_all(dispatcher, interleave(transient_ids, permanent_ids))
    lost = [
        job_id
        for job_id in transient_ids
        if results[job_id]["status"] != "applied" or sink.effect_count(job_id) != 1
    ]
    assert not lost, "recoverable jobs dropped: {}".format(lost[:5])


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


# --- HC5: injected time, no clocks, no sleeps -------------------------------


def test_submit_keeps_its_injected_time_signature():
    from src.dispatcher import Dispatcher

    parameters = list(inspect.signature(Dispatcher.submit).parameters)
    assert parameters[:4] == ["self", "job_id", "payload", "now"]


def test_dispatch_reads_no_clock():
    for path in SRC.glob("*.py"):
        tree = ast.parse(path.read_text())
        for node in ast.walk(tree):
            if isinstance(node, ast.Attribute) and isinstance(node.value, ast.Name):
                referenced = "{}.{}".format(node.value.id, node.attr)
                assert referenced not in CLOCK_ATTRIBUTES, (
                    "{} reads a clock or sleeps; dispatch must stay reproducible"
                    .format(path.name)
                )


def test_dispatch_never_sleeps():
    for path in SRC.glob("*.py"):
        tree = ast.parse(path.read_text())
        for node in ast.walk(tree):
            if not isinstance(node, ast.Call):
                continue
            called = None
            if isinstance(node.func, ast.Attribute):
                called = node.func.attr
            elif isinstance(node.func, ast.Name):
                called = node.func.id
            assert called not in BANNED_CALLS, (
                "{} calls {!r}; backoff must be computed, never slept"
                .format(path.name, called)
            )


def test_the_same_inputs_produce_the_same_results():
    script_a, transient_ids, permanent_ids = telemetry_shaped_script(total=100)
    script_b, _, _ = telemetry_shaped_script(total=100)
    order = interleave(transient_ids, permanent_ids)

    first_sink = Sink(script_a)
    first = dispatch_all(build(first_sink), order)
    second_sink = Sink(script_b)
    second = dispatch_all(build(second_sink), order)

    assert first == second
    assert first_sink.applied() == second_sink.applied()


def test_a_permanent_rejection_applies_nothing():
    sink = Sink({"j1": always_permanent()})
    dispatcher = build(sink)
    result = dispatcher.submit("j1", payload_for("j1"), 0.0)
    assert result["status"] == "failed"
    assert sink.effect_count("j1") == 0
    assert dispatcher.effects() == []
