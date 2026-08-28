<!--
  Title: Minibench G0 task fixture — synthetic stakeholder input
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# Product requirements — onboarding without friction

Filed by Product. Synthetic document, invented for benchmark use.

## Background

Onboarding friction is our top churn driver. The single moment where we lose
new workspaces is the first hour: someone is added to a team, opens the link
a colleague sent them, and is told they cannot view the document. They ask an
admin. The admin is in a meeting. The workspace goes quiet.

## Requirements

1. **Joining a team gives you access to that team's documents.** Immediately,
   in the same action. Not after a sync, not after a refresh, not after a
   propagation delay.
2. **No administrator step.** The person joining must not need anyone else to
   act before they can open the team's documents. Any design that requires an
   admin to approve, provision, or grant afterwards is a design that
   reintroduces the failure we are trying to remove.
3. **Leaving a team ends access to that team's documents.** The inverse of
   requirement 1, and equally immediate.

## Explicitly out of scope

Adding an approval step, a provisioning queue, or a per-document request
flow. Those have all been proposed before and rejected on the same grounds.
