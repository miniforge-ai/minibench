"""Suite shipped with the Stonecrop dispatcher. SYNTHETIC fixture.

These assertions describe behaviour that must hold before and after any
change to the dispatcher.
"""

from src.config import parse_dispatch_config
from src.dispatcher import Dispatcher
from src.sink import Sink, always_permanent, always_transient, transient_then_ok

MAX_RETRIES = 4
RETRY_BACKOFF_SECONDS = 2


def config(max_retries=MAX_RETRIES, retry_backoff_seconds=RETRY_BACKOFF_SECONDS, **extra):
    base = {
        "max_retries": max_retries,
        "retry_backoff_seconds": retry_backoff_seconds,
    }
    base.update(extra)
    return base


def payload(job_id):
    return {"job_id": job_id, "op": "post_ledger_entry", "amount_cents": 1200}


def test_parses_the_documented_keys():
    parsed = parse_dispatch_config("max_retries: 4\nretry_backoff_seconds: 2\n")
    assert parsed["max_retries"] == 4
    assert parsed["retry_backoff_seconds"] == 2


def test_ignores_comments_and_blank_lines():
    parsed = parse_dispatch_config(
        "# heading\n\nmax_retries: 6  # inline\nretry_backoff_seconds: 5\n"
    )
    assert parsed == {"max_retries": 6, "retry_backoff_seconds": 5}


def test_rejects_a_file_missing_required_keys():
    try:
        parse_dispatch_config("max_retries: 4\n")
    except ValueError:
        return
    raise AssertionError("expected a ValueError for a missing required key")


def test_a_job_the_sink_accepts_is_applied_once():
    sink = Sink()
    dispatcher = Dispatcher(config(), sink)
    result = dispatcher.submit("j1", payload("j1"), 0.0)
    assert result["job_id"] == "j1"
    assert result["status"] == "applied"
    assert sink.effect_count("j1") == 1


def test_a_transient_refusal_is_retried_until_it_succeeds():
    sink = Sink({"j1": transient_then_ok(2)})
    dispatcher = Dispatcher(config(), sink)
    result = dispatcher.submit("j1", payload("j1"), 0.0)
    assert result["status"] == "applied"
    assert result["attempts"] == 3
    assert sink.effect_count("j1") == 1


def test_a_job_the_sink_never_accepts_reports_failure():
    sink = Sink({"j1": always_transient()})
    dispatcher = Dispatcher(config(), sink)
    result = dispatcher.submit("j1", payload("j1"), 0.0)
    assert result["status"] == "failed"
    assert result["attempts"] == MAX_RETRIES + 1
    assert sink.effect_count("j1") == 0


def test_a_rejected_job_applies_no_side_effect():
    sink = Sink({"j1": always_permanent()})
    dispatcher = Dispatcher(config(), sink)
    result = dispatcher.submit("j1", payload("j1"), 0.0)
    assert result["status"] == "failed"
    assert dispatcher.effects() == []


def test_effects_reports_applied_side_effects_in_order():
    sink = Sink({"j2": always_permanent()})
    dispatcher = Dispatcher(config(), sink)
    for job_id in ("j1", "j2", "j3"):
        dispatcher.submit(job_id, payload(job_id), 0.0)
    assert [job_id for job_id, _ in dispatcher.effects()] == ["j1", "j3"]
    assert dispatcher.effects()[0][1] == payload("j1")


def test_a_first_attempt_success_does_not_advance_the_completion_time():
    dispatcher = Dispatcher(config(), Sink())
    result = dispatcher.submit("j1", payload("j1"), 100.0)
    assert result["completed_at"] == 100.0


def test_retrying_advances_the_completion_time_without_sleeping():
    """Backoff is computed into the result, never slept.

    The suite must stay fast and reproducible; wall time must not pass.
    """
    sink = Sink({"j1": transient_then_ok(2)})
    dispatcher = Dispatcher(config(), sink)
    result = dispatcher.submit("j1", payload("j1"), 100.0)
    assert result["completed_at"] > 100.0
