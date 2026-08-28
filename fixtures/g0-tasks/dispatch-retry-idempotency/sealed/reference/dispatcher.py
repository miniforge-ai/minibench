"""SEALED reference solution — deduplicated, failure-class-aware dispatch.

Solvability proof for N15 §6.3(1): this passes the full acceptance suite and
no LLM was involved in writing it.

Scheme, both halves required:

1. **Deduplicate by job id.** A job whose side effect has already been
   applied is not applied again, however many times it is re-submitted. This
   is what makes re-submission after a lost acknowledgement safe.
2. **Retry only transient refusals.** A `PermanentError` is terminal on the
   first attempt. The telemetry is the reason: no sampled permanent failure
   ever succeeded, at any retry depth, and every retry-storm incident in the
   window was a permanent failure being retried. Retrying them recovers no
   work — it only consumes the queue.

`max_retries` is unchanged, unrenamed, and still authoritative; it now
bounds transient retries only. `dedupe_window_seconds` is new, optional and
defaulted, so an existing two-key configuration keeps working. The default
of 300 seconds is set from the telemetry: it must cover the observed p99
completion latency (47s) and the observed acknowledgement-loss re-submission
delay (p99 96s, max 104s), with margin. A window sized to the retry span —
`max_retries * retry_backoff_seconds`, 8 seconds under the shipped
configuration — would have forgotten the job long before the re-submission
arrived.
"""

from src.sink import PermanentError, TransientError

APPLIED = "applied"
FAILED = "failed"

DEFAULT_DEDUPE_WINDOW_SECONDS = 300


class Dispatcher:
    """Dispatch with exactly-once application and classified retries."""

    def __init__(self, config, sink):
        self.max_retries = config["max_retries"]
        self.retry_backoff_seconds = config["retry_backoff_seconds"]
        self.dedupe_window_seconds = config.get(
            "dedupe_window_seconds", DEFAULT_DEDUPE_WINDOW_SECONDS
        )
        self._sink = sink
        self._applied_at = {}

    def _expire(self, now):
        """Forget jobs older than the dedupe window.

        The window is bounded so the ledger does not grow without limit; it
        is sized from the telemetry so it never expires an entry a
        re-submission could still arrive for.
        """
        expired = [
            job_id
            for job_id, completed_at in self._applied_at.items()
            if now - completed_at > self.dedupe_window_seconds
        ]
        for job_id in expired:
            del self._applied_at[job_id]

    def already_applied(self, job_id):
        """True when this job's side effect is on record as applied."""
        return job_id in self._applied_at

    def submit(self, job_id, payload, now):
        """Dispatch one job. Returns a result record."""
        self._expire(now)

        # Exactly-once: a re-submission of an applied job is a no-op that
        # reports the original outcome. The sink is never called again.
        if job_id in self._applied_at:
            return _result(job_id, APPLIED, 0, self._applied_at[job_id])

        attempts = 0
        clock = now
        while True:
            attempts += 1
            try:
                self._sink.apply(job_id, payload)
            except PermanentError:
                # Terminal on the first attempt. Retrying a rejection has
                # never recovered a job and is what saturates the queue.
                return _result(job_id, FAILED, attempts, clock)
            except TransientError:
                if attempts > self.max_retries:
                    return _result(job_id, FAILED, attempts, clock)
                clock = clock + self.retry_backoff_seconds
                continue
            self._applied_at[job_id] = clock
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
