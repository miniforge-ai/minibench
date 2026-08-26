<!--
  Title: Minibench G0 task fixture — synthetic stakeholder input
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# Client requirements — release compatibility

Filed by the Client Platform team. Synthetic document, invented for
benchmark use.

## Background

The Quillon client is installed on customer infrastructure. We do not
control when it is upgraded; the customer does, on their own change-control
calendar. Our published support policy is the current release and the
previous two — today that is `v3`, `v2` and `v1`.

`v1` predates the regional work. The field does not exist in that build:
there is no configuration flag, no environment variable and no server-side
change that will make a `v1` client emit a `region`. Waiting is not a plan
either — a customer who has not upgraded in a year is not going to upgrade
because we asked twice.

## Requirements

1. **A `v1` client must keep ingesting successfully.** A `v1` record is a
   valid record. An endpoint that rejects it has taken those customers'
   telemetry offline, and that is an outage on our side, not theirs.
2. **No silent drops.** If a record is accepted, it is stored and it is
   retrievable. "Accepted and then discarded" is worse than a rejection,
   because nobody finds out until the customer asks where their data went.
3. **Support policy is the boundary.** We will not carry `v1` forever. When
   we retire it, we need a stated condition for doing so that we can defend
   to the customers still on it — not a date somebody picked in a meeting.
