// Title: Minibench
// Subtitle: kernel — snapshot identity and registry guards
// Author: Christopher Lester
// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

//! Vocabulary shared across kernel operations: variant/experiment
//! labels, registry reference keys, and the snapshot-vs-registry guard
//! every registry-aware operation applies before trusting its input.

use workbench_contract::{RegistryRef, StateVarRegistry, WorkbenchSnapshotV1};

// ------------------------------------------------------------------ Layer 0

/// Experiment id used for one-off snapshots with no variant tag.
const UNGROUPED_EXPERIMENT_ID: &str = "ungrouped";

pub(crate) fn variant_label(snapshot: &WorkbenchSnapshotV1) -> String {
    snapshot
        .variant
        .as_ref()
        .map(|v| v.label.clone())
        .unwrap_or_else(|| snapshot.run_id.clone())
}

pub(crate) fn experiment_id(snapshot: &WorkbenchSnapshotV1) -> String {
    snapshot
        .variant
        .as_ref()
        .map(|v| v.experiment_id.clone())
        .unwrap_or_else(|| UNGROUPED_EXPERIMENT_ID.to_string())
}

pub(crate) fn registry_ref_key(registry_ref: &RegistryRef) -> String {
    format!("{}@{}", registry_ref.registry_id, registry_ref.version)
}

pub(crate) fn registry_key(registry: &StateVarRegistry) -> String {
    format!("{}@{}", registry.registry_id, registry.version)
}

/// Why a supplied registry cannot vouch for a snapshot. Callers map
/// this into their operation's own error type at the boundary.
pub(crate) enum RegistryMismatch {
    Ref { expected: String, found: String },
    Product { expected: String, found: String },
}

// ------------------------------------------------------------------ Layer 1

/// Guard that a registry is the one that scored the snapshot: the
/// registry ref and product must both match. All registry-aware kernel
/// operations apply this check before using the registry as a yardstick.
pub(crate) fn check_registry(
    snapshot: &WorkbenchSnapshotV1,
    registry: &StateVarRegistry,
) -> Result<(), RegistryMismatch> {
    let expected = registry_ref_key(&snapshot.registry_ref);
    let found = registry_key(registry);
    if found != expected {
        return Err(RegistryMismatch::Ref { expected, found });
    }
    if registry.product != snapshot.product {
        return Err(RegistryMismatch::Product {
            expected: snapshot.product.clone(),
            found: registry.product.clone(),
        });
    }
    Ok(())
}
