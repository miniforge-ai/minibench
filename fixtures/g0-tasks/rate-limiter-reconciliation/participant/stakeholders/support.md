<!--
  Title: Minibench G0 task fixture — synthetic stakeholder input
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# Support requirements — configuration stability

Filed by Support. Synthetic document, invented for benchmark use.

## Background

`limits.yaml` is edited by customers directly. Our runbooks, our onboarding
guide, and roughly two hundred customer-managed configuration files all
reference the same two keys:

```
global_rate: 800
per_user_rate: 10
```

## Requirements

1. **`global_rate` and `per_user_rate` must keep working.** A configuration
   file containing only those two keys must load and must produce sensible
   behaviour. We cannot ask customers to edit files we do not control.
2. **No renames.** If the new scheme needs additional inputs, they must be
   optional and must have defaults, so an existing file stays valid.
3. **`per_user_rate` must keep meaning what customers think it means.** They
   read it as "the rate I am entitled to". If the new behaviour makes that
   untrue in some conditions, Support needs to know the exact conditions so
   we can write them into the runbook — put it in the decision record.
