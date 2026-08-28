"""Background job dispatch for the Stonecrop workflow tool.

SYNTHETIC FIXTURE — this service, its customers, and its telemetry are
invented for benchmark use. Nothing here describes a real system.

The dispatcher hands a job to the downstream sink and decides what to do
when the sink refuses it. Time is injected into `submit` rather than read
from a clock, and retry backoff is computed into the returned completion
time rather than slept, so dispatch behaviour is reproducible.
"""

from src.sink import PermanentError, TransientError

APPLIED = "applied"
FAILED = "failed"


class Dispatcher:
    """Retrying dispatcher.

    The current implementation treats every refusal the same way: try again,
    up to `max_retries` times, with a fixed backoff between attempts.
    """

    def __init__(self, config, sink):
        self.max_retries = config["max_retries"]
        self.retry_backoff_seconds = config["retry_backoff_seconds"]
        self._sink = sink

    def submit(self, job_id, payload, now):
        """Dispatch one job. Returns a result record."""
        attempts = 0
        clock = now
        while True:
            attempts += 1
            try:
                self._sink.apply(job_id, payload)
            except (TransientError, PermanentError):
                if attempts > self.max_retries:
                    return _result(job_id, FAILED, attempts, clock)
                clock = clock + self.retry_backoff_seconds
                continue
            return _result(job_id, APPLIED, attempts, clock)

    def effects(self):
        """The side effects the sink actually applied, in order."""
        return self._sink.applied()


def _result(job_id, status, attempts, completed_at):
    return {
        "job_id": job_id,
        "status": status,
        "attempts": attempts,
        "completed_at": completed_at,
    }
