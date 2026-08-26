"""SEALED acceptance-suite helpers. Never mounted in a participant capsule."""

import pytest

from src.config import parse_dispatch_config
from src.dispatcher import Dispatcher
from src.sink import Sink, always_permanent, transient_then_ok

MAX_RETRIES = 4
RETRY_BACKOFF_SECONDS = 2

# telemetry/failure_classes.json — classes[].share_of_failures
TRANSIENT_SHARE = 0.886
PERMANENT_SHARE = 0.114
# telemetry/failure_classes.json — job_completion_latency_seconds.p99
P99_COMPLETION_LATENCY_SECONDS = 47.0
# telemetry/failure_classes.json — acknowledgement_loss.resubmission_delay_seconds
P99_RESUBMISSION_DELAY_SECONDS = 96.0
# telemetry/failure_classes.json — classes[transient].attempts_to_success.max
OBSERVED_TRANSIENT_DEPTH = 2


def payload_for(job_id):
    return {"job_id": job_id, "op": "post_ledger_entry", "amount_cents": 1200}


def build(sink, **overrides):
    """A dispatcher under the shipped two-key configuration unless overridden."""
    config = {
        "max_retries": MAX_RETRIES,
        "retry_backoff_seconds": RETRY_BACKOFF_SECONDS,
    }
    config.update(overrides)
    return Dispatcher(config, sink)


def dispatch_all(dispatcher, job_ids, now=0.0):
    """Submit each job once, in the given order. Returns job_id -> result."""
    return {
        job_id: dispatcher.submit(job_id, payload_for(job_id), now)
        for job_id in job_ids
    }


def effect_counts(sink):
    """job_id -> number of times its side effect was applied."""
    counts = {}
    for job_id, _ in sink.applied():
        counts[job_id] = counts.get(job_id, 0) + 1
    return counts


def duplicated(sink):
    """job_id -> count, for every job applied more than once."""
    return {
        job_id: count
        for job_id, count in effect_counts(sink).items()
        if count > 1
    }


def telemetry_shaped_script(total=1000):
    """Scripts for a workload at the observed transient/permanent split.

    Returns (script, transient_ids, permanent_ids). Transient jobs refuse
    twice — the observed maximum attempts-to-success minus the successful
    attempt — then apply. Permanent jobs are rejected on every attempt, as
    the telemetry says they are at any retry depth.
    """
    permanent_count = int(round(total * PERMANENT_SHARE))
    transient_count = total - permanent_count
    transient_ids = ["t{}".format(index) for index in range(transient_count)]
    permanent_ids = ["p{}".format(index) for index in range(permanent_count)]
    script = {}
    for job_id in transient_ids:
        script[job_id] = transient_then_ok(OBSERVED_TRANSIENT_DEPTH)
    for job_id in permanent_ids:
        script[job_id] = always_permanent()
    return script, transient_ids, permanent_ids


def interleave(first, second):
    """Round-robin two id lists.

    Jobs from both classes arrive mixed together in one queue. Draining one
    class before the other would make the result a function of arrival order
    rather than of the dispatch policy.
    """
    mixed = []
    for index in range(max(len(first), len(second))):
        if index < len(first):
            mixed.append(first[index])
        if index < len(second):
            mixed.append(second[index])
    return mixed


@pytest.fixture
def two_key_config_text():
    return "max_retries: {}\nretry_backoff_seconds: {}\n".format(
        MAX_RETRIES, RETRY_BACKOFF_SECONDS
    )


@pytest.fixture
def parsed_two_key(two_key_config_text):
    return parse_dispatch_config(two_key_config_text)


@pytest.fixture
def healthy_sink():
    return Sink()
