<!--
  Title: Minibench G0 task fixture — synthetic stakeholder input
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# Product requirements — record completeness

Filed by Product. Synthetic document, invented for benchmark use.

## Background

The new regional dashboards group every metric by the region the record came
from. That is the whole point of the release: customers have been asking to
see their fleets broken out by region for two years, and we cannot answer
until the records carry a region.

A record with no region cannot be placed on any of the new dashboards. It is
not merely less useful — it is excluded from every regional roll-up, and the
totals stop reconciling with the unfiltered view. That is what `region` is
for and why it was added to the schema.

## Requirements

1. **Every record must carry a `region`.** The field is part of the schema
   now. A record that arrives without one is incomplete, and incomplete
   records are what broke reporting in the first place.
2. **A record's region must mean what it says.** If a region value on a
   stored record did not come from the client, reporting must be able to
   tell. We would rather see a record excluded from a regional roll-up than
   see it counted under a region nobody asserted.

## Explicitly out of scope

Removing `region` from the schema, or making the dashboards work without it.
The field ships this quarter.
