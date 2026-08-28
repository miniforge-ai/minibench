<!--
  Title: Minibench G0 task fixture — synthetic stakeholder input
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# Operations requirements — job durability

Filed by Operations. Synthetic document, invented for benchmark use.

## Background

Stonecrop jobs are the tail end of work a person already did. When a job
disappears, the work disappears with it, and nobody finds out until a
customer asks where their result went. We have spent more on-call hours
reconstructing lost jobs than on anything else this quarter.

## Requirements

1. **No job may be lost.** A job that was accepted must end up applied.
   Silently dropping a job is the worst outcome the dispatcher can produce.
2. **If a job fails, retry it.** The sink is not always available. A refusal
   is not a verdict on the job, it is a statement about the moment, and the
   dispatcher's response to a refusal must be to try again.
3. **Retry until it works.** `max_retries` exists because the queue is
   finite, not because we want to give up. If we are running out of retries
   on real work, that is a capacity problem, not a reason to shed the job.

## Note

We are aware Data Integrity is worried about duplicates. A duplicate is a
row somebody can delete. A lost job is work that never happened and cannot
be recovered. If the two cannot both be had, durability wins.
