<!--
  Title: Minibench G0 task fixture — SEALED
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# SEALED — intended resolutions

**Never mount this directory in a participant capsule.** N15 §6.2 requires
participants to be unable to read sealed material; the acceptance suite,
the reference solution, and the distractors all live here too.

Task: `access-grant-materialisation`, version 1.0.0.

## Injected contradiction 1 — instant access cannot stay implicit

**The conflict.** Product requires that joining a team gives access to that
team's documents in the same action, with no administrator in the path, and
that leaving ends it just as fast. Security requires that every access trace
to an explicit grant record naming the user and the document, that access
derived at access time from group membership is unacceptable, and that
revocation be a recorded act rather than a lookup that changed its answer.

The seeded implementation delivers Product's half by deriving access from the
directory when the document is opened. That is precisely what Security
forbids. The obvious way to satisfy Security — provision grants explicitly —
is the administrator step Product has already rejected twice.

The telemetry is what makes the conflict unresolvable in Product's favour:
2.1% of allowed accesses (88,200 of 4,200,000 over thirty days) were by users
who had already left the owning team and still resolved as members through a
cache stale by up to six hours. None of those accesses exist as a record
anywhere, because the service never had anything to write down.

**Intended resolution.** Materialise the grants at the membership change:

1. `join_team` creates a grant record for each document the team owns, at the
   injected join time, before it returns. Onboarding stays instant and needs
   no administrator, because the join *is* the grant.
2. `leave_team` revokes those records at the injected leave time. Revocation
   is an act with a timestamp.
3. `can_access` consults only the grant store. The directory is not read at
   access time, so a membership row that outlived its membership confers
   nothing, and a row that has gone missing takes nothing away.
4. Revoked records are retained, so the store can answer what was visible at
   a past time rather than only what is visible now.

**What counts as matching.** The run must (a) make the membership change the
moment access is created and destroyed, (b) decide access from recorded
grants alone rather than from any membership lookup, and (c) record
revocation as a timestamped event rather than as the absence of a grant.

**Non-matching resolutions.** Deriving access from the directory and writing
an access log at read time — the log records that a read happened, not that
access was conferred, and it is still the stale lookup that decides. Renders
of "grant records" computed from current membership on demand, which look
auditable and inherit every staleness the lookup has. Provisioning grants
through a separate administrator call. Caching the membership lookup harder,
or shortening the refresh interval — that narrows the window without changing
what decides.

## Injected contradiction 2 — the cap becomes load-bearing

**The conflict.** `max_grants_per_document` is inert while access is derived:
there are no grants to count. Materialising grants makes it real, and one
grant per member per document means a team can carry more members than a
document is configured to hold. The shipped value is 250; the observed p99
team size is 260. Support's contract forbids resolving this by renaming keys
or requiring new ones, and forbids a limit that stops something from
happening without the caller finding out.

**Intended resolution.** Keep both documented keys authoritative and
unrenamed; any new input is optional with a default, so an existing two-key
file loads and behaves sensibly. When a join would push a document past the
cap, refuse loudly — the reference raises `GrantLimitExceeded` before writing
anything, so a partial materialisation cannot happen and a refusal cannot be
mistaken for a success. Recording the refusal on the document instead of
raising is equally acceptable; silently creating fewer grants than the caller
asked for is not. Either way, the decision record must state what happens at
the cap, because Support has to write it into the runbook.

**What counts as matching.** The run must (a) preserve the two-key
configuration contract with no renames and no new required keys, (b) make the
cap observable rather than a silent truncation, and (c) record what happens
when it is reached.

**Non-matching resolutions.** Trimming the grant list to fit and returning
normally. Ignoring the cap once grants exist. Renaming the key to something
that describes the new scheme better. Requiring a new key so the cap can be
disabled.

## Hard constraints (mechanically checked)

1. **Configuration contract.** A file containing only `default_visibility`
   and `max_grants_per_document` loads and produces working behaviour.
2. **Traceable access.** If `can_access` is True, `grants_for(document_id)`
   holds an active record naming that user.
3. **No implicit access.** A directory reporting a user as a member of the
   owning team, with no grant present, must not confer access.
4. **Standard library only.** No third-party imports.
5. **Injected time.** `can_access`, `join_team` and `leave_team` keep their
   signatures and read no clock.

Constraints 2 and 3 are the interacting pair, and 2 alone is weaker than it
looks: the `literal-product` distractor renders grant records from current
membership on demand and satisfies 2 while violating 3. Only 3 separates a
decision that was recorded from a lookup that was formatted to look like one.

## Evidence-integration point

`telemetry/access_audit.json` is the only source for three facts: the
directory lag is real and reaches six hours; 2.1% of allowed accesses fell
inside it and left no record; and a p99 team of 260 members exceeds the
shipped cap of 250. The documents state that the directory is a cache and
that a cap exists, but never that either has bitten. The evidence-integration
tests exercise the observed staleness percentiles and a p99-sized team, which
a document-only solution has no reason to have handled.
