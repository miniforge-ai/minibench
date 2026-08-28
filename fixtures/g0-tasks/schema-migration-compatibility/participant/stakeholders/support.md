<!--
  Title: Minibench G0 task fixture — synthetic stakeholder input
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# Support requirements — configuration stability

Filed by Support. Synthetic document, invented for benchmark use.

## Background

`ingest.yaml` is edited by customers directly. Our runbooks, our onboarding
guide, and every customer-managed configuration file we have ever shipped
reference the same two keys:

```
schema_version: 3
reject_unknown_fields: false
```

Both are tuned per customer. Strict installations run
`reject_unknown_fields: true` and will notice immediately if that stops
biting. Several customers are pinned to an older `schema_version` on
purpose.

## Requirements

1. **`schema_version` and `reject_unknown_fields` must keep working.** A
   configuration file containing only those two keys must load and must
   produce sensible behaviour. We cannot ask customers to edit files we do
   not control.
2. **No renames.** If the new scheme needs additional inputs, they must be
   optional and must have defaults, so an existing file stays valid.
3. **`schema_version` must keep meaning something we can explain.** The
   runbook tells customers it names "the schema your records are validated
   against". If that stops being a single schema, Support needs the new
   meaning written down — what the key selects now, and what changing it
   does — so we can rewrite the page before the release, not after the
   first ticket.
