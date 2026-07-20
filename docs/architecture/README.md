# Architecture

How tenant products feed `workbench_snapshot/v1` bodies into the
workbench, what the kernel does with them, and where the seams sit.

Redrawn 2026-07-19 from the implementation. Supersedes the
pre-implementation bootstrap sketches (the 2026-06-06
"thesium-miniforge-workbench-bootstrap" bundle); §6 records what changed
between that sketch and what shipped. Career-pipeline internals live in
`thesium-workflows/docs/design/career-pipeline-architecture.md`, not
here — this doc stops at the adapter seam.

All diagrams are Mermaid fences, rendered natively by GitHub. Edit them
in place; no generated images are checked in, so nothing can go stale
silently.

## 1. System context

Products never link minibench and minibench never links a product crate.
The seam is `workbench-contract` (pure types, Rust canonical, Clojure
Malli mirror for adapters) plus snapshot JSON on disk and HTTP on
loopback.

```mermaid
flowchart LR
  subgraph products["Product pipelines (separate repos)"]
    risk["Portfolio risk<br/>risk-core / risk-dashboard"]
    career["Career intelligence<br/>thesium-workflows"]
    mf["Miniforge governance<br/>supervisory-state projection"]
  end
  subgraph adapters["Adapters (Polylith components in thesium-workflows)"]
    pa["portfolio-workbench-adapter"]
    ca["career-workbench-adapter<br/>lens-report → snapshot<br/>oracle-eval → snapshot"]
    ma["miniforge adapter<br/>not built — hand-written<br/>fixture stands in"]
  end
  contract["workbench-contract<br/>pure types"]
  snaps[("workbench_snapshot/v1<br/>JSON files in<br/>MINIBENCH_SNAPSHOT_DIR")]
  subgraph mb["Minibench (this repo)"]
    dp["data-plane<br/>loopback :8789"]
    kernel["kernel<br/>summarize · compare · diff"]
    cli["minibench-cli"]
    app["Swift shell (app/)"]
  end
  risk --> pa --> snaps
  career --> ca --> snaps
  mf -.-> ma -.-> snaps
  contract -.->|types| adapters
  contract -.->|types| mb
  snaps --> dp --> kernel
  cli --> kernel
  dp -->|HTTP JSON| app
```

There is no plugin registration. A product joins by emitting a snapshot
whose `product` and `registry_ref` are self-describing; the data plane
groups snapshots by `variant.experiment_id` and the sidebar groups by
`product`. Tenants today: portfolio (live adapter output), career (live
adapter output via `bb workbench:*`), miniforge (hand-written fixture;
adapter not built), time (named in the README, unstarted).

Minibench is the third consumer of `thesium-app-foundation` (after risk
`:8787` and career `:8788`) — same foundation router, loopback `:8789`.

## 2. Contract anatomy

Canonical source: `workbench-contract/crates/contracts/src/lib.rs`. The
registry lives in the product repo; the snapshot carries only a
`RegistryRef`. Scoring happens in the adapter before the snapshot
exists.

```mermaid
classDiagram
  class WorkbenchSnapshotV1 {
    schema_version = "workbench_snapshot/v1"
    snapshot_id / generated_at / product / run_id
    variant: Option~RunVariant~
    registry_ref: RegistryRef
    source_hashes
    evaluations: Vec~StateEvaluation~
    entities: product-namespaced bag (Tier-3 views)
    signature: SnapshotSignatureV1 (foundation, reused verbatim)
  }
  class StateEvaluation {
    state_var_id / product
    status: pass warn fail blocked not_applicable unknown
    score / confidence
    score_components: per-component sub-scores
    evidence_refs: Vec~EvidenceRef~
    findings / gate_effect / regression
    evaluated_at
  }
  class StateVarRegistry {
    schema_version = "state_var_registry/v1"
    registry_id / version / product
    state_vars: Vec~StateVariable~
  }
  class StateVariable {
    id (dotted, product-qualified)
    kind: presence integrity sufficiency alignment compliance quality regression budget privacy
    value_type / thresholds (status bands)
    evidence_requirements
    score_components (declared names)
    gate_effects (status → effect)
    lifecycle
  }
  class RunVariant {
    experiment_id / label
    workflow prompt: VariantRef
    model / method / axes
  }
  WorkbenchSnapshotV1 *-- StateEvaluation
  WorkbenchSnapshotV1 *-- RunVariant
  WorkbenchSnapshotV1 --> StateVarRegistry : registry_ref resolves in product repo
  StateVarRegistry *-- StateVariable
  StateEvaluation ..> StateVariable : state_var_id
```

Snapshots sharing `variant.experiment_id` form one comparison set — the
run matrix.

## 3. Snapshot flow and kernel operations

The kernel never scores. It rolls up, compares, and diffs snapshots that
arrive pre-scored. The registry is an optional yardstick: `summarize`
resolves canonical gate effects from `StateVariable.gate_effects`
instead of trusting a possibly-stale `evaluation.gate_effect`, and
`compare` uses `StateVariable.thresholds` band widths to flag
meaningful score spread. It is never a scoring engine.

```mermaid
flowchart LR
  dir[("*.json in<br/>MINIBENCH_SNAPSHOT_DIR")]
  load["WorkbenchProvider::from_dir<br/>decoded_snapshots<br/>non-conforming files skipped"]
  subgraph routes["Router: foundation + minibench"]
    r5["5 foundation routes<br/>/v1/snapshots/* · app_config · validate_license"]
    r3["/v1/comparison<br/>/v1/experiments<br/>/v1/experiments/:id/matrix"]
  end
  subgraph ops["Kernel ops"]
    sum["summarize → RunSummary<br/>status roll-up +<br/>blocking-gate detection<br/>(blocks_transition on fail/blocked)"]
    cmp["compare → ComparisonMatrix<br/>validate comparable set ·<br/>group replicates by variant label ·<br/>spread / divergence signals"]
    dif["diff → RegressionReport<br/>status more severe, or<br/>score drop past epsilon,<br/>vs committed baseline"]
  end
  swift["Swift shell"]
  clic["minibench-cli<br/>compare · summarize · diff"]
  ci["CI regression gate<br/>bb regression-gate<br/>fixtures/baseline vs fixtures/experiments<br/>fails PR on any regression"]
  dir --> load --> r5
  load --> r3
  r3 --> cmp
  swift -->|HTTP| r3
  clic --> sum
  clic --> cmp
  clic --> dif
  ci --> dif
```

The regression gate is the loop the workbench exists to close: it does
not just measure, it gates. Accepting a new result means updating
`fixtures/baseline/`.

## 4. Tenant grounding

| Tenant | Registry | State vars | Feed |
|---|---|---|---|
| portfolio | `portfolio-state-vars` | `portfolio.lens.validation_readiness`, `portfolio.signal.quality_scored`, `portfolio.data.fidelity_gate` | live adapter output |
| career | `career-state-vars` @ 2026.06.06.1 | `career.lens.report_grounded`, `career.lens.evidence_sufficiency`, `career.claim.evidence_traceability`, `career.oracle.reproduction_delta` | live adapter output (`bb workbench:*` in thesium-workflows) |
| miniforge | `miniforge-state-vars` | `miniforge.workflow.machine_authoritative`, `miniforge.evidence.bundle_complete`, `miniforge.gate.critical_violations_block` | hand-written `fixtures/sample-snapshot.json`; adapter not built |
| time | — | — | named only, unstarted |

The miniforge tenant grounds in the supervisory-state projection and the
N-series normative anchors (N2 execution machine, N4 policy gates, N6
evidence). What the future adapter will project:

```mermaid
flowchart LR
  spec["Spec"] --> machine["workflow machine<br/>fsm over clj-statecharts<br/>spec → plan → design → implement →<br/>verify → review → release → observe → done<br/>gates: check / repair"]
  machine --> events["event-stream<br/>N3 append-only"]
  events --> proj["supervisory-state projection<br/>accumulator apply-event →<br/>EntityTable, schema v1"]
  proj --> ents["entities<br/>Spec · WorkflowRun · AgentSession · TaskNode<br/>PolicyEvaluation · PolicyViolation · PrFleetEntry<br/>AttentionItem · DecisionCard · InterventionRequest<br/>DependencyHealth"]
  anchors["normative anchors<br/>N2 execution · N4 policy gates · N6 evidence"]
  ents -.-> adapter["miniforge workbench adapter<br/>(not built)"]
  anchors -.-> adapter
  adapter -.-> snap["workbench_snapshot/v1<br/>miniforge.* state vars"]
```

Stale-sketch note: `EvidenceBundle` and budget records still exist in
miniforge (live component; phase-config `Budget`) but are no longer
first-class supervisory-state entities, so they no longer appear on the
projection spine.

## 5. Shell information architecture

Single macOS window, two-pane `NavigationSplitView`. The Rust/Swift seam
is HTTP on loopback, not FFI; the kernel runs server-side.

```mermaid
flowchart LR
  side["ExperimentSidebar<br/>experiments grouped by product<br/>id + variant count"]
  detail["MatrixView<br/>rows: state variables<br/>columns: variant cells<br/>(status · score · confidence · replicate range)<br/>+ Spread / Within / Confidence / Signals"]
  store["AppStore<br/>ExperimentsPhase · MatrixPhase<br/>fetch + decode"]
  dpl["data-plane :8789<br/>/v1/experiments<br/>/v1/experiments/:id/matrix"]
  side --> detail
  side -.-> store
  detail -.-> store
  store -->|HTTP| dpl
```

Signal chips: `single run`, `between`, `within noise`, `status`,
`meaningful`, `coverage`, `unstable`. Loading/empty/failure render as
phase states in the same window, not separate screens.

Deferred shell tiers: the experiment picker, the shared primitive kit,
and per-tenant bespoke view plugins. The contract already carries the
plugin declaration (`ViewPlugin`, tiers generic / primitive / bespoke,
`consumes` naming the entity-bag namespaces a bespoke view reads).

## 6. What changed between the bootstrap sketch and the implementation

The June bootstrap bundle drew a workbench that evaluates; what shipped
is a workbench that compares and gates, with evaluation pushed out to
the product adapters.

| Bootstrap sketch (2026-06-06) | Implementation |
|---|---|
| Kernel runs deterministic / semantic / LLM evaluators over snapshots | Adapters score in the product repo; kernel only summarizes, compares, diffs |
| State-variable registry drives evaluator dispatch | Registry is an optional yardstick: canonical gate effects, threshold band widths |
| Bounded LLM narration inside the workbench | Not built; narration-packet assembly deferred |
| Broad multi-view UI (registry browser, evidence chain, correction queue, per-tenant boards) | One window: sidebar + comparison matrix; the rest deferred |
| Shared local evidence store between products and workbench | File-drop snapshot dir + loopback HTTP; no shared store |
| Four tenants live | portfolio + career live; miniforge fixture stand-in; time unstarted |

Invariants that survived: products own scoring and evidence; the
workbench owns comparison and regression; every snapshot is versioned
(`workbench_snapshot/v1`) and diffable; the regression gate fails CI on
any status/score regression.
