// Title: Minibench
// Subtitle: kernel — regression diff against a baseline
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

//! Compare a current scan against a known-good baseline and report
//! every state variable that got worse — the basis for `minibench diff`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use workbench_contract::{StateStatus, WorkbenchSnapshotV1};

use crate::snapshot::{experiment_id, variant_label};

// ------------------------------------------------------------------ Layer 0

/// Severity rank for the comparable statuses (lower is healthier); `None`
/// for statuses that don't sit on the pass→blocked axis.
pub(crate) fn severity(status: StateStatus) -> Option<u8> {
    match status {
        StateStatus::Pass => Some(0),
        StateStatus::Warn => Some(1),
        StateStatus::Fail => Some(2),
        StateStatus::Blocked => Some(3),
        StateStatus::NotApplicable | StateStatus::Unknown => None,
    }
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
pub(crate) const SCORE_REGRESSION_EPSILON: f64 = 1e-9;

// ------------------------------------------------------------------ Layer 1

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
        let experiment = experiment_id(snap);
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
        let experiment = experiment_id(snap);
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
