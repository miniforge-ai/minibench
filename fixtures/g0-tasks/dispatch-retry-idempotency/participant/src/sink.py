"""The downstream side-effect sink, and its error types.

SYNTHETIC FIXTURE — invented for benchmark use. In the real Stonecrop
deployment this is a network call to another service; here it is an
in-process double so dispatch behaviour can be exercised deterministically.

**This file is given, not yours to change.** It stands in for a dependency
the dispatcher does not own. Its side effects are not idempotent: calling
`apply` twice for the same `job_id` records the effect twice.

The sink distinguishes two kinds of refusal:

1. `TransientError` — the sink was momentarily unable to accept the job.
2. `PermanentError` — the sink rejected the job itself.

A sink can be scripted so a test can decide, per job id, which refusals it
raises and in what order.
"""

OK = "ok"
TRANSIENT = "transient"
PERMANENT = "permanent"


class SinkError(Exception):
    """Base class for refusals raised by the sink."""


class TransientError(SinkError):
    """The sink could not accept the job at this moment."""


class PermanentError(SinkError):
    """The sink rejected the job itself."""


class Script:
    """The outcome sequence the sink produces for one job id.

    `sequence` is consumed one outcome per `apply` call. Once it is
    exhausted every further call produces `then`.
    """

    def __init__(self, sequence=(), then=OK):
        self._sequence = list(sequence)
        self._then = then

    def next_outcome(self):
        if self._sequence:
            return self._sequence.pop(0)
        return self._then


def succeeds():
    """Applies on the first attempt."""
    return Script()


def transient_then_ok(count):
    """Refuses transiently `count` times, then applies."""
    return Script([TRANSIENT] * count)


def always_transient():
    """Refuses transiently on every attempt, forever."""
    return Script(then=TRANSIENT)


def always_permanent():
    """Rejects the job on every attempt, forever. Retrying cannot help."""
    return Script(then=PERMANENT)


class Sink:
    """Records applied side effects and refusals, per the supplied script.

    `script` maps a job id to a `Script` (or to a plain list of outcomes,
    which is treated as a sequence followed by success). Job ids with no
    entry apply on the first attempt.
    """

    def __init__(self, script=None):
        self._script = {}
        for job_id, outcomes in (script or {}).items():
            self._script[job_id] = (
                outcomes if isinstance(outcomes, Script) else Script(outcomes)
            )
        self._applied = []
        self._attempts = {}

    def apply(self, job_id, payload):
        """Apply the job's side effect, or refuse.

        Raises `TransientError` or `PermanentError` per the script. On
        success the effect is appended to the applied log — twice, if
        called twice.
        """
        self._attempts[job_id] = self._attempts.get(job_id, 0) + 1
        scripted = self._script.get(job_id)
        outcome = scripted.next_outcome() if scripted is not None else OK
        if outcome == TRANSIENT:
            raise TransientError("sink unavailable for job {!r}".format(job_id))
        if outcome == PERMANENT:
            raise PermanentError("sink rejected job {!r}".format(job_id))
        self._applied.append((job_id, payload))
        return {"job_id": job_id, "accepted": True}

    def applied(self):
        """Every side effect applied, in order, as (job_id, payload)."""
        return list(self._applied)

    def effect_count(self, job_id):
        """How many times this job's side effect was applied."""
        return sum(1 for applied_id, _ in self._applied if applied_id == job_id)

    def attempts(self, job_id):
        """How many times the dispatcher called `apply` for this job."""
        return self._attempts.get(job_id, 0)

    def total_attempts(self):
        """How many times `apply` was called for any job."""
        return sum(self._attempts.values())
