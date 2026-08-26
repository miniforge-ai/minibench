<!--
  Title: Minibench G0 task fixture — synthetic stakeholder input
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# Support requirements — configuration stability

Filed by Support. Synthetic document, invented for benchmark use.

## Background

`cache.yaml` is edited by customers directly. Our runbooks, our onboarding
guide, and roughly three hundred customer-managed configuration files all
reference the same two keys:

```
ttl_seconds: 300
max_entries: 5000
```

Customers tune both. Large publishers raise `max_entries`; publishers with
slow-moving archives raise `ttl_seconds`.

## Requirements

1. **`ttl_seconds` and `max_entries` must keep working.** A configuration
   file containing only those two keys must load and must produce sensible
   behaviour. We cannot ask customers to edit files we do not control.
2. **No renames.** If the new scheme needs additional inputs, they must be
   optional and must have defaults, so an existing file stays valid.
3. **`max_entries` must keep bounding the cache.** It is the key customers
   reach for when a node runs out of memory, and the runbook step that tells
   them to lower it has to keep working.
4. **`ttl_seconds` must keep meaning what customers think it means.** They
   read it as "how stale a document can get". If the new behaviour makes that
   untrue in some conditions — or makes the key do something different from
   what it does today — Support needs the exact new meaning in writing so we
   can put it in the runbook. Put it in the decision record.
