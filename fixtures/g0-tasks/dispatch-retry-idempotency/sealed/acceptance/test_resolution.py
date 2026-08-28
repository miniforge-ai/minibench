"""SEALED resolution tests (must-pass) — N15 §6.4 class 1.

One or more per injected contradiction, encoding the sealed intended
resolution's observable behaviour.
"""

from conftest import (
    MAX_RETRIES,
    build,
    dispatch_all,
    duplicated,
    interleave,
    payload_for,
    telemetry_shaped_script,
)

from src.sink import Sink, always_permanent, always_transient, transient_then_ok

# --- Contradiction 1, half (a): deduplicate by job id ----------------------


def test_a_lost_acknowledgement_does_not_double_apply():
    """The caller never heard back, so it submits the same job again.

    The side effect was already committed. Submitting again must not commit
    it a second time.
    """
    sink = Sink()
    dispatcher = build(sink)
    first = dispatcher.submit("j1", payload_for("j1"), 0.0)
    assert first["status"] == "applied"

    second = dispatcher.submit("j1", payload_for("j1"), 8.0)
    assert second["status"] == "applied"
    assert sink.effect_count("j1") == 1
    assert sink.attempts("j1") == 1, "the sink was called again for an applied job"


def test_deduplication_is_by_job_id_not_by_payload():
    """Two different jobs are two jobs, even with identical payloads."""
    sink = Sink()
    dispatcher = build(sink)
    shared = payload_for("shared")
    dispatcher.submit("j1", shared, 0.0)
    dispatcher.submit("j2", shared, 0.0)
    assert sink.effect_count("j1") == 1
    assert sink.effect_count("j2") == 1


def test_a_failed_job_is_not_deduplicated_away():
    """Dedupe keys on the effect being applied, not on the job being seen.

    A job that failed has no side effect to protect. Re-submitting it must
    reach the sink again, or the dedupe ledger becomes a new way to lose
    work — the outcome Operations filed against.
    """
    sink = Sink({"j1": transient_then_ok(MAX_RETRIES + 1)})
    dispatcher = build(sink)
    assert dispatcher.submit("j1", payload_for("j1"), 0.0)["status"] == "failed"
    assert sink.effect_count("j1") == 0

    assert dispatcher.submit("j1", payload_for("j1"), 8.0)["status"] == "applied"
    assert sink.effect_count("j1") == 1


# --- Contradiction 1, half (b): retry only transient refusals --------------


def test_a_permanent_rejection_is_not_retried():
    """The telemetry says these never succeed. One attempt, then stop."""
    sink = Sink({"j1": always_permanent()})
    dispatcher = build(sink)
    result = dispatcher.submit("j1", payload_for("j1"), 0.0)
    assert result["status"] == "failed"
    assert sink.attempts("j1") == 1, (
        "a permanent rejection was retried {} times; retrying these recovers "
        "nothing and is what saturates the queue".format(sink.attempts("j1") - 1)
    )


def test_a_transient_refusal_is_still_retried():
    """The other half: classification must not become a reason to give up."""
    sink = Sink({"j1": transient_then_ok(3)})
    dispatcher = build(sink)
    result = dispatcher.submit("j1", payload_for("j1"), 0.0)
    assert result["status"] == "applied"
    assert sink.attempts("j1") == 4
    assert sink.effect_count("j1") == 1


def test_transient_retries_are_bounded_by_max_retries():
    sink = Sink({"j1": always_transient()})
    dispatcher = build(sink)
    result = dispatcher.submit("j1", payload_for("j1"), 0.0)
    assert result["status"] == "failed"
    assert sink.attempts("j1") == MAX_RETRIES + 1


def test_both_halves_hold_together_under_a_mixed_workload():
    """Dedupe alone or classification alone is not the resolution.

    Each job is submitted twice. A rejected job is not deduplicated — it has
    no side effect to protect — so it reaches the sink once per submission
    and never more, because it is never retried.
    """
    submissions = 2
    script, transient_ids, permanent_ids = telemetry_shaped_script(total=200)
    sink = Sink(script)
    dispatcher = build(sink)
    order = interleave(transient_ids, permanent_ids)
    for _ in range(submissions):
        dispatch_all(dispatcher, order)

    assert duplicated(sink) == {}
    wasted = [
        job_id for job_id in permanent_ids if sink.attempts(job_id) != submissions
    ]
    assert not wasted, "retry budget spent on unrecoverable jobs: {}".format(
        wasted[:5]
    )
    assert all(sink.effect_count(job_id) == 1 for job_id in transient_ids)


# --- Contradiction 2: what max_retries now governs -------------------------


def test_the_two_key_configuration_still_produces_correct_behaviour():
    """No new required key: the shipped two-key file must be sufficient."""
    sink = Sink({"t1": transient_then_ok(2), "p1": always_permanent()})
    dispatcher = build(sink)
    assert dispatcher.submit("t1", payload_for("t1"), 0.0)["status"] == "applied"
    assert dispatcher.submit("p1", payload_for("p1"), 0.0)["status"] == "failed"
    dispatcher.submit("t1", payload_for("t1"), 0.0)
    assert sink.effect_count("t1") == 1
    assert sink.attempts("p1") == 1


def test_max_retries_governs_transient_refusals_only():
    """The key keeps its name and its authority over a narrower set."""
    transient_sink = Sink({"j1": always_transient()})
    build(transient_sink).submit("j1", payload_for("j1"), 0.0)
    assert transient_sink.attempts("j1") == MAX_RETRIES + 1

    permanent_sink = Sink({"j1": always_permanent()})
    build(permanent_sink).submit("j1", payload_for("j1"), 0.0)
    assert permanent_sink.attempts("j1") == 1


def test_lowering_max_retries_still_shortens_transient_retrying():
    """Customers tune this key; it must keep responding to tuning."""
    sink = Sink({"j1": always_transient()})
    dispatcher = build(sink, max_retries=1)
    dispatcher.submit("j1", payload_for("j1"), 0.0)
    assert sink.attempts("j1") == 2
