// Title: Minibench
// Subtitle: kernel — human corrections as labeled gate expectations
// Author: Christopher Lester
// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

//! A reviewer records what one comparison cell SHOULD read and why, and
//! that labeled expectation — not the raw frozen baseline — becomes what
//! the regression gate enforces for that cell. Wholesale baseline
//! replacement stops being the only way to accept a change, and every
//! accepted change carries provenance: who corrected it, when, and why.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;

use serde::{Deserialize, Serialize};
use workbench_contract::{StateStatus, WorkbenchSnapshotV1};

use crate::regression::{SCORE_REGRESSION_EPSILON, severity};
use crate::snapshot::{experiment_id, variant_label};

// ------------------------------------------------------------------ Layer 0
/// The (experiment, variant, state-var) coordinate a correction targets —
/// the same key [`diff`] matches cells on.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CorrectionKey {
    pub experiment_id: String,
    pub variant_label: String,
    pub state_var_id: String,
}

impl fmt::Display for CorrectionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} [{}] {}",
            self.experiment_id, self.variant_label, self.state_var_id
        )
    }
}

/// One human-reviewed expectation for a single comparison cell. Stored as
/// one JSON file per correction so each override is individually
/// reviewable in a PR diff.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CorrectionV1 {
    pub experiment_id: String,
    pub variant_label: String,
    pub state_var_id: String,
    /// The status the gate should expect for this cell, in place of the
    /// baseline status.
    pub expected_status: StateStatus,
    /// The score floor the gate should enforce. When absent, the baseline
    /// score (if the cell exists in the baseline) remains the floor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_score: Option<f64>,
    /// Why the reviewer overrode the baseline. Required and non-empty — a
    /// correction without a why is refused.
    pub rationale: String,
    /// Who recorded the correction. Required and non-empty.
    pub corrected_by: String,
    /// RFC 3339 timestamp of when the correction was recorded.
    pub corrected_at: String,
    /// The snapshot the reviewer was looking at when they recorded this.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_id: Option<String>,
}

impl CorrectionV1 {
    /// The cell coordinate this correction overrides.
    pub fn key(&self) -> CorrectionKey {
        CorrectionKey {
            experiment_id: self.experiment_id.clone(),
            variant_label: self.variant_label.clone(),
            state_var_id: self.state_var_id.clone(),
        }
    }

    /// Refuse records that carry no provenance: a blank rationale or a
    /// blank corrector defeats the point of the correction loop.
    pub fn validate(&self) -> Result<(), CorrectionError> {
        if self.rationale.trim().is_empty() {
            return Err(CorrectionError::EmptyRationale { key: self.key() });
        }
        if self.corrected_by.trim().is_empty() {
            return Err(CorrectionError::EmptyCorrectedBy { key: self.key() });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CorrectionError {
    EmptyRationale { key: CorrectionKey },
    EmptyCorrectedBy { key: CorrectionKey },
    DuplicateKey { key: CorrectionKey },
}

impl fmt::Display for CorrectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRationale { key } => {
                write!(f, "correction {key} has an empty rationale; refused")
            }
            Self::EmptyCorrectedBy { key } => {
                write!(f, "correction {key} has an empty corrected_by; refused")
            }
            Self::DuplicateKey { key } => {
                write!(f, "duplicate correction for {key}; one correction per cell")
            }
        }
    }
}

impl std::error::Error for CorrectionError {}

/// Corrections indexed by cell coordinate. Construction validates every
/// record and rejects duplicate keys — two corrections for the same cell
/// are a conflict to resolve, never a silent last-wins.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CorrectionSet {
    by_key: BTreeMap<CorrectionKey, CorrectionV1>,
}

impl CorrectionSet {
    pub fn new(corrections: Vec<CorrectionV1>) -> Result<Self, CorrectionError> {
        let mut by_key = BTreeMap::new();
        for correction in corrections {
            correction.validate()?;
            let key = correction.key();
            if by_key.insert(key.clone(), correction).is_some() {
                return Err(CorrectionError::DuplicateKey { key });
            }
        }
        Ok(Self { by_key })
    }

    pub fn is_empty(&self) -> bool {
        self.by_key.is_empty()
    }

    pub fn len(&self) -> usize {
        self.by_key.len()
    }

    fn get(
        &self,
        experiment_id: &str,
        variant_label: &str,
        state_var_id: &str,
    ) -> Option<&CorrectionV1> {
        self.by_key.get(&CorrectionKey {
            experiment_id: experiment_id.to_string(),
            variant_label: variant_label.to_string(),
            state_var_id: state_var_id.to_string(),
        })
    }

    fn keys(&self) -> impl Iterator<Item = &CorrectionKey> {
        self.by_key.keys()
    }
}

/// A cell that failed its expectation in a corrections-aware diff. The
/// expectation defaults to the baseline cell and is overridden by a
/// human correction where one exists (`corrected` records which).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CorrectedRegression {
    pub experiment_id: String,
    pub variant: String,
    pub state_var_id: String,
    pub expected_status: StateStatus,
    /// The enforced score floor; `None` when the correction gave no score
    /// and the cell has no baseline score to fall back to.
    pub expected_score: Option<f64>,
    pub current_status: StateStatus,
    pub current_score: f64,
    /// True when the expectation came from a human correction rather than
    /// the raw baseline — provenance for the report.
    pub corrected: bool,
}

/// The outcome of [`diff_with_corrections`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CorrectedDiffReport {
    pub regressions: Vec<CorrectedRegression>,
    /// Cell coordinates judged against a human correction, whether or not
    /// they regressed — the report's provenance trail.
    pub applied: Vec<CorrectionKey>,
    /// Corrections whose key matched nothing in baseline or current.
    /// Surfaced as warnings so a renamed or removed cell cannot silently
    /// strand its correction.
    pub stale: Vec<CorrectionKey>,
}

impl CorrectedDiffReport {
    /// True when nothing regressed — the gate passes. Stale corrections
    /// warn but do not fail the gate.
    pub fn is_clean(&self) -> bool {
        self.regressions.is_empty()
    }
}

// ------------------------------------------------------------------ Layer 1

/// [`diff`] with human corrections layered over the baseline: where a
/// correction exists for a (experiment, variant, state-var) cell, its
/// `expected_status` / `expected_score` replace the baseline cell as the
/// expectation the current run is judged against. A correction with no
/// `expected_score` overrides only the status; the baseline score (when
/// the cell exists in baseline) remains the floor. Score drops are judged
/// with the same epsilon as [`diff`]. A correction matching a cell absent
/// from the baseline still judges the current cell — the correction IS
/// the expectation, no frozen baseline required.
pub fn diff_with_corrections(
    baseline: &[WorkbenchSnapshotV1],
    current: &[WorkbenchSnapshotV1],
    corrections: &CorrectionSet,
) -> CorrectedDiffReport {
    let mut base: HashMap<(String, String, String), (StateStatus, f64)> = HashMap::new();
    for snap in baseline {
        let experiment = experiment_id(snap);
        let variant = variant_label(snap);
        for ev in &snap.evaluations {
            base.insert(
                (experiment.clone(), variant.clone(), ev.state_var_id.clone()),
                (ev.status, ev.score),
            );
        }
    }

    let mut current_keys: BTreeSet<(String, String, String)> = BTreeSet::new();
    let mut applied: Vec<CorrectionKey> = Vec::new();
    let mut regressions = Vec::new();
    for snap in current {
        let experiment = experiment_id(snap);
        let variant = variant_label(snap);
        for ev in &snap.evaluations {
            let key = (experiment.clone(), variant.clone(), ev.state_var_id.clone());
            current_keys.insert(key.clone());
            let baseline_cell = base.get(&key).copied();
            let correction = corrections.get(&experiment, &variant, &ev.state_var_id);
            let (expected_status, expected_score, corrected) = match correction {
                Some(correction) => {
                    let correction_key = correction.key();
                    if !applied.contains(&correction_key) {
                        applied.push(correction_key);
                    }
                    (
                        correction.expected_status,
                        correction
                            .expected_score
                            .or(baseline_cell.map(|(_, score)| score)),
                        true,
                    )
                }
                None => match baseline_cell {
                    // A cell new in `current` with no correction is new,
                    // not a regression — same as [`diff`].
                    None => continue,
                    Some((status, score)) => (status, Some(score), false),
                },
            };
            let status_worse = match (severity(expected_status), severity(ev.status)) {
                (Some(was), Some(now)) => now > was,
                _ => false,
            };
            let score_dropped =
                expected_score.is_some_and(|floor| ev.score + SCORE_REGRESSION_EPSILON < floor);
            if status_worse || score_dropped {
                regressions.push(CorrectedRegression {
                    experiment_id: experiment.clone(),
                    variant: variant.clone(),
                    state_var_id: ev.state_var_id.clone(),
                    expected_status,
                    expected_score,
                    current_status: ev.status,
                    current_score: ev.score,
                    corrected,
                });
            }
        }
    }

    let stale = corrections
        .keys()
        .filter(|key| {
            let tuple = (
                key.experiment_id.clone(),
                key.variant_label.clone(),
                key.state_var_id.clone(),
            );
            !base.contains_key(&tuple) && !current_keys.contains(&tuple)
        })
        .cloned()
        .collect();

    CorrectedDiffReport {
        regressions,
        applied,
        stale,
    }
}
