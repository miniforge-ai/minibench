// Title: Minibench
// Subtitle: kernel — comparison matrix
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

// Comparison matrix — the permutation harness
//
// The workbench exists to compare PERMUTATIONS of a task (workflow /
// prompt / model / mechanical-vs-semantic). Given the snapshots for one
// experiment, `compare` lays them out as a matrix: rows are state
// variables, columns are variants, cells are score/status. Per row it
// flags where the variants actually diverge — that's the signal the
// shell highlights so you can see which config changed which outcome.
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use workbench_contract::{
    StateEvaluation, StateStatus, StateVarRegistry, StateVariable, WorkbenchSnapshotV1,
};

use crate::provenance::{
    EVALUATOR_PROVENANCE_KEYS, POLICY_PROVENANCE_KEYS, ProvenanceMismatch, validate_provenance_key,
};
use crate::snapshot::{
    RegistryMismatch, check_registry, experiment_id, registry_ref_key, variant_label,
};
use crate::stats::{
    SCORE_SPREAD_EPSILON, SpreadSignal, majority_status, mean, meaningful_score_spread_threshold,
    spread, spread_signal, standard_deviation,
};

// ------------------------------------------------------------------ Layer 0

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
    /// True when registry threshold bands say the score spread is large
    /// enough to be worth reviewing even without status divergence.
    pub meaningful_score_spread: bool,
    /// Interpretation of score spread relative to measured replicate noise.
    pub spread_signal: SpreadSignal,
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
    RegistryRefMismatch {
        expected: String,
        found: String,
    },
    RegistryProductMismatch {
        expected: String,
        found: String,
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
            Self::RegistryRefMismatch { expected, found } => {
                write!(
                    f,
                    "registry {found} does not match comparison registry {expected}"
                )
            }
            Self::RegistryProductMismatch { expected, found } => write!(
                f,
                "registry product {found} does not match comparison product {expected}"
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

impl CompareError {
    /// Map a provenance uniformity failure into the comparison error.
    fn from_provenance_mismatch(mismatch: ProvenanceMismatch) -> Self {
        Self::MixedProvenance {
            key: mismatch.key,
            expected: mismatch.expected,
            found: mismatch.found,
            snapshot_id: mismatch.snapshot_id,
        }
    }
}

struct CompareContext {
    experiment_id: String,
    warnings: Vec<CompareWarning>,
}

fn normalized_source_hashes(snapshot: &WorkbenchSnapshotV1) -> Option<Vec<String>> {
    snapshot.source_hashes.as_ref().map(|hashes| {
        let mut normalized = hashes.clone();
        normalized.sort();
        normalized
    })
}

// ------------------------------------------------------------------ Layer 1

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

    let provenance_group = |keys: &[&str]| -> Result<bool, CompareError> {
        let mut has_group_provenance = false;
        for key in keys {
            has_group_provenance |= validate_provenance_key(snapshots, key)
                .map_err(CompareError::from_provenance_mismatch)?;
        }
        Ok(has_group_provenance)
    };

    let mut warnings = Vec::new();
    if missing_source_hashes {
        warnings.push(CompareWarning::MissingSourceHashes);
    }
    if !provenance_group(POLICY_PROVENANCE_KEYS)? {
        warnings.push(CompareWarning::MissingPolicyProvenance);
    }
    if !provenance_group(EVALUATOR_PROVENANCE_KEYS)? {
        warnings.push(CompareWarning::MissingEvaluatorProvenance);
    }

    Ok(CompareContext {
        experiment_id: expected_experiment_id,
        warnings,
    })
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

fn validate_comparison_registry(
    snapshots: &[WorkbenchSnapshotV1],
    registry: &StateVarRegistry,
) -> Result<(), CompareError> {
    let Some(first) = snapshots.first() else {
        return Err(CompareError::EmptyInput);
    };
    check_registry(first, registry).map_err(|mismatch| match mismatch {
        RegistryMismatch::Ref { expected, found } => {
            CompareError::RegistryRefMismatch { expected, found }
        }
        RegistryMismatch::Product { expected, found } => {
            CompareError::RegistryProductMismatch { expected, found }
        }
    })
}

// ------------------------------------------------------------------ Layer 2

pub(crate) fn compare_inner(
    snapshots: &[WorkbenchSnapshotV1],
    registry: Option<&StateVarRegistry>,
) -> Result<ComparisonMatrix, CompareError> {
    let ctx = validate_comparable(snapshots)?;
    if let Some(registry) = registry {
        validate_comparison_registry(snapshots, registry)?;
    }
    let state_vars: BTreeMap<&str, &StateVariable> = registry
        .map(|registry| {
            registry
                .state_vars
                .iter()
                .map(|state_var| (state_var.id.as_str(), state_var))
                .collect()
        })
        .unwrap_or_default();

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
            let meaningful_score_spread = state_vars
                .get(state_var_id.as_str())
                .and_then(|state_var| meaningful_score_spread_threshold(state_var))
                .is_some_and(|threshold| score_spread + SCORE_SPREAD_EPSILON >= threshold);
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
                meaningful_score_spread,
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
