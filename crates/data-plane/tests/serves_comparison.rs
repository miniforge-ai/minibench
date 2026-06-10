// Title: Minibench
// Subtitle: integration test — the comparison route serves the kernel matrix
// Author: Christopher Lester
// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

//! End-to-end: the minibench comparison route runs the generic kernel over
//! the loaded snapshots and returns a `ComparisonMatrix` — the same matrix
//! the CLI prints and the Swift shell will render.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use minibench_data_plane::{COMPARISON_ROUTE, WorkbenchProvider, router};
use minibench_kernel::ComparisonMatrix;
use tower::ServiceExt; // oneshot

#[tokio::test]
async fn serves_comparison_matrix() {
    // Two real-adapter variants of one career lens experiment.
    let opus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("parse opus variant");
    let haiku: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/haiku-mechanical.json"
    ))
    .expect("parse haiku variant");
    let provider = WorkbenchProvider::new(vec![opus, haiku]);
    let app = router(provider);

    let response = app
        .oneshot(
            Request::builder()
                .uri(COMPARISON_ROUTE)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let matrix: ComparisonMatrix = serde_json::from_slice(&bytes).expect("decode matrix");

    assert_eq!(matrix.experiment_id, "career.lens.acme-l4-eval");
    assert_eq!(matrix.variants.len(), 2);
    let grounded = matrix
        .rows
        .iter()
        .find(|r| r.state_var_id == "career.lens.report_grounded")
        .expect("grounding row present");
    assert!(
        grounded.status_divergence,
        "the grounding row diverges (pass vs fail) across the two variants"
    );
}
