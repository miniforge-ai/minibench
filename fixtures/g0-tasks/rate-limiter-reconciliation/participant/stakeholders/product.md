<!--
  Title: Minibench G0 task fixture — synthetic stakeholder input
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# Product requirements — admission fairness

Filed by Product. Synthetic document, invented for benchmark use.

## Background

Our largest tenants generate request bursts that consume the entire gateway
allowance. When that happens, every other tenant sees rejections despite
sending modest traffic. Two customers have escalated this month.

## Requirement

1. **Every user is guaranteed 10 requests per second.** A user sending at or
   below 10 rps must not be rejected. This is the number we have put in
   writing, and it is the number Support quotes.
2. **No user may be starved by another user's traffic.** A single heavy
   sender must not be able to consume the allowance that a quiet tenant
   would otherwise have been able to use.

## Explicitly out of scope

Changing the 10 rps figure. It is published.
