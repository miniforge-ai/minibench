// Title: Minibench
// Subtitle: kernel — tenant-agnostic roll-up of a workbench snapshot
// Author: Christopher Lester
// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

//! The generic kernel: tenant-agnostic operations over a
//! [`WorkbenchSnapshotV1`]. It reads only the contract — never a
//! product's domain crate — so the same code summarizes a portfolio,
//! career, or miniforge snapshot identically. That genericity is the
//! whole point: product-specific scoring happens in the product's
//! adapter; the kernel does the cross-cutting roll-up, regression
//! diff, and narration-packet assembly the shell renders.
//!
//! Ships the run summary, comparison matrix, and regression diff;
//! narration-packet assembly lands in a later slice.

use std::collections::HashMap;

use std::cmp::Reverse;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use workbench_contract::{
    RegistryRef, StateEvaluation, StateStatus, StateVarRegistry, StateVariable, WorkbenchSnapshotV1,
};

/// Canonical registry gate effect for a state variable that blocks run
/// progression. Registries may add other effects for review/repair, but
/// blocking has to be exact here so unrelated product strings containing
/// "block" are not elevated accidentally.
const GATE_EFFECT_BLOCKS_TRANSITION: &str = "blocks_transition";
/// Gate effects the generic kernel treats as progression-blocking when
/// no registry is supplied. Registry-aware summary resolves the effect
/// from `StateVariable::gate_effects` first.
const BLOCKING_GATE_EFFECTS: &[&str] = &[GATE_EFFECT_BLOCKS_TRANSITION];
/// Status key for `StateStatus::Pass` inside registry `gate_effects`.
const STATUS_KEY_PASS: &str = "pass";
/// Status key for `StateStatus::Warn` inside registry `gate_effects`.
const STATUS_KEY_WARN: &str = "warn";
/// Status key for `StateStatus::Fail` inside registry `gate_effects`.
const STATUS_KEY_FAIL: &str = "fail";
/// Status key for `StateStatus::Blocked` inside registry `gate_effects`.
const STATUS_KEY_BLOCKED: &str = "blocked";
/// Status key for `StateStatus::NotApplicable` inside registry `gate_effects`.
const STATUS_KEY_NOT_APPLICABLE: &str = "not_applicable";
/// Status key for `StateStatus::Unknown` inside registry `gate_effects`.
const STATUS_KEY_UNKNOWN: &str = "unknown";
/// Experiment id used for one-off snapshots with no variant tag.
const UNGROUPED_EXPERIMENT_ID: &str = "ungrouped";
/// Standard deviation is only meaningful once at least two replicate
/// observations exist.
const MIN_REPLICATES_FOR_SPREAD: usize = 2;
/// Float tolerance for classifying score-spread signals.
const SCORE_SPREAD_EPSILON: f64 = 1e-9;
/// Stable ordering used only to make tied status votes deterministic.
const STATE_STATUS_ORDER: &[StateStatus] = &[
    StateStatus::Pass,
    StateStatus::Warn,
    StateStatus::Fail,
    StateStatus::Blocked,
    StateStatus::NotApplicable,
    StateStatus::Unknown,
];
/// Snapshot/variant provenance key for a policy content hash.
const PROVENANCE_KEY_POLICY_HASH: &str = "policy_hash";
/// Snapshot/variant provenance key for a policy version id.
const PROVENANCE_KEY_POLICY_VERSION: &str = "policy_version";
/// Snapshot/variant provenance key for an evaluator implementation hash.
const PROVENANCE_KEY_EVALUATOR_HASH: &str = "evaluator_hash";
/// Snapshot/variant provenance key for an evaluator version id.
const PROVENANCE_KEY_EVALUATOR_VERSION: &str = "evaluator_version";
/// Metadata object that may hold provenance fields.
const METADATA_PROVENANCE_FIELD: &str = "provenance";
/// Provenance fields that prove policy comparability when uniform.
const POLICY_PROVENANCE_KEYS: &[&str] =
    &[PROVENANCE_KEY_POLICY_HASH, PROVENANCE_KEY_POLICY_VERSION];
/// Provenance fields that prove evaluator comparability when uniform.
const EVALUATOR_PROVENANCE_KEYS: &[&str] = &[
    PROVENANCE_KEY_EVALUATOR_HASH,
    PROVENANCE_KEY_EVALUATOR_VERSION,
];

/// A tenant-agnostic roll-up of one snapshot's evaluations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunSummary {
    pub product: String,
    pub snapshot_id: String,
    pub total: usize,
    pub pass: usize,
    pub warn: usize,
    pub fail: usize,
    pub blocked: usize,
    pub not_applicable: usize,
    pub unknown: usize,
    /// `state_var_id`s whose resolved gate effect blocks progression.
    pub blocking: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SummaryError {
    RegistryRefMismatch { expected: String, found: String },
    RegistryProductMismatch { expected: String, found: String },
}

impl fmt::Display for SummaryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RegistryRefMismatch { expected, found } => {
                write!(
                    f,
                    "registry {found} does not match snapshot registry {expected}"
                )
            }
            Self::RegistryProductMismatch { expected, found } => {
                write!(
                    f,
                    "registry product {found} does not match snapshot product {expected}"
                )
            }
        }
    }
}

impl std::error::Error for SummaryError {}

/// Roll up a snapshot's evaluations by status and collect the
/// state variables whose gate effect blocks progression.
pub fn summarize(snapshot: &WorkbenchSnapshotV1) -> RunSummary {
    summarize_with_gate_resolver(snapshot, |evaluation| Some(evaluation.gate_effect.as_str()))
}

/// Roll up a snapshot using the registry's status→gate-effect map as the
/// canonical yardstick. This avoids trusting a stale resolved
/// `evaluation.gate_effect` when the caller has the registry that scored
/// the snapshot.
pub fn summarize_with_registry(
    snapshot: &WorkbenchSnapshotV1,
    registry: &StateVarRegistry,
) -> Result<RunSummary, SummaryError> {
    let snapshot_registry_ref = registry_ref_key(&snapshot.registry_ref);
    let registry_ref = format!("{}@{}", registry.registry_id, registry.version);
    if registry_ref != snapshot_registry_ref {
        return Err(SummaryError::RegistryRefMismatch {
            expected: snapshot_registry_ref,
            found: registry_ref,
        });
    }
    if registry.product != snapshot.product {
        return Err(SummaryError::RegistryProductMismatch {
            expected: snapshot.product.clone(),
            found: registry.product.clone(),
        });
    }

    let state_vars: BTreeMap<&str, &StateVariable> = registry
        .state_vars
        .iter()
        .map(|state_var| (state_var.id.as_str(), state_var))
        .collect();

    Ok(summarize_with_gate_resolver(snapshot, |evaluation| {
        state_vars
            .get(evaluation.state_var_id.as_str())
            .and_then(|state_var| state_var.gate_effects.get(status_key(evaluation.status)))
            .map(String::as_str)
            .or(Some(evaluation.gate_effect.as_str()))
    }))
}

fn summarize_with_gate_resolver<'a>(
    snapshot: &'a WorkbenchSnapshotV1,
    gate_effect: impl Fn(&'a StateEvaluation) -> Option<&'a str>,
) -> RunSummary {
    let mut summary = RunSummary {
        product: snapshot.product.clone(),
        snapshot_id: snapshot.snapshot_id.clone(),
        total: snapshot.evaluations.len(),
        pass: 0,
        warn: 0,
        fail: 0,
        blocked: 0,
        not_applicable: 0,
        unknown: 0,
        blocking: Vec::new(),
    };
    for ev in &snapshot.evaluations {
        match ev.status {
            StateStatus::Pass => summary.pass += 1,
            StateStatus::Warn => summary.warn += 1,
            StateStatus::Fail => summary.fail += 1,
            StateStatus::Blocked => summary.blocked += 1,
            StateStatus::NotApplicable => summary.not_applicable += 1,
            StateStatus::Unknown => summary.unknown += 1,
        }
        if status_can_block(ev.status) && gate_effect(ev).is_some_and(is_blocking_gate_effect) {
            summary.blocking.push(ev.state_var_id.clone());
        }
    }
    summary
}

fn status_key(status: StateStatus) -> &'static str {
    match status {
        StateStatus::Pass => STATUS_KEY_PASS,
        StateStatus::Warn => STATUS_KEY_WARN,
        StateStatus::Fail => STATUS_KEY_FAIL,
        StateStatus::Blocked => STATUS_KEY_BLOCKED,
        StateStatus::NotApplicable => STATUS_KEY_NOT_APPLICABLE,
        StateStatus::Unknown => STATUS_KEY_UNKNOWN,
    }
}

fn status_can_block(status: StateStatus) -> bool {
    matches!(status, StateStatus::Fail | StateStatus::Blocked)
}

fn is_blocking_gate_effect(gate_effect: &str) -> bool {
    BLOCKING_GATE_EFFECTS.contains(&gate_effect)
}

//------------------------------------------------------------------------------
// Comparison matrix — the permutation harness
//
// The workbench exists to compare PERMUTATIONS of a task (workflow /
// prompt / model / mechanical-vs-semantic). Given the snapshots for one
// experiment, `compare` lays them out as a matrix: rows are state
// variables, columns are variants, cells are score/status. Per row it
// flags where the variants actually diverge — that's the signal the
// shell highlights so you can see which config changed which outcome.

/// One variant's result for one state variable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComparisonCell {
    /// Majority status across replicates. A tied replicate vote resolves
    /// to `Unknown` and sets `status_unstable`.
    pub status: StateStatus,
    /// Mean score across replicates that produced this state variable.
    pub score: f64,
    pub score_min: f64,
    pub score_max: f64,
    pub score_sd: f64,
    /// Mean confidence across replicates that produced this state
    /// variable.
    pub confidence: f64,
    pub confidence_min: f64,
    pub confidence_max: f64,
    pub present_count: usize,
    pub replicate_count: usize,
    pub status_unstable: bool,
}

/// One state variable across every variant in the experiment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComparisonRow {
    pub state_var_id: String,
    /// Aligned to `ComparisonMatrix::variants`; `None` where a variant
    /// produced no evaluation for this state variable.
    pub cells: Vec<Option<ComparisonCell>>,
    /// max − min of variant mean scores across produced cells.
    pub score_spread: f64,
    /// Largest within-variant min/max score spread for this row.
    pub within_score_spread: f64,
    /// Lowest confidence observed across the row's present cells.
    pub confidence_min: f64,
    /// Highest confidence observed across the row's present cells.
    pub confidence_max: f64,
    /// max - min confidence across variants and replicates for this row.
    pub confidence_spread: f64,
    /// Interpretation of score spread relative to measured replicate noise.
    pub spread_signal: SpreadSignal,
    /// True when stable majority statuses differ across variants.
    pub status_divergence: bool,
    /// True when any variant or replicate did not produce this row.
    pub coverage_divergence: bool,
    /// True when at least one variant has no unique majority status.
    pub status_unstable: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpreadSignal {
    NoSpread,
    SingleRun,
    BetweenExceedsWithin,
    WithinMatchesBetween,
}

/// A run matrix for one experiment: state variables × variants.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComparisonMatrix {
    pub experiment_id: String,
    /// Column labels, in first-seen variant order (variant label, else
    /// run id). Snapshots with the same experiment + label and distinct
    /// run ids are grouped as replicates under one column.
    pub variants: Vec<String>,
    pub variant_replicates: Vec<usize>,
    pub rows: Vec<ComparisonRow>,
    pub warnings: Vec<CompareWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompareWarning {
    MissingSourceHashes,
    MissingPolicyProvenance,
    MissingEvaluatorProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompareError {
    EmptyInput,
    MixedExperimentIds {
        expected: String,
        found: String,
        snapshot_id: String,
    },
    MixedProducts {
        expected: String,
        found: String,
        snapshot_id: String,
    },
    MixedRegistryRefs {
        expected: String,
        found: String,
        snapshot_id: String,
    },
    MixedSourceHashes {
        snapshot_id: String,
    },
    MixedProvenance {
        key: String,
        expected: String,
        found: String,
        snapshot_id: String,
    },
    DuplicateStateVarId {
        snapshot_id: String,
        state_var_id: String,
    },
    DuplicateVariantRun {
        label: String,
        run_id: String,
    },
}

impl fmt::Display for CompareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => write!(f, "no snapshots to compare"),
            Self::MixedExperimentIds {
                expected,
                found,
                snapshot_id,
            } => write!(
                f,
                "snapshot {snapshot_id} has experiment {found}, expected {expected}"
            ),
            Self::MixedProducts {
                expected,
                found,
                snapshot_id,
            } => write!(
                f,
                "snapshot {snapshot_id} has product {found}, expected {expected}"
            ),
            Self::MixedRegistryRefs {
                expected,
                found,
                snapshot_id,
            } => write!(
                f,
                "snapshot {snapshot_id} uses registry {found}, expected {expected}"
            ),
            Self::MixedSourceHashes { snapshot_id } => write!(
                f,
                "snapshot {snapshot_id} has source hashes that do not match the comparison set"
            ),
            Self::MixedProvenance {
                key,
                expected,
                found,
                snapshot_id,
            } => write!(
                f,
                "snapshot {snapshot_id} has provenance {key}={found}, expected {expected}"
            ),
            Self::DuplicateStateVarId {
                snapshot_id,
                state_var_id,
            } => write!(
                f,
                "snapshot {snapshot_id} repeats evaluation {state_var_id}"
            ),
            Self::DuplicateVariantRun { label, run_id } => write!(
                f,
                "variant {label} contains duplicate run id {run_id}; replicates must have distinct run ids"
            ),
        }
    }
}

impl std::error::Error for CompareError {}

struct CompareContext {
    experiment_id: String,
    warnings: Vec<CompareWarning>,
}

fn variant_label(snapshot: &WorkbenchSnapshotV1) -> String {
    snapshot
        .variant
        .as_ref()
        .map(|v| v.label.clone())
        .unwrap_or_else(|| snapshot.run_id.clone())
}

fn experiment_id(snapshot: &WorkbenchSnapshotV1) -> String {
    snapshot
        .variant
        .as_ref()
        .map(|v| v.experiment_id.clone())
        .unwrap_or_else(|| UNGROUPED_EXPERIMENT_ID.to_string())
}

fn registry_ref_key(registry_ref: &RegistryRef) -> String {
    format!("{}@{}", registry_ref.registry_id, registry_ref.version)
}

fn normalized_source_hashes(snapshot: &WorkbenchSnapshotV1) -> Option<Vec<String>> {
    snapshot.source_hashes.as_ref().map(|hashes| {
        let mut normalized = hashes.clone();
        normalized.sort();
        normalized
    })
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

fn provenance_value(snapshot: &WorkbenchSnapshotV1, key: &str) -> Option<String> {
    snapshot
        .variant
        .as_ref()
        .and_then(|variant| variant.axes.get(key).cloned())
        .or_else(|| metadata_provenance_value(snapshot.metadata.as_ref(), key).map(str::to_string))
}

fn validate_provenance_key(
    snapshots: &[WorkbenchSnapshotV1],
    key: &str,
) -> Result<bool, CompareError> {
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
                return Err(CompareError::MixedProvenance {
                    key: key.to_string(),
                    expected,
                    found: value,
                    snapshot_id: snapshot.snapshot_id.clone(),
                });
            }
            None => {
                return Err(CompareError::MixedProvenance {
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

fn validate_provenance_group(
    snapshots: &[WorkbenchSnapshotV1],
    keys: &[&str],
) -> Result<bool, CompareError> {
    let mut has_group_provenance = false;
    for key in keys {
        has_group_provenance |= validate_provenance_key(snapshots, key)?;
    }
    Ok(has_group_provenance)
}

fn validate_comparable(snapshots: &[WorkbenchSnapshotV1]) -> Result<CompareContext, CompareError> {
    let Some(first) = snapshots.first() else {
        return Err(CompareError::EmptyInput);
    };

    let expected_experiment_id = experiment_id(first);
    let expected_product = first.product.clone();
    let expected_registry_ref = first.registry_ref.clone();
    let expected_source_hashes = normalized_source_hashes(first);
    let mut missing_source_hashes = expected_source_hashes.is_none();

    let mut seen_variant_runs: BTreeSet<(String, String)> = BTreeSet::new();
    for snapshot in snapshots {
        let found_experiment_id = experiment_id(snapshot);
        if found_experiment_id != expected_experiment_id {
            return Err(CompareError::MixedExperimentIds {
                expected: expected_experiment_id.clone(),
                found: found_experiment_id,
                snapshot_id: snapshot.snapshot_id.clone(),
            });
        }

        if snapshot.product != expected_product {
            return Err(CompareError::MixedProducts {
                expected: expected_product.clone(),
                found: snapshot.product.clone(),
                snapshot_id: snapshot.snapshot_id.clone(),
            });
        }

        if snapshot.registry_ref != expected_registry_ref {
            return Err(CompareError::MixedRegistryRefs {
                expected: registry_ref_key(&expected_registry_ref),
                found: registry_ref_key(&snapshot.registry_ref),
                snapshot_id: snapshot.snapshot_id.clone(),
            });
        }

        let source_hashes = normalized_source_hashes(snapshot);
        missing_source_hashes |= source_hashes.is_none();
        if source_hashes.is_some()
            && expected_source_hashes.is_some()
            && source_hashes != expected_source_hashes
        {
            return Err(CompareError::MixedSourceHashes {
                snapshot_id: snapshot.snapshot_id.clone(),
            });
        }
        if source_hashes.is_some() != expected_source_hashes.is_some() {
            return Err(CompareError::MixedSourceHashes {
                snapshot_id: snapshot.snapshot_id.clone(),
            });
        }

        let mut state_var_ids = BTreeSet::new();
        for evaluation in &snapshot.evaluations {
            if !state_var_ids.insert(evaluation.state_var_id.clone()) {
                return Err(CompareError::DuplicateStateVarId {
                    snapshot_id: snapshot.snapshot_id.clone(),
                    state_var_id: evaluation.state_var_id.clone(),
                });
            }
        }

        let label = variant_label(snapshot);
        let run_key = (label.clone(), snapshot.run_id.clone());
        if !seen_variant_runs.insert(run_key) {
            return Err(CompareError::DuplicateVariantRun {
                label,
                run_id: snapshot.run_id.clone(),
            });
        }
    }

    let mut warnings = Vec::new();
    if missing_source_hashes {
        warnings.push(CompareWarning::MissingSourceHashes);
    }
    if !validate_provenance_group(snapshots, POLICY_PROVENANCE_KEYS)? {
        warnings.push(CompareWarning::MissingPolicyProvenance);
    }
    if !validate_provenance_group(snapshots, EVALUATOR_PROVENANCE_KEYS)? {
        warnings.push(CompareWarning::MissingEvaluatorProvenance);
    }

    Ok(CompareContext {
        experiment_id: expected_experiment_id,
        warnings,
    })
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn spread(values: &[f64]) -> f64 {
    if values.len() < MIN_REPLICATES_FOR_SPREAD {
        return 0.0;
    }
    let min = values.iter().copied().reduce(f64::min).unwrap_or(0.0);
    let max = values.iter().copied().reduce(f64::max).unwrap_or(0.0);
    max - min
}

fn spread_signal(
    score_spread: f64,
    within_score_spread: f64,
    row_present_counts: &[usize],
) -> SpreadSignal {
    if score_spread <= SCORE_SPREAD_EPSILON && within_score_spread <= SCORE_SPREAD_EPSILON {
        return SpreadSignal::NoSpread;
    }
    if row_present_counts
        .iter()
        .all(|present_count| *present_count < MIN_REPLICATES_FOR_SPREAD)
    {
        return SpreadSignal::SingleRun;
    }
    if within_score_spread + SCORE_SPREAD_EPSILON >= score_spread {
        SpreadSignal::WithinMatchesBetween
    } else {
        SpreadSignal::BetweenExceedsWithin
    }
}

fn standard_deviation(values: &[f64]) -> f64 {
    if values.len() < MIN_REPLICATES_FOR_SPREAD {
        return 0.0;
    }
    let avg = mean(values);
    let variance = values
        .iter()
        .map(|value| {
            let delta = value - avg;
            delta * delta
        })
        .sum::<f64>()
        / values.len() as f64;
    variance.sqrt()
}

fn majority_status(statuses: &[StateStatus]) -> (StateStatus, bool) {
    let mut ranked: Vec<(StateStatus, usize)> = STATE_STATUS_ORDER
        .iter()
        .map(|status| {
            (
                *status,
                statuses
                    .iter()
                    .filter(|candidate| *candidate == status)
                    .count(),
            )
        })
        .filter(|(_, count)| *count > 0)
        .collect();
    ranked.sort_by_key(|(_, count)| Reverse(*count));

    let Some((status, count)) = ranked.first() else {
        return (StateStatus::Unknown, true);
    };
    let tied = ranked
        .get(1)
        .is_some_and(|(_, next_count)| next_count == count);
    if tied {
        (StateStatus::Unknown, true)
    } else {
        (*status, false)
    }
}

fn aggregate_cell(
    evaluations: Vec<&StateEvaluation>,
    replicate_count: usize,
) -> Option<ComparisonCell> {
    if evaluations.is_empty() {
        return None;
    }

    let scores: Vec<f64> = evaluations
        .iter()
        .map(|evaluation| evaluation.score)
        .collect();
    let confidences: Vec<f64> = evaluations
        .iter()
        .map(|evaluation| evaluation.confidence)
        .collect();
    let statuses: Vec<StateStatus> = evaluations
        .iter()
        .map(|evaluation| evaluation.status)
        .collect();
    let (status, status_unstable) = majority_status(&statuses);

    Some(ComparisonCell {
        status,
        score: mean(&scores),
        score_min: scores.iter().copied().reduce(f64::min).unwrap_or(0.0),
        score_max: scores.iter().copied().reduce(f64::max).unwrap_or(0.0),
        score_sd: standard_deviation(&scores),
        confidence: mean(&confidences),
        confidence_min: confidences.iter().copied().reduce(f64::min).unwrap_or(0.0),
        confidence_max: confidences.iter().copied().reduce(f64::max).unwrap_or(0.0),
        present_count: evaluations.len(),
        replicate_count,
        status_unstable,
    })
}

/// Lay out the snapshots of one experiment as a comparison matrix.
/// Variant columns keep first-seen order; snapshots with the same
/// experiment + label and distinct run ids are grouped as replicates.
/// State-variable rows keep first-seen order across snapshots so the
/// matrix is stable.
pub fn compare(snapshots: &[WorkbenchSnapshotV1]) -> Result<ComparisonMatrix, CompareError> {
    let ctx = validate_comparable(snapshots)?;

    let mut variants: Vec<String> = Vec::new();
    let mut groups: BTreeMap<String, Vec<&WorkbenchSnapshotV1>> = BTreeMap::new();
    for snapshot in snapshots {
        let label = variant_label(snapshot);
        match groups.entry(label.clone()) {
            Entry::Vacant(entry) => {
                variants.push(label);
                entry.insert(vec![snapshot]);
            }
            Entry::Occupied(mut entry) => entry.get_mut().push(snapshot),
        }
    }

    // First-seen order of state-variable ids across all snapshots.
    let mut order: Vec<String> = Vec::new();
    for snap in snapshots {
        for ev in &snap.evaluations {
            if !order.iter().any(|id| id == &ev.state_var_id) {
                order.push(ev.state_var_id.clone());
            }
        }
    }

    let variant_replicates: Vec<usize> = variants
        .iter()
        .map(|variant| groups[variant].len())
        .collect();

    let rows = order
        .into_iter()
        .map(|state_var_id| {
            let cells: Vec<Option<ComparisonCell>> = variants
                .iter()
                .map(|variant| {
                    let group = &groups[variant];
                    let evaluations: Vec<_> = group
                        .iter()
                        .filter_map(|snapshot| {
                            snapshot
                                .evaluations
                                .iter()
                                .find(|ev| ev.state_var_id == state_var_id)
                        })
                        .collect();
                    aggregate_cell(evaluations, group.len())
                })
                .collect();

            let scores: Vec<f64> = cells.iter().flatten().map(|c| c.score).collect();
            let score_spread = spread(&scores);
            let within_score_spread = cells
                .iter()
                .flatten()
                .map(|cell| cell.score_max - cell.score_min)
                .reduce(f64::max)
                .unwrap_or(0.0);
            let confidence_min = cells
                .iter()
                .flatten()
                .map(|cell| cell.confidence_min)
                .reduce(f64::min)
                .unwrap_or(0.0);
            let confidence_max = cells
                .iter()
                .flatten()
                .map(|cell| cell.confidence_max)
                .reduce(f64::max)
                .unwrap_or(0.0);
            let confidence_spread = confidence_max - confidence_min;
            let row_present_counts: Vec<usize> = cells
                .iter()
                .map(|cell| cell.as_ref().map_or(0, |cell| cell.present_count))
                .collect();
            let spread_signal =
                spread_signal(score_spread, within_score_spread, &row_present_counts);
            let stable_statuses: Vec<StateStatus> = cells
                .iter()
                .flatten()
                .filter(|cell| !cell.status_unstable)
                .map(|cell| cell.status)
                .collect();
            let status_divergence = stable_statuses
                .split_first()
                .is_some_and(|(first, rest)| rest.iter().any(|status| status != first));
            let coverage_divergence = cells.iter().any(|cell| match cell {
                Some(cell) => cell.present_count != cell.replicate_count,
                None => true,
            });
            let status_unstable = cells.iter().flatten().any(|cell| cell.status_unstable);

            ComparisonRow {
                state_var_id,
                cells,
                score_spread,
                within_score_spread,
                confidence_min,
                confidence_max,
                confidence_spread,
                spread_signal,
                status_divergence,
                coverage_divergence,
                status_unstable,
            }
        })
        .collect();

    Ok(ComparisonMatrix {
        experiment_id: ctx.experiment_id,
        variants,
        variant_replicates,
        rows,
        warnings: ctx.warnings,
    })
}

/// Severity rank for the comparable statuses (lower is healthier); `None`
/// for statuses that don't sit on the pass→blocked axis.
fn severity(status: StateStatus) -> Option<u8> {
    match status {
        StateStatus::Pass => Some(0),
        StateStatus::Warn => Some(1),
        StateStatus::Fail => Some(2),
        StateStatus::Blocked => Some(3),
        StateStatus::NotApplicable | StateStatus::Unknown => None,
    }
}

/// The snapshot's experiment id, or `"ungrouped"` when it carries no variant.
fn experiment_id_of(snapshot: &WorkbenchSnapshotV1) -> String {
    snapshot
        .variant
        .as_ref()
        .map(|v| v.experiment_id.clone())
        .unwrap_or_else(|| "ungrouped".to_string())
}

/// One state variable that regressed against the baseline — its status got
/// more severe, or its score dropped.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Regression {
    pub experiment_id: String,
    pub variant: String,
    pub state_var_id: String,
    pub baseline_status: StateStatus,
    pub current_status: StateStatus,
    pub baseline_score: f64,
    pub current_score: f64,
}

/// The outcome of [`diff`] — the regressions found, in current-scan order.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegressionReport {
    pub regressions: Vec<Regression>,
}

impl RegressionReport {
    /// True when nothing regressed — the gate passes.
    pub fn is_clean(&self) -> bool {
        self.regressions.is_empty()
    }
}

/// A score below baseline by more than this is a regression; the epsilon
/// absorbs float round-trip noise (snapshots carry 2-dp scores).
const SCORE_REGRESSION_EPSILON: f64 = 1e-9;

/// Compare `current` against a known-good `baseline` and report every state
/// variable that got worse — status more severe, or score dropped — matched
/// by (experiment, variant, state-var). The basis for `minibench diff`: a
/// snapshot can differ from the last release and the kernel can say it is
/// *worse*, not merely *different*. A
/// cell present in `current` but absent from `baseline` is new, not a
/// regression.
pub fn diff(baseline: &[WorkbenchSnapshotV1], current: &[WorkbenchSnapshotV1]) -> RegressionReport {
    let mut base: HashMap<(String, String, String), (StateStatus, f64)> = HashMap::new();
    for snap in baseline {
        let experiment = experiment_id_of(snap);
        let variant = variant_label(snap);
        for ev in &snap.evaluations {
            base.insert(
                (experiment.clone(), variant.clone(), ev.state_var_id.clone()),
                (ev.status, ev.score),
            );
        }
    }

    let mut regressions = Vec::new();
    for snap in current {
        let experiment = experiment_id_of(snap);
        let variant = variant_label(snap);
        for ev in &snap.evaluations {
            let key = (experiment.clone(), variant.clone(), ev.state_var_id.clone());
            let Some(&(baseline_status, baseline_score)) = base.get(&key) else {
                continue;
            };
            let status_worse = match (severity(baseline_status), severity(ev.status)) {
                (Some(was), Some(now)) => now > was,
                _ => false,
            };
            let score_dropped = ev.score + SCORE_REGRESSION_EPSILON < baseline_score;
            if status_worse || score_dropped {
                regressions.push(Regression {
                    experiment_id: experiment.clone(),
                    variant: variant.clone(),
                    state_var_id: ev.state_var_id.clone(),
                    baseline_status,
                    current_status: ev.status,
                    baseline_score,
                    current_score: ev.score,
                });
            }
        }
    }
    RegressionReport { regressions }
}

#[cfg(test)]
mod tests {
    use super::*;
    use workbench_contract::{
        EvidenceRequirements, Lifecycle, STATE_VAR_REGISTRY_V1, StateVarKind, ValueType,
    };

    #[test]
    fn summarizes_any_tenant_snapshot() {
        // A real miniforge snapshot (one pass, one blocking fail).
        let snap: WorkbenchSnapshotV1 =
            serde_json::from_str(include_str!("../../../fixtures/sample-snapshot.json"))
                .expect("decode fixture");
        let summary = summarize(&snap);
        assert_eq!(summary.total, snap.evaluations.len());
        assert_eq!(
            summary.pass
                + summary.warn
                + summary.fail
                + summary.blocked
                + summary.not_applicable
                + summary.unknown,
            summary.total
        );
        assert!(summary.fail >= 1, "fixture has a failing evaluation");
        assert!(
            summary
                .blocking
                .iter()
                .any(|id| id.contains("bundle_complete")),
            "the failing evaluation blocks progression"
        );
    }

    fn missing_provenance_warnings() -> Vec<CompareWarning> {
        vec![
            CompareWarning::MissingSourceHashes,
            CompareWarning::MissingPolicyProvenance,
            CompareWarning::MissingEvaluatorProvenance,
        ]
    }

    fn stamp_variant_axes_provenance(snapshot: &mut WorkbenchSnapshotV1) {
        let variant = snapshot.variant.as_mut().expect("fixture has variant");
        variant.axes.insert(
            PROVENANCE_KEY_POLICY_HASH.to_string(),
            "sha256:policy".to_string(),
        );
        variant.axes.insert(
            PROVENANCE_KEY_EVALUATOR_VERSION.to_string(),
            "career-evaluator/2026.07.03".to_string(),
        );
    }

    #[test]
    fn summarize_only_blocks_failing_or_blocked_exact_gate_effects() {
        let mut snap: WorkbenchSnapshotV1 =
            serde_json::from_str(include_str!("../../../fixtures/sample-snapshot.json"))
                .expect("decode fixture");
        snap.evaluations[0].gate_effect = GATE_EFFECT_BLOCKS_TRANSITION.to_string();
        snap.evaluations[1].gate_effect = "unblocked_review".to_string();

        let summary = summarize(&snap);

        assert!(
            summary.blocking.is_empty(),
            "pass status and non-canonical strings must not block"
        );
    }

    #[test]
    fn summarize_with_registry_resolves_gate_effects_from_registry() {
        let mut snap: WorkbenchSnapshotV1 =
            serde_json::from_str(include_str!("../../../fixtures/sample-snapshot.json"))
                .expect("decode fixture");
        let registry = registry_for_snapshot(&snap);
        snap.evaluations[1].gate_effect = "none".to_string();

        let summary = summarize_with_registry(&snap, &registry).expect("matching registry");

        assert_eq!(summary.blocking, vec!["miniforge.evidence.bundle_complete"]);
    }

    #[test]
    fn summarize_with_registry_rejects_wrong_registry() {
        let snap: WorkbenchSnapshotV1 =
            serde_json::from_str(include_str!("../../../fixtures/sample-snapshot.json"))
                .expect("decode fixture");
        let mut registry = registry_for_snapshot(&snap);
        registry.version = "2026.06.06.2".to_string();

        let err = summarize_with_registry(&snap, &registry).expect_err("wrong registry rejected");

        assert!(matches!(err, SummaryError::RegistryRefMismatch { .. }));
    }

    fn registry_for_snapshot(snapshot: &WorkbenchSnapshotV1) -> StateVarRegistry {
        StateVarRegistry {
            schema_version: STATE_VAR_REGISTRY_V1.to_string(),
            registry_id: snapshot.registry_ref.registry_id.clone(),
            version: snapshot.registry_ref.version.clone(),
            product: snapshot.product.clone(),
            state_vars: snapshot
                .evaluations
                .iter()
                .map(|evaluation| {
                    let mut gate_effects = BTreeMap::new();
                    gate_effects
                        .insert(STATUS_KEY_FAIL.to_string(), evaluation.gate_effect.clone());
                    StateVariable {
                        id: evaluation.state_var_id.clone(),
                        version: snapshot.registry_ref.version.clone(),
                        product: snapshot.product.clone(),
                        area: "test".to_string(),
                        kind: StateVarKind::Quality,
                        description: "test state variable".to_string(),
                        value_type: ValueType::Number,
                        thresholds: BTreeMap::new(),
                        evidence_requirements: EvidenceRequirements {
                            required_refs: Vec::new(),
                            min_count: None,
                            must_include_hash: None,
                            must_include_source_role: None,
                            freshness_sla_hours: None,
                        },
                        score_components: Vec::new(),
                        gate_effects,
                        lifecycle: Lifecycle::Active,
                        owner: None,
                        notes: None,
                    }
                })
                .collect(),
        }
    }

    #[test]
    fn compares_permutations_and_flags_divergence() {
        // Same task (career.lens.acme-l4-eval), two variants.
        let opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/opus-semantic.json"
        ))
        .expect("decode opus variant");
        let haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/haiku-mechanical.json"
        ))
        .expect("decode haiku variant");

        let matrix = compare(&[opus, haiku]).expect("valid comparison");

        assert_eq!(matrix.experiment_id, "career.lens.acme-l4-eval");
        assert_eq!(matrix.variants, vec!["opus+semantic", "haiku+mechanical"]);
        assert_eq!(matrix.variant_replicates, vec![1, 1]);
        assert_eq!(matrix.warnings, missing_provenance_warnings());

        // The grounding row diverges (pass vs fail), with a real spread;
        // that's the signal "semantic vs mechanical changed the outcome".
        let grounded = matrix
            .rows
            .iter()
            .find(|r| r.state_var_id == "career.lens.report_grounded")
            .expect("grounding row present");
        assert!(grounded.status_divergence, "pass vs fail across variants");
        assert!(
            (grounded.score_spread - 0.45).abs() < 1e-9,
            "0.88 vs 0.43 spread"
        );
        assert_eq!(grounded.cells.len(), 2);
        assert!(grounded.cells.iter().all(Option::is_some));
        assert_eq!(grounded.spread_signal, SpreadSignal::SingleRun);
        assert!(!grounded.coverage_divergence);
        assert!(!grounded.status_unstable);
    }

    #[test]
    fn groups_replicates_and_reports_within_variant_spread() {
        let opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/opus-semantic.json"
        ))
        .expect("decode opus variant");
        let haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/haiku-mechanical.json"
        ))
        .expect("decode haiku variant");
        let mut opus_replicate = opus.clone();
        opus_replicate.snapshot_id = "wb-career-opus-semantic-0002".to_string();
        opus_replicate.run_id = "run-opus-0002".to_string();
        opus_replicate.evaluations[0].score = 0.92;
        opus_replicate.evaluations[0].confidence = 0.9;

        let matrix = compare(&[opus, opus_replicate, haiku]).expect("valid comparison");

        assert_eq!(matrix.variants, vec!["opus+semantic", "haiku+mechanical"]);
        assert_eq!(matrix.variant_replicates, vec![2, 1]);

        let grounded = matrix
            .rows
            .iter()
            .find(|r| r.state_var_id == "career.lens.report_grounded")
            .expect("grounding row present");
        let opus_cell = grounded.cells[0]
            .as_ref()
            .expect("opus aggregate cell present");
        assert_eq!(opus_cell.status, StateStatus::Pass);
        assert_eq!(opus_cell.present_count, 2);
        assert_eq!(opus_cell.replicate_count, 2);
        assert!((opus_cell.score - 0.9).abs() < 1e-9);
        assert!((opus_cell.score_min - 0.88).abs() < 1e-9);
        assert!((opus_cell.score_max - 0.92).abs() < 1e-9);
        assert!((grounded.within_score_spread - 0.04).abs() < 1e-9);
        assert!((grounded.score_spread - 0.47).abs() < 1e-9);
        assert_eq!(grounded.spread_signal, SpreadSignal::BetweenExceedsWithin);
    }

    #[test]
    fn flags_spread_confounded_by_within_variant_noise() {
        let opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/opus-semantic.json"
        ))
        .expect("decode opus variant");
        let haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/haiku-mechanical.json"
        ))
        .expect("decode haiku variant");
        let mut opus_noisy_replicate = opus.clone();
        opus_noisy_replicate.snapshot_id = "wb-career-opus-semantic-0002".to_string();
        opus_noisy_replicate.run_id = "run-opus-0002".to_string();
        opus_noisy_replicate.evaluations[0].score = 0.10;

        let matrix = compare(&[opus, opus_noisy_replicate, haiku]).expect("valid comparison");

        let grounded = matrix
            .rows
            .iter()
            .find(|r| r.state_var_id == "career.lens.report_grounded")
            .expect("grounding row present");
        assert_eq!(grounded.spread_signal, SpreadSignal::WithinMatchesBetween);
        assert!(
            grounded.within_score_spread >= grounded.score_spread,
            "within-variant spread should dominate the between-variant mean spread"
        );
    }

    #[test]
    fn row_spread_uses_present_replicates_not_variant_replicates() {
        let opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/opus-semantic.json"
        ))
        .expect("decode opus variant");
        let haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/haiku-mechanical.json"
        ))
        .expect("decode haiku variant");
        let mut opus_missing_replicate = opus.clone();
        opus_missing_replicate.snapshot_id = "wb-career-opus-semantic-0002".to_string();
        opus_missing_replicate.run_id = "run-opus-0002".to_string();
        opus_missing_replicate.evaluations.clear();

        let matrix = compare(&[opus, opus_missing_replicate, haiku]).expect("valid comparison");

        let grounded = matrix
            .rows
            .iter()
            .find(|r| r.state_var_id == "career.lens.report_grounded")
            .expect("grounding row present");
        assert!(
            grounded.coverage_divergence,
            "one variant replicate is missing the row"
        );
        assert_eq!(grounded.spread_signal, SpreadSignal::SingleRun);
    }

    #[test]
    fn missing_evaluations_are_coverage_divergence() {
        let opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/opus-semantic.json"
        ))
        .expect("decode opus variant");
        let mut haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/haiku-mechanical.json"
        ))
        .expect("decode haiku variant");
        haiku
            .evaluations
            .retain(|ev| ev.state_var_id != "career.lens.report_grounded");

        let matrix = compare(&[opus, haiku]).expect("valid comparison");

        let traceability = matrix
            .rows
            .iter()
            .find(|r| r.state_var_id == "career.lens.report_grounded")
            .expect("grounding row present");
        assert!(
            traceability.coverage_divergence,
            "missing evaluation is a result"
        );
        assert!(
            !traceability.status_divergence,
            "coverage has its own signal"
        );
        assert!(traceability.cells[1].is_none());
    }

    #[test]
    fn rows_report_confidence_range_across_variants() {
        let mut opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/opus-semantic.json"
        ))
        .expect("decode opus variant");
        let mut haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/haiku-mechanical.json"
        ))
        .expect("decode haiku variant");
        opus.evaluations[0].confidence = 0.51;
        haiku.evaluations[0].confidence = 0.99;

        let matrix = compare(&[opus, haiku]).expect("valid comparison");

        let grounded = matrix
            .rows
            .iter()
            .find(|r| r.state_var_id == "career.lens.report_grounded")
            .expect("grounding row present");
        assert!((grounded.confidence_min - 0.51).abs() < 1e-9);
        assert!((grounded.confidence_max - 0.99).abs() < 1e-9);
        assert!((grounded.confidence_spread - 0.48).abs() < 1e-9);
    }

    #[test]
    fn rejects_mixed_experiments() {
        let opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/opus-semantic.json"
        ))
        .expect("decode opus variant");
        let mut haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/haiku-mechanical.json"
        ))
        .expect("decode haiku variant");
        haiku
            .variant
            .as_mut()
            .expect("fixture has variant")
            .experiment_id = "career.lens.other-eval".to_string();

        let err = compare(&[opus, haiku]).expect_err("mixed experiments rejected");

        assert!(matches!(err, CompareError::MixedExperimentIds { .. }));
    }

    #[test]
    fn rejects_mixed_registry_refs() {
        let opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/opus-semantic.json"
        ))
        .expect("decode opus variant");
        let mut haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/haiku-mechanical.json"
        ))
        .expect("decode haiku variant");
        haiku.registry_ref.version = "2026.06.06.2".to_string();

        let err = compare(&[opus, haiku]).expect_err("mixed registries rejected");

        assert!(matches!(err, CompareError::MixedRegistryRefs { .. }));
    }

    #[test]
    fn rejects_duplicate_state_var_ids() {
        let mut opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/opus-semantic.json"
        ))
        .expect("decode opus variant");
        opus.evaluations.push(opus.evaluations[0].clone());

        let err = compare(&[opus]).expect_err("duplicate evaluations rejected");

        assert!(matches!(err, CompareError::DuplicateStateVarId { .. }));
    }

    #[test]
    fn rejects_duplicate_run_ids_inside_variant_replicates() {
        let opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/opus-semantic.json"
        ))
        .expect("decode opus variant");
        let duplicate = opus.clone();

        let err = compare(&[opus, duplicate]).expect_err("duplicate run ids rejected");

        assert!(matches!(err, CompareError::DuplicateVariantRun { .. }));
    }

    #[test]
    fn rejects_mismatched_source_hashes() {
        let mut opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/opus-semantic.json"
        ))
        .expect("decode opus variant");
        let mut haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/haiku-mechanical.json"
        ))
        .expect("decode haiku variant");
        opus.source_hashes = Some(vec!["sha256:aaaaaaaa".to_string()]);
        haiku.source_hashes = Some(vec!["sha256:bbbbbbbb".to_string()]);

        let err = compare(&[opus, haiku]).expect_err("mismatched inputs rejected");

        assert!(matches!(err, CompareError::MixedSourceHashes { .. }));
    }

    #[test]
    fn accepts_uniform_provenance_in_variant_axes() {
        let mut opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/opus-semantic.json"
        ))
        .expect("decode opus variant");
        let mut haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/haiku-mechanical.json"
        ))
        .expect("decode haiku variant");
        stamp_variant_axes_provenance(&mut opus);
        stamp_variant_axes_provenance(&mut haiku);

        let matrix = compare(&[opus, haiku]).expect("uniform provenance is comparable");

        assert_eq!(matrix.warnings, vec![CompareWarning::MissingSourceHashes]);
    }

    #[test]
    fn accepts_uniform_provenance_in_metadata() {
        let mut opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/opus-semantic.json"
        ))
        .expect("decode opus variant");
        let mut haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/haiku-mechanical.json"
        ))
        .expect("decode haiku variant");
        let metadata = serde_json::json!({
            "provenance": {
                "policy_version": "career-policy/2026.07.03",
                "evaluator_hash": "sha256:evaluator"
            }
        });
        opus.metadata = Some(metadata.clone());
        haiku.metadata = Some(metadata);

        let matrix = compare(&[opus, haiku]).expect("uniform metadata provenance is comparable");

        assert_eq!(matrix.warnings, vec![CompareWarning::MissingSourceHashes]);
    }

    #[test]
    fn rejects_mixed_policy_provenance() {
        let mut opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/opus-semantic.json"
        ))
        .expect("decode opus variant");
        let mut haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/haiku-mechanical.json"
        ))
        .expect("decode haiku variant");
        stamp_variant_axes_provenance(&mut opus);
        stamp_variant_axes_provenance(&mut haiku);
        haiku
            .variant
            .as_mut()
            .expect("fixture has variant")
            .axes
            .insert(
                PROVENANCE_KEY_POLICY_HASH.to_string(),
                "sha256:other-policy".to_string(),
            );

        let err = compare(&[opus, haiku]).expect_err("mixed policy provenance rejected");

        assert!(matches!(err, CompareError::MixedProvenance { .. }));
    }

    #[test]
    fn rejects_partially_missing_evaluator_provenance() {
        let opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/opus-semantic.json"
        ))
        .expect("decode opus variant");
        let mut haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/haiku-mechanical.json"
        ))
        .expect("decode haiku variant");
        haiku
            .variant
            .as_mut()
            .expect("fixture has variant")
            .axes
            .insert(
                PROVENANCE_KEY_EVALUATOR_VERSION.to_string(),
                "career-evaluator/2026.07.03".to_string(),
            );

        let err = compare(&[opus, haiku]).expect_err("partial evaluator provenance rejected");

        assert!(matches!(err, CompareError::MixedProvenance { .. }));
    }

    #[test]
    fn diff_is_clean_against_self_and_flags_a_worsened_cell() {
        let baseline: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
            "../../../fixtures/experiments/opus-semantic.json"
        ))
        .expect("decode baseline");

        // Identical current → nothing regressed.
        let same = std::slice::from_ref(&baseline);
        assert!(diff(same, same).is_clean());

        // Worsen the status and drop the score on a copy → exactly one
        // regression, on the same (experiment, variant, state-var) cell.
        let mut regressed = baseline.clone();
        regressed.evaluations[0].status = StateStatus::Fail;
        regressed.evaluations[0].score = 0.10;
        let report = diff(
            std::slice::from_ref(&baseline),
            std::slice::from_ref(&regressed),
        );

        assert!(!report.is_clean());
        assert_eq!(report.regressions.len(), 1);
        assert_eq!(report.regressions[0].baseline_status, StateStatus::Pass);
        assert_eq!(report.regressions[0].current_status, StateStatus::Fail);
    }
}
