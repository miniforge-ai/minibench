// Title: Minibench
// Subtitle: integration test — the data plane serves a valid contract snapshot
// Author: Christopher Lester
// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

//! End-to-end: a snapshot served over the foundation's
//! `/v1/snapshots/latest` route decodes back into the typed
//! `WorkbenchSnapshotV1`, and the generic kernel summarizes it.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use minibench_data_plane::{WorkbenchProvider, router};
use minibench_kernel::ComparisonMatrix;
use tower::ServiceExt; // oneshot
use workbench_contract::{WORKBENCH_SNAPSHOT_V1, WorkbenchSnapshotV1};

const LATEST_SNAPSHOT_ROUTE: &str = "/v1/snapshots/latest";
const CAREER_EXPERIMENT_MATRIX_ROUTE: &str = "/v1/experiments/career.lens.acme-l4-eval/matrix";
const EARLIER_SNAPSHOT_ID: &str = "wb-miniforge-sample-0001";
const LATER_SNAPSHOT_ID: &str = "wb-miniforge-sample-0002";

#[tokio::test]
async fn serves_latest_snapshot_as_valid_contract() {
    let snap: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/sample-snapshot.json"))
            .expect("parse fixture");
    let provider = WorkbenchProvider::new(vec![snap]);
    let app = router(provider);

    let response = app
        .oneshot(
            Request::builder()
                .uri(LATEST_SNAPSHOT_ROUTE)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let decoded: WorkbenchSnapshotV1 = serde_json::from_slice(&bytes).expect("decode contract");
    assert_eq!(decoded.schema_version, WORKBENCH_SNAPSHOT_V1);

    // The generic kernel reads only the contract — no tenant code.
    let summary = minibench_kernel::summarize(&decoded);
    assert_eq!(summary.total, decoded.evaluations.len());
    assert!(
        !summary.blocking.is_empty() || summary.fail >= 1,
        "the sample snapshot has a failing/blocking evaluation"
    );
}

#[tokio::test]
async fn latest_snapshot_ties_break_by_snapshot_id() {
    let mut earlier: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/sample-snapshot.json"))
            .expect("parse fixture");
    let mut later = earlier.clone();
    earlier["snapshot_id"] = serde_json::Value::String(EARLIER_SNAPSHOT_ID.to_string());
    later["snapshot_id"] = serde_json::Value::String(LATER_SNAPSHOT_ID.to_string());
    let provider = WorkbenchProvider::new(vec![later, earlier]);
    let app = router(provider);

    let response = app
        .oneshot(
            Request::builder()
                .uri(LATEST_SNAPSHOT_ROUTE)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let decoded: WorkbenchSnapshotV1 = serde_json::from_slice(&bytes).expect("decode contract");
    assert_eq!(decoded.snapshot_id, LATER_SNAPSHOT_ID);
}

#[tokio::test]
async fn serves_experiment_matrix() {
    let opus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("parse opus fixture");
    let haiku: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/haiku-mechanical.json"
    ))
    .expect("parse haiku fixture");
    let provider = WorkbenchProvider::new(vec![opus, haiku]);
    let app = router(provider);

    let response = app
        .oneshot(
            Request::builder()
                .uri(CAREER_EXPERIMENT_MATRIX_ROUTE)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let decoded: ComparisonMatrix = serde_json::from_slice(&bytes).expect("decode matrix");
    assert_eq!(decoded.experiment_id, "career.lens.acme-l4-eval");
    assert_eq!(decoded.variants, vec!["opus+semantic", "haiku+mechanical"]);
}

#[tokio::test]
async fn invalid_experiment_matrix_returns_bad_request() {
    let mut snap: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("parse opus fixture");
    snap.evaluations.push(snap.evaluations[0].clone());
    let value = serde_json::to_value(snap).expect("encode invalid snapshot");
    let provider = WorkbenchProvider::new(vec![value]);
    let app = router(provider);

    let response = app
        .oneshot(
            Request::builder()
                .uri(CAREER_EXPERIMENT_MATRIX_ROUTE)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
