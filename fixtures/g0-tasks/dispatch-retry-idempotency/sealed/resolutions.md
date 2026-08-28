<!--
  Title: Minibench G0 task fixture — SEALED
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# SEALED — intended resolutions

**Never mount this directory in a participant capsule.** N15 §6.2 requires
participants to be unable to read sealed material; the acceptance suite,
the reference solution, and the distractors all live here too.

Task: `dispatch-retry-idempotency`, version 1.0.0.

## Injected contradiction 1 — retry everything and apply exactly once

**The conflict.** Operations requires that no job be lost and that every
failure be retried. Data Integrity requires that each `job_id`'s side effect
be applied at most once, and states that callers re-submit when they get no
acknowledgement. The seeded dispatcher retries every refusal and remembers
nothing, so a job whose side effect was committed but whose acknowledgement
was lost is applied a second time on re-submission.

The two requirements are jointly unsatisfiable as written, and neither half
of the fix is sufficient alone. Deduplication without classification still
burns the whole retry budget on jobs that cannot succeed. Classification
without deduplication still double-applies on re-submission.

**Intended resolution.** Both halves:

1. **Deduplicate by `job_id`.** Once a job's side effect has been applied,
   a further submission of the same `job_id` does not reach the sink. The
   dedupe ledger is keyed on the effect having been applied, not on the job
   having been seen — a failed job has no side effect to protect and must
   be reachable again, or the ledger becomes a new way to lose work.
2. **Retry transient refusals only.** A `PermanentError` is terminal on the
   first attempt. This half is not derivable from the stakeholder documents:
   it rests on the telemetry, which shows that no sampled permanent failure
   ever succeeded at any retry depth (deepest observed chain: 64 attempts,
   64 rejections) and that all six retry-storm incidents in the window were
   permanent failures being retried. Operations' "retry until it works"
   holds for the 88.6% of refusals where trying again is what works.

**What counts as matching.** The run must (a) prevent a re-submitted
`job_id` from applying a second side effect, (b) stop retrying permanent
rejections after one attempt, citing the telemetry rather than taste, and
(c) keep retrying transient refusals so recoverable work is not lost.

**Non-matching resolutions.** Deduplicating and retrying everything (the
literal Operations reading — never loses work, storms on rejections);
refusing to retry so exactly-once is trivially safe (the literal Data
Integrity reading — loses 88.6% of recoverable work); making the sink
idempotent, which is out of scope because the dispatcher does not own it;
declaring duplicates acceptable at low volume, which Data Integrity
explicitly ruled out.

## Injected contradiction 2 — `max_retries` cannot keep its plain meaning

**The conflict.** Support requires that `max_retries` and
`retry_backoff_seconds` keep working unchanged and keep meaning "how many
times a failing job will be tried again". Resolution 1 makes that untrue for
permanent rejections, which are now tried exactly once regardless of how
`max_retries` is set.

**Intended resolution.** Keep both documented keys authoritative and
unrenamed; any new input — a dedupe window, a classification override — is
optional with a default, so an existing two-key file loads and behaves
sensibly. Then state in the decision record exactly which failures
`max_retries` still governs: transient refusals, where the bound is
`max_retries` retries after the first attempt, for a total of
`max_retries + 1` attempts; and which it no longer governs: permanent
rejections, which are terminal after one attempt at any setting. Support
asked for it in those terms so the answer to "what is `max_retries` set to"
stays useful in the runbook.

**What counts as matching.** The run must (a) preserve the two-key
configuration contract with no renames and no new required keys, (b) keep
`max_retries` responsive to tuning for the failures it still governs, and
(c) record the narrowed scope explicitly, naming the failure class, not
merely noting that "some failures behave differently".

**Non-matching resolutions.** Renaming a key; requiring a new key; adding
`max_permanent_retries` as a required input; silently narrowing the meaning
without recording it; "resolving" the conflict by asserting `max_retries`
still means what it used to.

## Hard constraints (mechanically checked)

1. **Configuration contract.** A file containing only `max_retries` and
   `retry_backoff_seconds` loads and produces working behaviour.
2. **Exactly-once.** For any `job_id`, the sink's side effect is applied at
   most once, across any number of submissions.
3. **No lost recoverable work.** A job that refuses transiently and then
   succeeds ends applied.
4. **Standard library only.** No third-party imports.
5. **Injected time.** `submit(job_id, payload, now)` keeps its signature,
   reads no clock, and never sleeps — backoff is computed into the returned
   completion time.

Constraints 2 and 3 are the interacting pair: the seeded implementation
satisfies 3 and violates 2; a dispatcher that never retries satisfies 2 and
violates 3.

Permanent-rejection behaviour is deliberately **not** a hard constraint. It
encodes the sealed resolution rather than a mechanical property of the
system, so it lives in `test_resolution.py`, where a run that reasons its
way to a different but defensible retry policy fails for the right reason.

## Evidence-integration point

`telemetry/failure_classes.json` is the only source for three facts: which
failures are recoverable and in what proportion, that permanent failures
never succeed at any retry depth, and how late a re-submission arrives after
a lost acknowledgement. The stakeholder documents state none of them —
Operations asserts the opposite of the second.

The dedupe window is where the third fact bites. The natural
document-derived window is the span over which a job could still be
retrying, `max_retries * retry_backoff_seconds`: 8 seconds under the shipped
configuration. The telemetry shows completion latency running to a p99 of 47
seconds and re-submission after acknowledgement loss observed as late as 104
seconds. A window sized from the documents has forgotten the job before the
re-submission arrives, and double-applies. The evidence tests probe at 47s,
96s, and 104s.
