<!--
  Title: Minibench G0 task fixture — SEALED
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# SEALED — intended resolutions

**Never mount this directory in a participant capsule.** N15 §6.2 requires
participants to be unable to read sealed material; the acceptance suite, the
reference solution, and the distractors all live here too.

Task: `schema-migration-compatibility`, version 1.0.0.

## Injected contradiction 1 — the new field cannot be required of everyone

**The conflict.** Product requires every record to carry a `region`, and
requires that a stored region mean what it says. Clients requires that a
`v1` client keep ingesting successfully, and states that no `v1` build can
emit a `region` under any configuration.

The two are jointly unsatisfiable as written for as long as `v1` traffic
exists. How long that is cannot be read off the documents — Clients says
rollout is slow without saying how slow. The telemetry says `v1` is 3.2% of
records and falling by 0.15 percentage points per week, roughly 18 weeks to
reach 0.5%, with 41% of that traffic in two accounts on multi-year
change-control cycles. So the conflict is live for months, not weeks, and
the resolution has to be a durable scheme rather than a short bridge.

**Intended resolution.**

1. Validation is per client version rather than global. A record from a
   `v1` client is validated against the pre-region schema and accepted
   without a `region`; a record from a region-capable client (`v2`, `v3`) is
   required to carry one, so Product's requirement is enforced everywhere it
   can be.
2. An accepted record that carried no `region` is stored with an explicit,
   stable sentinel value — `"unknown"` in the reference — never dropped and
   never left absent. Downstream reads a region on every record.
3. The sentinel is recorded together with its provenance, so a record whose
   region was never sent is distinguishable from one a client declared as
   `"unknown"`. The reference stores `region_source` and `client_version`
   alongside the record; any mechanism that keeps the two cases apart
   qualifies.
4. Retirement of the pre-region path is gated on the telemetry-observed
   `v1` share falling below a threshold — `can_deprecate_v1(observed_share)`
   — not on a fixed date and not on a count of releases since `v1` shipped.
   The observed decline is what makes a date indefensible: any date chosen
   this quarter either lands while `v1` is still carrying real traffic or
   sits so far out that it decides nothing.

**What counts as matching.** The run must (a) accept `v1` records lacking
`region` while still requiring it from clients that can send it, (b) store
an explicit sentinel with enough provenance to distinguish it from a
client-declared value, and (c) express retirement as a condition on the
observed share rather than on the calendar.

**Non-matching resolutions.** Requiring `region` of every client
(the seeded reading — takes `v1` customers' telemetry offline); making it
optional for every client (Product's requirement is then enforced against
nobody); accepting `v1` records and dropping them, or accepting them and
storing no region at all (pushes the case onto every downstream reader);
back-filling a plausible region — inferring it from the account or the
device id — and storing it as though the client had asserted it; picking a
sunset date for `v1`.

## Injected contradiction 2 — `schema_version` cannot keep its plain meaning

**The conflict.** Support requires that `schema_version` and
`reject_unknown_fields` keep working unchanged, with no renames and no new
required keys, and that `schema_version` keep meaning something explainable
— the runbook calls it "the schema your records are validated against".
Resolution 1 makes that false: there is no longer one schema every record is
held to.

**Intended resolution.** Keep both documented keys authoritative and
unrenamed; any new input is optional with a default, so an existing two-key
file loads and behaves sensibly. `schema_version` becomes the newest schema
generation the endpoint knows, and a record is validated against
`min(client_generation, schema_version)` — it selects a validation path
rather than naming a single schema. Then state that new meaning in the
decision record, because Support asked for it in time to rewrite the page.

**What counts as matching.** The run must (a) preserve the two-key
configuration contract with no renames and no new required keys, both keys
still doing what they did, and (b) record what `schema_version` selects now
and what changing it does — not merely note that its meaning "evolved".

**Non-matching resolutions.** Renaming keys; requiring a new key; adding a
per-client override that customers must set; silently redefining
`schema_version` without recording the new meaning; "resolving" the conflict
by asserting the key still names one schema.

## Hard constraints (mechanically checked)

1. **Configuration contract.** A file containing only `schema_version` and
   `reject_unknown_fields` loads and produces working behaviour.
2. **No `v1` breakage.** A valid `v1` record, carrying no `region`, is
   accepted and never raises — including when a region-capable record
   preceded it.
3. **No silent data loss.** Every accepted record is retrievable from
   `stored()` with every client-sent field preserved; the stored record
   carries an explicit region; and a record whose region was never sent is
   distinguishable from one a client declared as `"unknown"`.
4. **Standard library only.** No third-party imports.
5. **No clock reads.** `ingest(record, client_version)` keeps its signature
   and nothing in `src/` imports or reads a clock, so retirement cannot be
   date-driven.

Constraints 2 and 3 are the interacting pair: the seeded implementation
violates 2 and satisfies 3 for the records it accepts; making `region`
optional for every client satisfies 2 and violates 3, because an absent
region and a client-declared `"unknown"` become the same stored record.

## Evidence-integration point

`telemetry/client_versions.json` is the only source for the size and
trajectory of the `v1` population. The stakeholder documents establish that
`v1` exists and will one day be retired; nothing in them says it is 3.2% of
records, or that the share falls by only 0.15 percentage points per week, or
that two accounts hold 41% of it. A run working from the documents alone has
no basis for preferring a share-based retirement condition over a date, and
no way to check whether a threshold it picked sits inside the observed
range. The evidence-integration tests drive the retirement predicate with
the observed 3.2% share — which must not open the gate — and with a
hypothetical 0.05% residual, which must.
