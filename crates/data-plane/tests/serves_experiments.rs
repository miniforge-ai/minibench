// Title: Minibench
// Subtitle: integration test — the experiment list + per-experiment matrix routes
// Author: Christopher Lester
// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai)
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The data-plane groups loaded snapshots by experiment and serves the list
//! (`/v1/experiments`) and one experiment's matrix
//! (`/v1/experiments/:id/matrix`) — the two routes the Swift sidebar + detail
//! consume.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use minibench_data_plane::{EXPERIMENTS_ROUTE, ExperimentSummary, WorkbenchProvider, router};
use minibench_kernel::ComparisonMatrix;
use serde::de::DeserializeOwned;
use tower::ServiceExt; // oneshot

/// The four real-adapter variant snapshots: two experiments (career lens,
/// portfolio readiness), two variants each.
fn provider() -> WorkbenchProvider {
    let parse = |s: &str| serde_json::from_str(s).expect("parse fixture");
    WorkbenchProvider::new(vec![
        parse(include_str!(
            "../../../fixtures/experiments/opus-semantic.json"
        )),
        parse(include_str!(
            "../../../fixtures/experiments/haiku-mechanical.json"
        )),
        parse(include_str!(
            "../../../fixtures/experiments/portfolio-baseline.json"
        )),
        parse(include_str!(
            "../../../fixtures/experiments/portfolio-degraded.json"
        )),
    ])
}

fn get(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

async fn decode<T: DeserializeOwned>(response: Response) -> T {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).expect("decode body")
}

#[tokio::test]
async fn lists_experiments() {
    let response = router(provider())
        .oneshot(get(EXPERIMENTS_ROUTE))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let experiments: Vec<ExperimentSummary> = decode(response).await;
    assert_eq!(experiments.len(), 2);
    assert!(
        experiments
            .iter()
            .any(|e| e.experiment_id == "portfolio.readiness"
                && e.product == "portfolio"
                && e.variants.len() == 2)
    );
    assert!(
        experiments
            .iter()
            .any(|e| e.experiment_id == "career.lens.acme-l4-eval" && e.product == "career")
    );
}

#[tokio::test]
async fn serves_one_experiment_matrix() {
    let response = router(provider())
        .oneshot(get("/v1/experiments/portfolio.readiness/matrix"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let matrix: ComparisonMatrix = decode(response).await;
    assert_eq!(matrix.experiment_id, "portfolio.readiness");
    // validation_readiness + fidelity_gate, both diverge across the variants.
    assert_eq!(matrix.rows.len(), 2);
    assert!(matrix.rows.iter().all(|r| r.status_divergence));
}

#[tokio::test]
async fn unknown_experiment_is_404() {
    let response = router(provider())
        .oneshot(get("/v1/experiments/does.not.exist/matrix"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
