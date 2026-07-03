// Title: Minibench
// Subtitle: data-plane provider serving WorkbenchSnapshotV1 over the foundation router
// Author: Christopher Lester
// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

//! Minibench's wiring of the shared `DataPlaneProvider`. Snapshots are
//! exchanged as opaque JSON `Value` (the foundation contract); the
//! bodies are `WorkbenchSnapshotV1` per `workbench-contract`. Minibench
//! is the third consumer of `thesium-app-foundation`, after risk and
//! career — proving the foundation's domain-neutral claim.

pub mod strings;

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use axum::extract::Path as RoutePath;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Extension, Json, Router};
use minibench_kernel::{CompareError, ComparisonMatrix, compare};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thesium_app_foundation_contracts::{
    APP_CONFIG_V1, AppConfigV1, DistributionV1, LicenseValidationResponseV1,
};
use thesium_app_foundation_data_plane::{DataPlaneProvider, build_router};
use workbench_contract::WorkbenchSnapshotV1;

/// Serves workbench snapshots held in memory. A snapshot is a
/// `WorkbenchSnapshotV1` body; here it stays a `Value` because the
/// foundation routes are domain-neutral — the typed contract is
/// re-applied at the consuming edges (the kernel, the Swift shell).
pub struct WorkbenchProvider {
    snapshots: Vec<Value>,
}

impl WorkbenchProvider {
    /// Build from already-parsed snapshot values.
    pub fn new(snapshots: Vec<Value>) -> Self {
        Self { snapshots }
    }

    /// Load every `*.json` snapshot in `dir`.
    pub fn from_dir(dir: impl AsRef<Path>) -> std::io::Result<Self> {
        let mut snapshots = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let bytes = std::fs::read(&path)?;
                let value: Value = serde_json::from_slice(&bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                snapshots.push(value);
            }
        }
        Ok(Self::new(snapshots))
    }

    /// Most recent snapshot by `generated_at` (RFC 3339 sorts lexically).
    fn latest(&self) -> Option<&Value> {
        self.snapshots.iter().max_by(|a, b| {
            let ga = a.get("generated_at").and_then(Value::as_str).unwrap_or("");
            let gb = b.get("generated_at").and_then(Value::as_str).unwrap_or("");
            ga.cmp(gb)
        })
    }

    /// Decode the held snapshot `Value`s into the typed contract, skipping
    /// any that do not conform — the kernel reads only typed snapshots.
    fn decoded_snapshots(&self) -> Vec<WorkbenchSnapshotV1> {
        self.snapshots
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect()
    }
}

#[async_trait]
impl DataPlaneProvider for WorkbenchProvider {
    async fn latest_snapshot(&self) -> Result<Value, String> {
        self.latest()
            .cloned()
            .ok_or_else(|| strings::NO_SNAPSHOTS_LOADED.to_string())
    }

    async fn snapshot_by_id(&self, snapshot_id: &str) -> Result<Option<Value>, String> {
        Ok(self
            .snapshots
            .iter()
            .find(|s| s.get("snapshot_id").and_then(Value::as_str) == Some(snapshot_id))
            .cloned())
    }

    async fn app_config(&self) -> AppConfigV1 {
        AppConfigV1 {
            schema_version: APP_CONFIG_V1.to_string(),
            minimum_supported_snapshot_major: 1,
            distribution: DistributionV1 {
                channel: "direct-download".to_string(),
                platform: "macos".to_string(),
            },
        }
    }

    async fn validate_license(&self) -> LicenseValidationResponseV1 {
        LicenseValidationResponseV1 {
            valid: true,
            tier: "local".to_string(),
        }
    }
}

/// Minibench's added route — the kernel comparison matrix over the loaded
/// snapshots. The foundation router is domain-neutral (the five snapshot
/// routes only), so the roll-up rides this route, not the foundation's.
pub const COMPARISON_ROUTE: &str = "/v1/comparison";
/// Lists the experiments present in the loaded snapshots (sidebar source).
pub const EXPERIMENTS_ROUTE: &str = "/v1/experiments";
/// The comparison matrix for one experiment, by id.
pub const EXPERIMENT_MATRIX_ROUTE: &str = "/v1/experiments/:id/matrix";

/// One experiment in the loaded snapshots — its tenant product and the
/// variant labels under it. The Swift shell lists these in the sidebar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentSummary {
    pub experiment_id: String,
    pub product: String,
    pub variants: Vec<String>,
}

/// Group the loaded snapshots by `variant.experiment_id`, in first-seen
/// order. Snapshots with no variant (not part of an experiment) are skipped.
fn experiments(snapshots: &[WorkbenchSnapshotV1]) -> Vec<ExperimentSummary> {
    let mut order: Vec<String> = Vec::new();
    let mut by_id: HashMap<String, ExperimentSummary> = HashMap::new();
    for snap in snapshots {
        if let Some(variant) = &snap.variant {
            let summary = by_id
                .entry(variant.experiment_id.clone())
                .or_insert_with(|| {
                    order.push(variant.experiment_id.clone());
                    ExperimentSummary {
                        experiment_id: variant.experiment_id.clone(),
                        product: snap.product.clone(),
                        variants: Vec::new(),
                    }
                });
            if !summary.variants.contains(&variant.label) {
                summary.variants.push(variant.label.clone());
            }
        }
    }
    order
        .into_iter()
        .filter_map(|id| by_id.remove(&id))
        .collect()
}

/// The matrix for one experiment, or `None` when no loaded snapshot carries
/// that id (so the route can 404 rather than serve an empty matrix).
fn experiment_matrix(
    snapshots: &[WorkbenchSnapshotV1],
    experiment_id: &str,
) -> Result<Option<ComparisonMatrix>, CompareError> {
    let group: Vec<WorkbenchSnapshotV1> = snapshots
        .iter()
        .filter(|s| s.variant.as_ref().map(|v| v.experiment_id.as_str()) == Some(experiment_id))
        .cloned()
        .collect();
    if group.is_empty() {
        Ok(None)
    } else {
        compare(&group).map(Some)
    }
}

async fn handle_comparison(
    Extension(snapshots): Extension<Arc<Vec<WorkbenchSnapshotV1>>>,
) -> Result<Json<ComparisonMatrix>, StatusCode> {
    compare(&snapshots)
        .map(Json)
        .map_err(|_| StatusCode::BAD_REQUEST)
}

async fn handle_experiments(
    Extension(snapshots): Extension<Arc<Vec<WorkbenchSnapshotV1>>>,
) -> Json<Vec<ExperimentSummary>> {
    Json(experiments(&snapshots))
}

async fn handle_experiment_matrix(
    Extension(snapshots): Extension<Arc<Vec<WorkbenchSnapshotV1>>>,
    RoutePath(experiment_id): RoutePath<String>,
) -> Result<Json<ComparisonMatrix>, StatusCode> {
    match experiment_matrix(&snapshots, &experiment_id) {
        Ok(Some(matrix)) => Ok(Json(matrix)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

/// The foundation's five-route router around a [`WorkbenchProvider`], plus
/// minibench's comparison + experiment routes. The decoded snapshots ride
/// an `Extension` layer on each added route, so the foundation router's
/// state type is unchanged (no `merge` of differing state types).
pub fn router(provider: WorkbenchProvider) -> Router {
    let snapshots = Arc::new(provider.decoded_snapshots());
    build_router(provider)
        .route(
            COMPARISON_ROUTE,
            get(handle_comparison).layer(Extension(snapshots.clone())),
        )
        .route(
            EXPERIMENTS_ROUTE,
            get(handle_experiments).layer(Extension(snapshots.clone())),
        )
        .route(
            EXPERIMENT_MATRIX_ROUTE,
            get(handle_experiment_matrix).layer(Extension(snapshots)),
        )
}
