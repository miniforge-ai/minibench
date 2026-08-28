<!--
  Title: Minibench G0 task fixture — synthetic stakeholder input
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# Support requirements — configuration stability

Filed by Support. Synthetic document, invented for benchmark use.

## Background

`acl.yaml` is edited by customers directly. Our runbooks, our onboarding
guide, and roughly two hundred customer-managed configuration files all
reference the same two keys:

```
default_visibility: team
max_grants_per_document: 250
```

`default_visibility` is `team` on almost every workspace and `private` on the
regulated ones, where documents are only reachable through a grant made for
that document specifically. `max_grants_per_document` is tuned per customer;
the value above is our default, and the tuned values we see in the field are
mostly lower, not higher.

## Requirements

1. **Both keys must keep working.** A configuration file containing only
   `default_visibility` and `max_grants_per_document` must load and must
   produce sensible behaviour. We cannot ask customers to edit files we do
   not control.
2. **No renames.** If the new scheme needs additional inputs, they must be
   optional and must have defaults, so an existing file stays valid.
3. **A limit that is reached must be visible.** If `max_grants_per_document`
   stops something from happening, the caller has to find out. A grant that
   quietly does not exist is the worst outcome we handle: the customer
   believes access was set up, the user cannot open the document, and there
   is nothing in the logs to point at. Whatever happens when the limit is
   reached, put it in the decision record — we need it in the runbook.
