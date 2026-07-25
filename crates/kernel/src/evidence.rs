// Title: Minibench
// Subtitle: kernel — evidence-requirements validation
// Author: Christopher Lester
// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

// Evidence-requirements validation — the trust gate
//
// The registry declares, per state variable, what an evaluation must be
// able to point at to be trusted (`EvidenceRequirements`). Nothing else
// in the pipeline enforces that declaration, so an evaluator can report
// `pass` with zero evidence and every downstream view renders it as
// settled fact. `validate` closes that hole: given a snapshot and the
// registry that scored it, it reports every evaluation whose evidence
// falls short of the registry's requirements. The registry is REQUIRED —
// the requirements are the yardstick, so there is no registry-free
// variant of this op.
//
// `required_refs` matching: an entry names an evidence TYPE in
// PascalCase ("LensVerdict"); the refs that satisfy it carry that type
// as a kebab-case `source_role` ("lens-verdict"). The golden fixtures
// pair exactly those spellings, and `EvidenceRef.id` is an instance id
// ("career.lens-verdict.technical-execution.l3") no registry could name
// ahead of time — so entries match against `source_role`, with both
// sides reduced to lowercase alphanumerics.
use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use workbench_contract::{
    EvidenceRequirements, StateEvaluation, StateStatus, StateVarRegistry, StateVariable,
    WorkbenchSnapshotV1,
};

use crate::checks::{
    check_freshness, check_hashes, check_min_count, check_required_refs, check_source_roles,
};
use crate::snapshot::{RegistryMismatch, check_registry};
use crate::violations::{EvidenceViolation, EvidenceViolationKind, violation};

// ------------------------------------------------------------------ Layer 0

/// The outcome of [`validate`] — every shortfall found, in evaluation
/// order.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationReport {
    pub product: String,
    pub snapshot_id: String,
    /// Evaluations checked.
    pub total: usize,
    pub violations: Vec<EvidenceViolation>,
}

impl ValidationReport {
    /// True when every evaluation satisfies its evidence requirements —
    /// the gate passes.
    pub fn is_clean(&self) -> bool {
        self.violations.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidateError {
    RegistryRefMismatch { expected: String, found: String },
    RegistryProductMismatch { expected: String, found: String },
}

impl fmt::Display for ValidateError {
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

impl std::error::Error for ValidateError {}

/// True when the state variable declares ANY evidence requirement. A
/// `must_include_*` of `Some(false)` explicitly waives that rule, so it
/// does not count as a declared requirement.
fn declares_requirements(requirements: &EvidenceRequirements) -> bool {
    !requirements.required_refs.is_empty()
        || requirements.min_count.is_some()
        || requirements.must_include_hash == Some(true)
        || requirements.must_include_source_role == Some(true)
        || requirements.freshness_sla_hours.is_some()
}

// ------------------------------------------------------------------ Layer 1

fn validate_evaluation(
    evaluation: &StateEvaluation,
    requirements: &EvidenceRequirements,
    violations: &mut Vec<EvidenceViolation>,
) {
    if evaluation.evidence_refs.is_empty() {
        // Zero refs implies every per-rule check, so report the two
        // invariant shortfalls and skip the redundant detail.
        if evaluation.status == StateStatus::Pass {
            violations.push(violation(
                evaluation,
                EvidenceViolationKind::PassWithoutEvidence,
                "status pass with no evidence refs".to_string(),
            ));
        }
        if declares_requirements(requirements) {
            violations.push(violation(
                evaluation,
                EvidenceViolationKind::MissingEvidence,
                "declares evidence requirements but carries no evidence refs".to_string(),
            ));
        }
        return;
    }
    violations.extend(check_min_count(evaluation, requirements));
    violations.extend(check_required_refs(evaluation, requirements));
    violations.extend(check_hashes(evaluation, requirements));
    violations.extend(check_source_roles(evaluation, requirements));
    violations.extend(check_freshness(evaluation, requirements));
}

// ------------------------------------------------------------------ Layer 2

/// Check every evaluation's evidence refs against the registry's
/// declared `evidence_requirements` and report each shortfall. All
/// thresholds (`min_count`, `freshness_sla_hours`, the required ref
/// set) come from the registry; the kernel invents none.
pub fn validate(
    snapshot: &WorkbenchSnapshotV1,
    registry: &StateVarRegistry,
) -> Result<ValidationReport, ValidateError> {
    check_registry(snapshot, registry).map_err(|mismatch| match mismatch {
        RegistryMismatch::Ref { expected, found } => {
            ValidateError::RegistryRefMismatch { expected, found }
        }
        RegistryMismatch::Product { expected, found } => {
            ValidateError::RegistryProductMismatch { expected, found }
        }
    })?;

    let state_vars: BTreeMap<&str, &StateVariable> = registry
        .state_vars
        .iter()
        .map(|state_var| (state_var.id.as_str(), state_var))
        .collect();

    let mut violations = Vec::new();
    for evaluation in &snapshot.evaluations {
        match state_vars.get(evaluation.state_var_id.as_str()) {
            Some(state_var) => validate_evaluation(
                evaluation,
                &state_var.evidence_requirements,
                &mut violations,
            ),
            None => violations.push(violation(
                evaluation,
                EvidenceViolationKind::UnknownStateVar,
                "evaluation names a state variable absent from the registry".to_string(),
            )),
        }
    }

    Ok(ValidationReport {
        product: snapshot.product.clone(),
        snapshot_id: snapshot.snapshot_id.clone(),
        total: snapshot.evaluations.len(),
        violations,
    })
}
