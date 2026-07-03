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
use workbench_contract::{RegistryRef, StateEvaluation, StateStatus, WorkbenchSnapshotV1};

/// Substring marking a gate effect that blocks progression. Gate-effect
/// strings are product-owned (`blocks_transition`, ...); a snapshot
/// whose effect contains this marker holds the run back.
const BLOCKING_GATE_MARKER: &str = "block";
/// Experiment id used for one-off snapshots with no variant tag.
const UNGROUPED_EXPERIMENT_ID: &str = "ungrouped";
/// Standard deviation is only meaningful once at least two replicate
/// observations exist.
const MIN_REPLICATES_FOR_SPREAD: usize = 2;
/// Stable ordering used only to make tied status votes deterministic.
const STATE_STATUS_ORDER: &[StateStatus] = &[
    StateStatus::Pass,
    StateStatus::Warn,
    StateStatus::Fail,
    StateStatus::Blocked,
    StateStatus::NotApplicable,
    StateStatus::Unknown,
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

/// Roll up a snapshot's evaluations by status and collect the
/// state variables whose gate effect blocks progression.
pub fn summarize(snapshot: &WorkbenchSnapshotV1) -> RunSummary {
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
        if ev.gate_effect.contains(BLOCKING_GATE_MARKER) {
            summary.blocking.push(ev.state_var_id.clone());
        }
    }
    summary
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
    /// True when stable majority statuses differ across variants.
    pub status_divergence: bool,
    /// True when any variant or replicate did not produce this row.
    pub coverage_divergence: bool,
    /// True when at least one variant has no unique majority status.
    pub status_unstable: bool,
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
                expected: expected_experiment_id,
                found: found_experiment_id,
                snapshot_id: snapshot.snapshot_id.clone(),
            });
        }

        if snapshot.product != expected_product {
            return Err(CompareError::MixedProducts {
                expected: expected_product,
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

    let warnings = if missing_source_hashes {
        vec![CompareWarning::MissingSourceHashes]
    } else {
        Vec::new()
    };

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
                status_divergence,
                coverage_divergence,
                status_unstable,
            }
        })
        .collect();

    let variant_replicates = variants
        .iter()
        .map(|variant| groups[variant].len())
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
        assert_eq!(matrix.warnings, vec![CompareWarning::MissingSourceHashes]);

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
