<!--
  Title: Minibench G0 task fixture — synthetic stakeholder input
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# Support requirements — configuration stability

Filed by Support. Synthetic document, invented for benchmark use.

## Background

`dispatch.yaml` is edited by customers directly. Our runbooks, our
onboarding guide, and roughly three hundred customer-managed configuration
files all reference the same two keys:

```
max_retries: 4
retry_backoff_seconds: 2
```

Both are tuned per deployment. Customers with a slow sink raise
`retry_backoff_seconds`; customers with a strict queue budget lower
`max_retries`. The first question in our dispatch runbook is "what is
`max_retries` set to", and the answer is supposed to tell the responder how
many times a failing job will be tried.

## Requirements

1. **`max_retries` and `retry_backoff_seconds` must keep working.** A
   configuration file containing only those two keys must load and must
   produce sensible behaviour. We cannot ask customers to edit files we do
   not control.
2. **No renames.** If the new scheme needs additional inputs, they must be
   optional and must have defaults, so an existing file stays valid.
3. **`max_retries` must keep meaning what customers think it means.** They
   read it as "the number of times a failing job will be tried again". If
   the new behaviour makes that untrue for some failures, Support needs to
   know exactly which failures it still governs and which it no longer
   governs — put it in the decision record, in those terms, so we can write
   it into the runbook.
