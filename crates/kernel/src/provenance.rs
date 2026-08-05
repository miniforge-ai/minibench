// Title: Minibench
// Subtitle: kernel — snapshot provenance extraction
// Author: Christopher Lester
// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

//! Where a snapshot's policy/evaluator provenance lives (variant axes,
//! then metadata) and the uniformity check comparison requires.

use serde_json::Value;
use workbench_contract::WorkbenchSnapshotV1;

// ------------------------------------------------------------------ Layer 0

/// Snapshot/variant provenance key for a policy content hash.
pub(crate) const PROVENANCE_KEY_POLICY_HASH: &str = "policy_hash";
/// Snapshot/variant provenance key for a policy version id.
const PROVENANCE_KEY_POLICY_VERSION: &str = "policy_version";
/// Snapshot/variant provenance key for an evaluator implementation hash.
const PROVENANCE_KEY_EVALUATOR_HASH: &str = "evaluator_hash";
/// Snapshot/variant provenance key for an evaluator version id.
pub(crate) const PROVENANCE_KEY_EVALUATOR_VERSION: &str = "evaluator_version";
/// Metadata object that may hold provenance fields.
const METADATA_PROVENANCE_FIELD: &str = "provenance";
/// Provenance fields that prove policy comparability when uniform.
pub(crate) const POLICY_PROVENANCE_KEYS: &[&str] =
    &[PROVENANCE_KEY_POLICY_HASH, PROVENANCE_KEY_POLICY_VERSION];
/// Provenance fields that prove evaluator comparability when uniform.
pub(crate) const EVALUATOR_PROVENANCE_KEYS: &[&str] = &[
    PROVENANCE_KEY_EVALUATOR_HASH,
    PROVENANCE_KEY_EVALUATOR_VERSION,
];

/// A snapshot whose provenance value for `key` diverges from the
/// comparison set. The comparison boundary maps this into
/// `CompareError::MixedProvenance`.
pub(crate) struct ProvenanceMismatch {
    pub key: String,
    pub expected: String,
    pub found: String,
    pub snapshot_id: String,
}

fn metadata_provenance_value<'a>(metadata: Option<&'a Value>, key: &str) -> Option<&'a str> {
    metadata
        .and_then(|value| value.get(key).and_then(Value::as_str))
        .or_else(|| {
            metadata.and_then(|value| {
                value
                    .get(METADATA_PROVENANCE_FIELD)
                    .and_then(|provenance| provenance.get(key))
                    .and_then(Value::as_str)
            })
        })
}

// ------------------------------------------------------------------ Layer 1

fn provenance_value(snapshot: &WorkbenchSnapshotV1, key: &str) -> Option<String> {
    snapshot
        .variant
        .as_ref()
        .and_then(|variant| variant.axes.get(key).cloned())
        .or_else(|| metadata_provenance_value(snapshot.metadata.as_ref(), key).map(str::to_string))
}

// ------------------------------------------------------------------ Layer 2

pub(crate) fn validate_provenance_key(
    snapshots: &[WorkbenchSnapshotV1],
    key: &str,
) -> Result<bool, ProvenanceMismatch> {
    let values: Vec<_> = snapshots
        .iter()
        .map(|snapshot| (snapshot, provenance_value(snapshot, key)))
        .collect();
    if values.iter().all(|(_, value)| value.is_none()) {
        return Ok(false);
    }

    let expected = values.iter().find_map(|(_, value)| value.as_ref()).cloned();
    let Some(expected) = expected else {
        return Ok(false);
    };

    for (snapshot, value) in values {
        match value {
            Some(value) if value == expected => {}
            Some(value) => {
                return Err(ProvenanceMismatch {
                    key: key.to_string(),
                    expected,
                    found: value,
                    snapshot_id: snapshot.snapshot_id.clone(),
                });
            }
            None => {
                return Err(ProvenanceMismatch {
                    key: key.to_string(),
                    expected,
                    found: "<missing>".to_string(),
                    snapshot_id: snapshot.snapshot_id.clone(),
                });
            }
        }
    }
    Ok(true)
}
