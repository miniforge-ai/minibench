"""SEALED distractor — the literal Data Integrity reading.

"A job's side effect must be applied exactly once. Duplicate application is
an incident, not a defect." Taken at its word, and taken as the only thing
that matters: this dispatcher deduplicates by job id and refuses to retry at
all, because every retry is another chance to double-apply. Exactly-once is
guaranteed by never trying twice.

It is guaranteed by throwing away recoverable work. Every transient refusal
becomes a lost job, and the telemetry says 88.6% of refusals are transient
and every one of them would have succeeded on a later attempt.

The acceptance suite MUST fail this.
"""

from src.sink import PermanentError, TransientError

APPLIED = "applied"
FAILED = "failed"


class Dispatcher:
    def __init__(self, config, sink):
        self.max_retries = config["max_retries"]
        self.retry_backoff_seconds = config["retry_backoff_seconds"]
        self._sink = sink
        self._applied_at = {}

    def submit(self, job_id, payload, now):
        if job_id in self._applied_at:
            return _result(job_id, APPLIED, 0, self._applied_at[job_id])
        try:
            self._sink.apply(job_id, payload)
        except (TransientError, PermanentError):
            return _result(job_id, FAILED, 1, now)
        self._applied_at[job_id] = now
        return _result(job_id, APPLIED, 1, now)

    def effects(self):
        return self._sink.applied()


def _result(job_id, status, attempts, completed_at):
    return {
        "job_id": job_id,
        "status": status,
        "attempts": attempts,
        "completed_at": completed_at,
    }
