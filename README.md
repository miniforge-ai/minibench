# Minibench

The workbench **app shell**: hosts and renders the cross-product state
validation that tenants (portfolio risk, career, miniforge, future
products) emit as `workbench_snapshot/v1`. Minibench is infrastructure —
the products plug in; it never links a product's domain crate.

```
   risk adapter ─┐
 career adapter ─┤
miniforge adapter┼──▶ workbench-contract ◀── Minibench (this repo)
   time adapter ─┘        (pure types)         data plane + kernel + UI
```

Minibench is the **third consumer** of `thesium-app-foundation` (after
risk and career), and exists partly to prove that foundation's
domain-neutral claim and pull its per-side rewiring to completion.

## What's here (first slice)

| Crate | Purpose |
|---|---|
| `crates/data-plane` | A `DataPlaneProvider` (from `thesium-app-foundation-data-plane`) serving `WorkbenchSnapshotV1` bodies over the foundation's five routes, plus a thin binary that binds loopback `:8789` |
| `crates/kernel` | The generic, tenant-agnostic kernel — reads only `workbench-contract`. First op: `summarize` (status roll-up + blocking-gate detection) |

The data plane speaks **JSON over HTTP/loopback** — the shipped Thesium
app-stack transport — reusing the foundation router verbatim. Snapshot
bodies are the typed `workbench-contract` shapes, re-applied at the
consuming edges (kernel, and later the Swift shell).

## Deferred to later slices

- **Swift UI shell** — the three view tiers (generic registry-driven
  views, the shared primitive kit, bespoke product view plugins).
- **Kernel** — regression diff, narration-packet assembly,
  registry-driven generic evaluators.
- **Live tenant feeds** — wiring real adapters (the career reference
  adapter lives in `workbench-contract/bindings/clojure`) in place of the
  sample fixture.

## Run

```bash
cargo test                       # kernel unit test + data-plane integration test
cargo run --bin minibench-data-plane
# then, in another shell:
curl -s http://127.0.0.1:8789/v1/snapshots/latest | jq .product
```

Loads snapshots from `MINIBENCH_SNAPSHOT_DIR` (default `fixtures`).
Requires `thesium-app-foundation` and `workbench-contract` checked out as
sibling directories (path dependencies).

## Ports

`:8787` risk · `:8788` career · **`:8789` minibench** — all loopback,
all the same foundation router.

## License

Proprietary. © 2025–2026 Christopher Lester (christopher@miniforge.ai).
