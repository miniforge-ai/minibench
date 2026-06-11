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

use serde::{Deserialize, Serialize};
use workbench_contract::{StateStatus, WorkbenchSnapshotV1};

/// Substring marking a gate effect that blocks progression. Gate-effect
/// strings are product-owned (`blocks_transition`, ...); a snapshot
/// whose effect contains this marker holds the run back.
const BLOCKING_GATE_MARKER: &str = "block";

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
    pub status: StateStatus,
    pub score: f64,
    pub confidence: f64,
}

/// One state variable across every variant in the experiment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComparisonRow {
    pub state_var_id: String,
    /// Aligned to `ComparisonMatrix::variants`; `None` where a variant
    /// produced no evaluation for this state variable.
    pub cells: Vec<Option<ComparisonCell>>,
    /// max − min score across the variants that produced a cell.
    pub score_spread: f64,
    /// True when the present cells do not all share one status.
    pub status_divergence: bool,
}

/// A run matrix for one experiment: state variables × variants.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComparisonMatrix {
    pub experiment_id: String,
    /// Column labels, in `snapshots` order (variant label, else run id).
    pub variants: Vec<String>,
    pub rows: Vec<ComparisonRow>,
}

fn variant_label(snapshot: &WorkbenchSnapshotV1) -> String {
    snapshot
        .variant
        .as_ref()
        .map(|v| v.label.clone())
        .unwrap_or_else(|| snapshot.run_id.clone())
}

/// Lay out the snapshots of one experiment as a comparison matrix.
/// Variants keep the input order; state-variable rows keep first-seen
/// order across the snapshots so the matrix is stable.
pub fn compare(snapshots: &[WorkbenchSnapshotV1]) -> ComparisonMatrix {
    let experiment_id = snapshots
        .iter()
        .find_map(|s| s.variant.as_ref().map(|v| v.experiment_id.clone()))
        .unwrap_or_else(|| "ungrouped".to_string());

    let variants: Vec<String> = snapshots.iter().map(variant_label).collect();

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
            let cells: Vec<Option<ComparisonCell>> = snapshots
                .iter()
                .map(|snap| {
                    snap.evaluations
                        .iter()
                        .find(|ev| ev.state_var_id == state_var_id)
                        .map(|ev| ComparisonCell {
                            status: ev.status,
                            score: ev.score,
                            confidence: ev.confidence,
                        })
                })
                .collect();

            let scores: Vec<f64> = cells.iter().flatten().map(|c| c.score).collect();
            let score_spread = match (
                scores.iter().cloned().reduce(f64::max),
                scores.iter().cloned().reduce(f64::min),
            ) {
                (Some(hi), Some(lo)) => hi - lo,
                _ => 0.0,
            };

            let mut statuses = cells.iter().flatten().map(|c| c.status);
            let status_divergence = match statuses.next() {
                Some(first) => statuses.any(|s| s != first),
                None => false,
            };

            ComparisonRow {
                state_var_id,
                cells,
                score_spread,
                status_divergence,
            }
        })
        .collect();

    ComparisonMatrix {
        experiment_id,
        variants,
        rows,
    }
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
        assert_eq!(summary.pass + summary.warn + summary.fail, summary.total);
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

        let matrix = compare(&[opus, haiku]);

        assert_eq!(matrix.experiment_id, "career.lens.acme-l4-eval");
        assert_eq!(matrix.variants, vec!["opus+semantic", "haiku+mechanical"]);

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
