<!--
  Title: Minibench G0 task fixture — synthetic stakeholder input
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# Platform requirements — capacity protection

Filed by Platform. Synthetic document, invented for benchmark use.

## Background

The gateway fronts a fixed pool of upstream capacity. Past the configured
ceiling the upstreams degrade for everyone, not just the sender who pushed
us over it. The ceiling is not advisory.

## Requirements

1. **The gateway must never admit more than `global_rate` requests in any
   one-second window.** This is a hard safety limit. Exceeding it is an
   incident.
2. **Capacity must not sit idle while requests are being rejected.** We are
   paying for the upstream pool whether or not it is used. If the gateway
   rejects a request while the window still has room under `global_rate`,
   that is a defect.

## Note

We are aware Product wants a per-user guarantee. Whatever scheme is chosen,
requirement 1 is not negotiable — a reservation scheme that can oversubscribe
the ceiling is not acceptable, however good its fairness properties.
