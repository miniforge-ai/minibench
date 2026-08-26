"""SEALED distractor — the literal Operations reading.

"No job may be lost. If a job fails, retry it." Taken at its word: every
refusal is retried up to `max_retries`, whatever kind of refusal it is. It
does deduplicate by job id, so it never double-applies and it never loses
recoverable work — which is exactly why it is a plausible answer.

What it misses is the half of the resolution that only the telemetry
supplies: permanent rejections never succeed, and retrying them is what
caused every retry-storm incident. This dispatcher spends the full retry
budget on jobs that cannot be recovered.

The acceptance suite MUST fail this.
"""

from src.sink import PermanentError, TransientError

APPLIED = "applied"
FAILED = "failed"

DEFAULT_DEDUPE_WINDOW_SECONDS = 300


class Dispatcher:
    def __init__(self, config, sink):
        self.max_retries = config["max_retries"]
        self.retry_backoff_seconds = config["retry_backoff_seconds"]
        self.dedupe_window_seconds = config.get(
            "dedupe_window_seconds", DEFAULT_DEDUPE_WINDOW_SECONDS
        )
        self._sink = sink
        self._applied_at = {}

    def _expire(self, now):
        expired = [
            job_id
            for job_id, completed_at in self._applied_at.items()
            if now - completed_at > self.dedupe_window_seconds
        ]
        for job_id in expired:
            del self._applied_at[job_id]

    def submit(self, job_id, payload, now):
        self._expire(now)
        if job_id in self._applied_at:
            return _result(job_id, APPLIED, 0, self._applied_at[job_id])

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
            self._applied_at[job_id] = clock
            return _result(job_id, APPLIED, attempts, clock)

    def effects(self):
        return self._sink.applied()


def _result(job_id, status, attempts, completed_at):
    return {
        "job_id": job_id,
        "status": status,
        "attempts": attempts,
        "completed_at": completed_at,
    }
