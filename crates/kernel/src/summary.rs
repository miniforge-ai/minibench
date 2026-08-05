// Title: Minibench
// Subtitle: kernel — run summary roll-up
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

//! Roll up one snapshot's evaluations by status and surface the state
//! variables whose gate effect blocks progression.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use workbench_contract::{
    StateEvaluation, StateStatus, StateVarRegistry, StateVariable, WorkbenchSnapshotV1,
};

use crate::snapshot::{RegistryMismatch, check_registry};

// ------------------------------------------------------------------ Layer 0

/// Canonical registry gate effect for a state variable that blocks run
/// progression. Registries may add other effects for review/repair, but
/// blocking has to be exact here so unrelated product strings containing
/// "block" are not elevated accidentally.
pub(crate) const GATE_EFFECT_BLOCKS_TRANSITION: &str = "blocks_transition";
/// Gate effects the generic kernel treats as progression-blocking when
/// no registry is supplied. Registry-aware summary resolves the effect
/// from `StateVariable::gate_effects` first.
const BLOCKING_GATE_EFFECTS: &[&str] = &[GATE_EFFECT_BLOCKS_TRANSITION];
/// Status key for `StateStatus::Pass` inside registry `gate_effects`.
pub(crate) const STATUS_KEY_PASS: &str = "pass";
/// Status key for `StateStatus::Warn` inside registry `gate_effects`.
pub(crate) const STATUS_KEY_WARN: &str = "warn";
/// Status key for `StateStatus::Fail` inside registry `gate_effects`.
pub(crate) const STATUS_KEY_FAIL: &str = "fail";
/// Status key for `StateStatus::Blocked` inside registry `gate_effects`.
const STATUS_KEY_BLOCKED: &str = "blocked";
/// Status key for `StateStatus::NotApplicable` inside registry `gate_effects`.
const STATUS_KEY_NOT_APPLICABLE: &str = "not_applicable";
/// Status key for `StateStatus::Unknown` inside registry `gate_effects`.
const STATUS_KEY_UNKNOWN: &str = "unknown";

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

// ------------------------------------------------------------------ Layer 1

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

// ------------------------------------------------------------------ Layer 2

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
    check_registry(snapshot, registry).map_err(|mismatch| match mismatch {
        RegistryMismatch::Ref { expected, found } => {
            SummaryError::RegistryRefMismatch { expected, found }
        }
        RegistryMismatch::Product { expected, found } => {
            SummaryError::RegistryProductMismatch { expected, found }
        }
    })?;

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
