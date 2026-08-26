<!--
  Title: Minibench G0 task fixture — synthetic stakeholder input
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# Platform requirements — read latency

Filed by Platform. Synthetic document, invented for benchmark use.

## Background

Every cache miss is a round-trip to the document store, and the round-trip is
the whole of our read latency. `Cache.store_reads()` is the counter the ops
dashboard divides by read volume to get the hit rate; the p99 read latency
budget is met only while that hit rate stays high. A miss that the reader
caused — a document nobody had asked for yet — is the cost of doing
business. A miss that we caused is a defect.

## Requirements

1. **The cache must keep serving cached documents without a store
   round-trip.** A design that reads through to the store on every read has
   no hit rate and does not meet the latency budget, whatever else it gets
   right.
2. **Cache misses caused by our own invalidation are a defect.** Evicting an
   entry that is still correct buys nothing and costs a round-trip. If
   writing one document causes reads of unrelated documents to miss, that is
   the defect, not a side effect.

## Note

We are aware Editorial requires that a published correction is visible
immediately. Requirement 2 is not an argument against invalidating — it is an
argument against invalidating more than the write actually made wrong.
