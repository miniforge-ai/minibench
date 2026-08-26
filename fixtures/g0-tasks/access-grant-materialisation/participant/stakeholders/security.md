<!--
  Title: Minibench G0 task fixture — synthetic stakeholder input
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# Security requirements — auditable access

Filed by Security. Synthetic document, invented for benchmark use.

## Background

We are asked, in writing, roughly once a quarter: *who could see this
document last March, and on what basis?* Today we cannot answer. Access is
not recorded anywhere — it is worked out at the moment someone opens a
document, from whatever the membership lookup happens to say at that moment.
There is nothing to produce, because nothing was ever written down.

Answering that question is a compliance requirement, not a preference.

## Requirements

1. **Every access decision must trace to an explicit grant record.** If a
   user can open a document, there must be a record naming that user and that
   document, created before the access and still in force at the time of it.
2. **Implicit access is not acceptable.** Access derived at access time from
   group membership is unauditable: it leaves no trace of what was visible,
   to whom, or when. A membership lookup is not a grant.
3. **Revocation must be an act, not a side effect.** Access must end because
   something recorded that it ended, at a recorded time. It must not end
   merely because a lookup started returning a different answer — and it must
   not survive because a lookup is still returning the old one.

## Note

We are aware Product does not want an administrator in the path. We are not
asking for one. We are asking that whatever confers access writes down that
it did.
