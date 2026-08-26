<!--
  Title: Minibench G0 task fixture — SEALED
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# SEALED — intended resolutions

**Never mount this directory in a participant capsule.** N15 §6.2 requires
participants to be unable to read sealed material; the acceptance suite, the
reference solution, and the distractors all live here too.

Task: `cache-invalidation-consistency`, version 1.0.0.

## Injected contradiction 1 — never-stale versus the hit rate

**The conflict.** Editorial requires that a reader is never served content
older than the latest publish, with no acceptable window and no timer-based
mechanism. Platform requires that the cache keep serving without store
round-trips, and states that a miss caused by our own invalidation is a
defect. The seeded implementation satisfies Editorial exactly — it drops
every entry on every write — and by doing so manufactures the misses Platform
calls a defect.

The documents do not say how expensive the flush is. That comes only from
`telemetry/publish_patterns.json`: 94.2% of documents are never republished
after first publish, 61% of reads target that group, and the 0.8% hot set
carries 71% of all republishes. So a global flush is throwing away an
overwhelmingly cold-but-still-correct cache to fix one document, over and
over, at a mean of 12 republishes per hot document per hour.

**Intended resolution.** Per-key invalidation:

1. `publish` writes through to the store and invalidates exactly the key it
   wrote. Nothing else is touched — the write did not make anything else
   wrong.
2. Never-stale is therefore absolute for content this cache wrote, and it is
   absolute immediately, not within a TTL window.
3. Every other entry keeps serving from cache, so a republish costs one
   subsequent round-trip rather than a corpus-wide refill.

**What counts as matching.** The run must (a) invalidate on write rather than
rely on expiry, (b) scope the invalidation to the written key rather than the
whole cache, and (c) leave the cache serving unrelated keys without a store
round-trip.

**Non-matching resolutions.** Global flush on write (never stale, no hit
rate); TTL-only with no invalidation (hit rate, stale reads); shortening
`ttl_seconds` to make the staleness window small — that is the "average, not
a guarantee" Editorial explicitly rejects; disabling the cache, or reading
through to the store on every read, which satisfies freshness by having no
cache at all and fails Platform's first requirement.

## Injected contradiction 2 — `ttl_seconds` stops being the freshness mechanism

**The conflict.** Support requires that `ttl_seconds` and `max_entries` keep
working with no renames, and that `ttl_seconds` keep meaning what customers
think it means — "how stale a document can get". Resolution 1 changes what
the key does: freshness for content this cache wrote now comes from
invalidation, and the TTL no longer bounds staleness for those documents
because they are never stale at all.

**Intended resolution.** Keep both documented keys authoritative and
unrenamed; any new input is optional with a default, so an existing two-key
file loads and behaves sensibly. `ttl_seconds` is retained and still applied,
with a narrowed and stated meaning: it is the backstop for changes this cache
never saw — a peer node, a restore, an out-of-band edit — and it bounds how
long such a change can go unnoticed. It is no longer the mechanism by which a
publish becomes visible. `max_entries` is unchanged and must still bound the
cache.

**What counts as matching.** The run must (a) preserve the two-key
configuration contract with no renames and no new required keys, (b) keep
`ttl_seconds` actually applied rather than ignored or removed, and (c) record
the exact new meaning of `ttl_seconds` in the decision record — naming what
it now bounds, not merely saying it is "still there".

**Non-matching resolutions.** Renaming keys; requiring a new key; deleting
the TTL because invalidation supersedes it; leaving `ttl_seconds` parsed but
unused; claiming the key's meaning is unchanged.

## Hard constraints (mechanically checked)

1. **Configuration contract.** A file containing only `ttl_seconds` and
   `max_entries` loads and produces working behaviour.
2. **Never stale.** A read after a publish for that key never returns
   pre-publish content, at any `now`, in any order.
3. **No global flush.** Publishing one key does not evict unrelated keys —
   they still serve without a store round-trip.
4. **Standard library only.** No third-party imports.
5. **Injected time.** `read(key, now)` and `publish(key, content, now)` keep
   their signatures and read no clock.
6. **Bounded memory.** `max_entries` still bounds the number of retained
   entries.

Constraints 2 and 3 are the interacting pair: the seeded implementation
satisfies 2 and violates 3; a TTL-only cache satisfies 3 and violates 2.

## Evidence-integration point

`telemetry/publish_patterns.json` is the only source for the size of what the
flush destroys. The stakeholder documents establish that self-inflicted
misses are bad; only the telemetry says that 94.2% of the corpus is never
republished and takes 61% of reads, which is what makes the global flush
quantitatively indefensible rather than merely inelegant. The
evidence-integration tests run a workload built in exactly that split — 942
cold, 50 occasional, 8 hot out of 1000 — and compare the candidate's measured
hit rate against a global-flush cache driven through the identical workload.
