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
use tower::ServiceExt; // oneshot
use workbench_contract::{WORKBENCH_SNAPSHOT_V1, WorkbenchSnapshotV1};

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
                .uri("/v1/snapshots/latest")
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
