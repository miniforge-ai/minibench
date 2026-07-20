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

## Live feeds

The demo fixtures are **real adapter output**, not hand-written. The
career and portfolio tenants project their actual validation runs into
`workbench_snapshot/v1` via the `bb workbench:*` tasks in
`thesium-workflows`. `bb regen-fixtures` runs those tasks on the
synthetic, non-personal inputs in `fixtures/inputs/` and writes:

- `fixtures/experiments/{opus-semantic,haiku-mechanical}.json` — one career
  lens experiment under two variants: the permutation matrix (pass vs fail).
- `fixtures/portfolio-daily.json` — a portfolio daily snapshot (second tenant).

```bash
bb regen-fixtures            # needs a thesium-workflows checkout + babashka
cargo run -p minibench-cli -- compare fixtures/experiments
```

`fixtures/sample-snapshot.json` remains the hand-written Miniforge
orchestration example. The product-owned Miniforge ETL adapter now supplies a
real baseline/candidate pair and its registry under `fixtures/miniforge-etl/`.
The pair differs at exactly one resolved-run factor (`:pipeline/mode`) and is
checked by the kernel integration suite.

```bash
cargo run -p minibench-cli -- compare \
  fixtures/miniforge-etl/variants fixtures/miniforge-etl/registry.json
```

## Shell (macOS)

A native SwiftUI app (`app/`, Miniforge UX) — a two-pane shell: a sidebar
listing experiments grouped by tenant, and a detail pane rendering the
selected experiment's comparison matrix. The Rust/Swift seam is HTTP; the
data-plane groups loaded snapshots by experiment and serves
`GET /v1/experiments` (the list) and `GET /v1/experiments/:id/matrix`.

```bash
bb serve       # data-plane on :8789, serving the experiment fixtures
bb run-app     # builds the .app bundle + opens it (in another shell)
```

## Regression gate

The harness can say a run is *worse*, not just *different*. `kernel::diff`
compares a current run set against a committed baseline and reports every
state variable whose status got more severe or whose score dropped:

```bash
bb regression-gate    # minibench diff fixtures/baseline fixtures/experiments
```

`fixtures/baseline/` is the known-good freeze. CI (`.github/workflows/ci.yml`)
runs this on every PR and **fails on any regression**; to accept a new
result, update the baseline. That closes the loop the workbench exists to
close — it doesn't just measure, it gates.

## Evidence validation gate

`minibench validate` enforces the registry's declared
`evidence_requirements` — no `pass` without evidence, required ref
types present, hashes where demanded. Two explicit outs:
`not_applicable` evaluations may carry zero refs (a variable that does
not apply has nothing to evidence — the requirements describe what a
scored evaluation must cite), and a registry `min_count` of `0` is a
waiver, not a demand, mirroring `must_include_*: false`. `bb
validate-gate` runs the op over every committed snapshot —
`fixtures/*.json`, `fixtures/baseline/`, `fixtures/experiments/`, and
`fixtures/miniforge-etl/variants/` — pairing each snapshot with
`fixtures/registries/<registry_id>.json` by its own `registry_ref`; a
snapshot whose registry copy is missing fails the gate rather than
being skipped. CI runs this after the regression gate.

```bash
bb validate-gate      # minibench validate <snapshot> fixtures/registries/<id>.json, per snapshot
```

The registry copies under `fixtures/registries/` are pinned duplicates
of the product repos' registries (career, portfolio, and miniforge from
`workbench-contract/fixtures/`, plus the miniforge-etl registry the ETL
adapter fixtures name); `validate` verifies
registry_id/version/product against each snapshot, so an upstream
version bump fails loudly here instead of validating against a stale
yardstick.

## Registry-aware compare

`minibench compare` can take the registry that scored the snapshots. With it,
same-status score spread is flagged when it crosses that state variable's
threshold-band width:

```bash
cargo run --bin minibench -- compare path/to/one-experiment path/to/registry.json
```

## Deferred to later slices

- **Swift UI shell tiers** — slice 1 (the comparison-matrix window) lives in
  `app/`. Still deferred: the experiment picker, the shared primitive kit,
  and the bespoke per-tenant view plugins (claim graph, equity curve, PR fleet).
- **Kernel** — narration-packet assembly, registry-driven generic
  evaluators (regression diff shipped — see **Regression gate**).
- **Miniforge orchestration feed** — ETL now emits real snapshots; the
  hand-written `sample-snapshot.json` can be replaced after orchestration has
  one canonical resolved-run configuration boundary.

## Run

```bash
cargo test                       # kernel unit test + data-plane integration test
cargo run --bin minibench-data-plane
# then, in another shell:
curl -s http://127.0.0.1:8789/v1/snapshots/latest | jq .product
```

Loads snapshots from `MINIBENCH_SNAPSHOT_DIR` (default `fixtures`).
`thesium-app-foundation` and `workbench-contract` are pinned **git
dependencies** (private; `.cargo/config` sets `git-fetch-with-cli` for the
SSH fetch) — no sibling checkout needed to build.

## Ports

`:8787` risk · `:8788` career · **`:8789` minibench** — all loopback,
all the same foundation router.

## License

Proprietary. © 2025–2026 Christopher Lester (christopher@miniforge.ai).
