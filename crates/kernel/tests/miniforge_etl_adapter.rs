// Title: Minibench
// Subtitle: integration test — real Miniforge ETL adapter output
// Author: Christopher Lester
// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

use minibench_kernel::compare_with_registry;
use workbench_contract::{StateVarRegistry, WorkbenchSnapshotV1};

const BASELINE_JSON: &str = include_str!("../../../fixtures/miniforge-etl/variants/baseline.json");
const CANDIDATE_JSON: &str =
    include_str!("../../../fixtures/miniforge-etl/variants/incremental.json");
const REGISTRY_JSON: &str = include_str!("../../../fixtures/miniforge-etl/registry.json");

#[test]
fn compares_real_miniforge_etl_adapter_snapshots() {
    let baseline: WorkbenchSnapshotV1 =
        serde_json::from_str(BASELINE_JSON).expect("decode ETL baseline");
    let candidate: WorkbenchSnapshotV1 =
        serde_json::from_str(CANDIDATE_JSON).expect("decode ETL candidate");
    let registry: StateVarRegistry =
        serde_json::from_str(REGISTRY_JSON).expect("decode ETL registry");

    let factor_id = candidate
        .variant
        .as_ref()
        .and_then(|variant| variant.axes.get("factor_id"))
        .map(String::as_str);
    assert_eq!(factor_id, Some("[:pipeline :pipeline/mode]"));

    let matrix = compare_with_registry(&[baseline, candidate], &registry)
        .expect("adapter snapshots are comparable");
    assert_eq!(matrix.experiment_id, "miniforge.etl.adapter-e2e");
    assert_eq!(matrix.variants, ["baseline", "incremental"]);
    assert_eq!(matrix.rows.len(), 3);
    assert!(matrix.warnings.is_empty());
}

#[test]
fn persisted_factor_inventory_contains_no_credentials() {
    assert!(!BASELINE_JSON.contains("redacted-fixture-secret"));
    assert!(!CANDIDATE_JSON.contains("redacted-fixture-secret"));
    assert!(!BASELINE_JSON.contains("credential-id"));
    assert!(!CANDIDATE_JSON.contains("credential-id"));
}
