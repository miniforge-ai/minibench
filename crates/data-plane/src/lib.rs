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

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use axum::routing::get;
use axum::{Extension, Json, Router};
use minibench_kernel::{ComparisonMatrix, compare};
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

async fn handle_comparison(
    Extension(snapshots): Extension<Arc<Vec<WorkbenchSnapshotV1>>>,
) -> Json<ComparisonMatrix> {
    Json(compare(&snapshots))
}

/// The foundation's five-route router around a [`WorkbenchProvider`], plus
/// minibench's comparison route ([`COMPARISON_ROUTE`]). The decoded
/// snapshots ride an `Extension` layer on that one route, so the foundation
/// router's state type is unchanged — no `merge` of differing state types.
pub fn router(provider: WorkbenchProvider) -> Router {
    let snapshots = Arc::new(provider.decoded_snapshots());
    build_router(provider).route(
        COMPARISON_ROUTE,
        get(handle_comparison).layer(Extension(snapshots)),
    )
}
