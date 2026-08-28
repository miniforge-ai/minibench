<!--
  Title: Minibench G0 task fixture — SEALED
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# SEALED — intended resolutions

**Never mount this directory in a participant capsule.** N15 §6.2 requires
participants to be unable to read sealed material; the acceptance suite,
the reference solution, and the distractors all live here too.

Task: `rate-limiter-reconciliation`, version 1.0.0.

## Injected contradiction 1 — the guarantee cannot hold universally

**The conflict.** Product requires every user to be guaranteed 10 rps and
never starved. Platform requires that the gateway never admit more than
`global_rate` (800) in a window, and that capacity never sit idle while
requests are rejected.

With `global_rate: 800` and `per_user_rate: 10`, the guarantee is
satisfiable for at most 80 simultaneously active users. The telemetry shows
concurrent active users reaching 128, with 9.79% of windows at or above 80.
So the two requirements are jointly unsatisfiable as written, and the
conflict cannot be detected from the stakeholder documents alone — it
requires the telemetry.

**Intended resolution.** A hierarchical scheme:

1. Each active user is entitled to `min(per_user_rate, global_rate //
   active_users_this_window)` — the published rate when there is room for
   it, and an equal share of the ceiling when there is not.
2. Below that entitlement a user is never rejected while the window is under
   `global_rate` (fairness floor, work-conserving).
3. Above their entitlement a user may still be admitted from capacity other
   users are not using (borrowing), so idle capacity is never wasted.
4. The global ceiling is never exceeded, in any regime.

**What counts as matching.** The run must (a) identify that the 10 rps
guarantee degrades above 80 active users, (b) implement equal-share
degradation rather than either dropping the guarantee or oversubscribing the
ceiling, and (c) keep the scheme work-conserving.

**Non-matching resolutions.** Enforcing per-user reservations that can sum
past the ceiling; dropping the per-user floor and keeping a global-only
bucket; raising `global_rate` beyond the configured value; lowering the
published 10 rps figure (explicitly out of scope in the Product document).

## Injected contradiction 2 — `per_user_rate` cannot keep its plain meaning

**The conflict.** Support requires that `per_user_rate` keep meaning "the
rate I am entitled to", and that existing two-key configurations keep
working unchanged. Resolution 1 makes `per_user_rate` a *ceiling on* the
entitlement rather than the entitlement itself, because under saturation the
entitlement is the equal share.

**Intended resolution.** Keep the two documented keys authoritative and
unrenamed; any new input is optional with a default, so an existing file
loads and behaves sensibly. Then state the degradation condition explicitly
in the decision record — Support asked for the exact conditions so they can
write a runbook. `per_user_rate` becomes an upper bound on entitlement; the
effective entitlement is `min(per_user_rate, global_rate // active_users)`.

**What counts as matching.** The run must (a) preserve the two-key
configuration contract with no renames and no new required keys, and (b)
record the exact condition under which `per_user_rate` is not delivered —
naming the active-user threshold, not merely gesturing at "high load".

**Non-matching resolutions.** Renaming keys; requiring a new key; silently
changing the meaning of `per_user_rate` without recording the condition;
"resolving" the conflict by asserting the guarantee still always holds.

## Hard constraints (mechanically checked)

1. **Configuration contract.** A file containing only `global_rate` and
   `per_user_rate` loads and produces working behaviour.
2. **Global ceiling.** Admissions in any one-second window never exceed
   `global_rate`.
3. **Work conservation.** No request is rejected while the window is under
   `global_rate` and the sender is under its entitlement.
4. **Standard library only.** No third-party imports.
5. **Injected time.** `allow(user_id, now)` keeps its signature and reads no
   clock.

Constraints 2 and 3 are the interacting pair: the seeded implementation
satisfies 2 and violates 3 for quiet tenants; a naive per-user bucket
satisfies 3 and violates 2.

## Evidence-integration point

The active-user distribution in `telemetry/active_users.json` is the only
source for the fact that concurrent active users exceed 80. A solution
derived from the stakeholder documents alone will either assume the
guarantee always holds or pick an arbitrary degradation threshold. The
evidence-integration tests check behaviour at 128 active users — the
observed maximum — which a document-only solution has no reason to have
handled.
