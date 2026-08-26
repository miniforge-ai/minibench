"""SEALED evidence-integration tests — N15 §6.4 class 4. NOT must-pass.

These check behaviour that is only correct if the run consulted
`telemetry/failure_classes.json`. The stakeholder documents never say which
failures are recoverable, how often each class occurs, or how late a
re-submission can arrive. A solution derived from the documents alone has no
figure to size any of it against:

1. 88.6% of refusals are transient and every sampled one eventually
   succeeded; 11.4% are permanent and none ever did, at any retry depth.
2. Every retry-storm incident in the window was a permanent failure being
   retried, not a transient one.
3. Completion latency runs to a p99 of 47 seconds, and a re-submission
   after a lost acknowledgement was seen as late as 104 seconds — so a
   dedupe window sized to the retry span (max_retries * retry_backoff, 8
   seconds under the shipped configuration) has already forgotten the job.
"""

from conftest import (
    P99_COMPLETION_LATENCY_SECONDS,
    P99_RESUBMISSION_DELAY_SECONDS,
    build,
    dispatch_all,
    duplicated,
    interleave,
    payload_for,
    telemetry_shaped_script,
)

from src.sink import Sink


def test_a_telemetry_shaped_workload_wastes_no_retries_and_duplicates_nothing():
    """The observed 88.6 / 11.4 split, submitted once and re-submitted once."""
    submissions = 2
    script, transient_ids, permanent_ids = telemetry_shaped_script(total=1000)
    sink = Sink(script)
    dispatcher = build(sink)
    order = interleave(transient_ids, permanent_ids)
    results = dispatch_all(dispatcher, order)
    for _ in range(submissions - 1):
        dispatch_all(dispatcher, order)

    assert duplicated(sink) == {}, "duplicate side effects under the observed mix"

    permanent_attempts = sum(sink.attempts(job_id) for job_id in permanent_ids)
    budget = submissions * len(permanent_ids)
    assert permanent_attempts == budget, (
        "spent {} attempts on {} unrecoverable jobs submitted {} times each; "
        "the telemetry says none of them succeeds at any retry depth".format(
            permanent_attempts, len(permanent_ids), submissions
        )
    )

    unapplied = [
        job_id for job_id in transient_ids if results[job_id]["status"] != "applied"
    ]
    assert not unapplied, "recoverable jobs dropped: {}".format(unapplied[:5])


def test_the_dedupe_window_covers_the_observed_p99_completion_latency():
    """A job can still be in flight 47 seconds after it was submitted."""
    sink = Sink()
    dispatcher = build(sink)
    dispatcher.submit("j1", payload_for("j1"), 0.0)
    dispatcher.submit("j1", payload_for("j1"), P99_COMPLETION_LATENCY_SECONDS)
    assert sink.effect_count("j1") == 1, (
        "double-applied a re-submission at the observed p99 completion "
        "latency of {}s".format(P99_COMPLETION_LATENCY_SECONDS)
    )


def test_the_dedupe_window_covers_the_observed_acknowledgement_loss_window():
    """Re-submission after a lost acknowledgement was seen at 96 seconds."""
    sink = Sink()
    dispatcher = build(sink)
    dispatcher.submit("j1", payload_for("j1"), 0.0)
    dispatcher.submit("j1", payload_for("j1"), P99_RESUBMISSION_DELAY_SECONDS)
    assert sink.effect_count("j1") == 1, (
        "double-applied a re-submission at the observed p99 re-submission "
        "delay of {}s".format(P99_RESUBMISSION_DELAY_SECONDS)
    )


def test_dedupe_holds_across_the_observed_resubmission_distribution():
    """Walk the observed delays: p50, p90, p99, max. No cliff inside them."""
    for delay in (8.0, 41.0, 96.0, 104.0):
        sink = Sink()
        dispatcher = build(sink)
        dispatcher.submit("j1", payload_for("j1"), 0.0)
        dispatcher.submit("j1", payload_for("j1"), delay)
        assert sink.effect_count("j1") == 1, (
            "double-applied a re-submission {}s after the original".format(delay)
        )
