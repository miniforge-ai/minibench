<!--
  Title: Minibench G0 task fixture — synthetic stakeholder input
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# Data Integrity requirements — exactly-once application

Filed by Data Integrity. Synthetic document, invented for benchmark use.

## Background

Every job the dispatcher hands to the sink mutates downstream state:
postings, balances, entitlement grants. Those mutations are not idempotent
on the sink's side. Applying one twice does not produce the same state
twice — it produces wrong state, and the wrongness propagates into every
report built on top of it.

We have closed three incidents this quarter whose root cause was one job's
side effect applied more than once. Each took days to unpick, because by
the time anybody noticed, the duplicate had been aggregated into figures
customers had already seen.

## Requirements

1. **A job's side effect must be applied exactly once.** For any given
   `job_id`, the sink must see at most one successful application, no matter
   how many times that job is submitted or re-submitted.
2. **Duplicate application is an incident, not a defect.** It is not
   something to trade off against latency or throughput. There is no volume
   of duplicates that is acceptable.
3. **Re-submission is normal and must be safe.** Callers re-submit when they
   do not get an acknowledgement. The dispatcher must treat a re-submitted
   `job_id` as the same job, not as a new one.

## Note

We are aware Operations wants failures retried. A retry that re-applies a
side effect the sink already committed is exactly the failure mode we are
describing. Whatever retry scheme is chosen, requirement 1 is not
negotiable.
