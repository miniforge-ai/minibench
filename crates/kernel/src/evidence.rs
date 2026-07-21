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
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use workbench_contract::{
    EvidenceRef, EvidenceRequirements, StateEvaluation, StateStatus, StateVarRegistry,
    StateVariable, WorkbenchSnapshotV1,
};

use crate::snapshot::{RegistryMismatch, check_registry};

// ------------------------------------------------------------------ Layer 0

/// Seconds per hour — unit conversion for `freshness_sla_hours`.
const SECONDS_PER_HOUR: i128 = 3_600;

/// Why an evaluation's evidence falls short of its registry
/// requirements. Fieldless so reports stay machine-greppable; the
/// specifics ride in the violation's human message.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceViolationKind {
    /// The state variable declares evidence requirements but the
    /// evaluation carries zero evidence refs.
    MissingEvidence,
    /// The evaluation reports `pass` with zero evidence refs — the
    /// "no high confidence without evidence" invariant, enforced even
    /// when the state variable declares no requirements.
    PassWithoutEvidence,
    /// Fewer evidence refs than the registry's `min_count`.
    BelowMinCount,
    /// No evidence ref's source role matches a `required_refs` entry.
    MissingRequiredRef,
    /// `must_include_hash` is set and a ref carries no hash.
    MissingHash,
    /// `must_include_source_role` is set and a ref's source role is
    /// empty.
    MissingSourceRole,
    /// A ref's `created_at` is older than `freshness_sla_hours` at the
    /// evaluation's `evaluated_at`.
    StaleEvidence,
    /// A freshness SLA is set and a ref carries no `created_at`. The
    /// SLA cannot be verified without a timestamp, so this is strict —
    /// a violation, not a skip.
    MissingCreatedAt,
    /// A timestamp needed for the freshness check is not RFC 3339.
    MalformedTimestamp,
    /// The evaluation names a state variable the registry does not
    /// define, so its requirements cannot be resolved. A violation
    /// rather than a hard error, consistent with `summarize`/`compare`
    /// tolerating unregistered ids — but validation cannot vouch for
    /// what it cannot look up.
    UnknownStateVar,
}

/// One evaluation's shortfall against its evidence requirements.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceViolation {
    pub state_var_id: String,
    pub kind: EvidenceViolationKind,
    pub message: String,
}

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

/// Case/format-insensitive key matching a registry `required_refs` entry
/// ("LensVerdict") against an evidence ref's `source_role`
/// ("lens-verdict"): both sides reduce to lowercase alphanumerics.
fn required_ref_key(value: &str) -> String {
    value
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn parse_rfc3339(value: &str) -> Result<OffsetDateTime, time::error::Parse> {
    OffsetDateTime::parse(value, &Rfc3339)
}

fn violation(
    evaluation: &StateEvaluation,
    kind: EvidenceViolationKind,
    message: String,
) -> EvidenceViolation {
    EvidenceViolation {
        state_var_id: evaluation.state_var_id.clone(),
        kind,
        message,
    }
}

// ------------------------------------------------------------------ Layer 1

fn check_min_count(
    evaluation: &StateEvaluation,
    requirements: &EvidenceRequirements,
) -> Option<EvidenceViolation> {
    let required = requirements.min_count?;
    let found = evaluation.evidence_refs.len();
    if found >= required as usize {
        return None;
    }
    Some(violation(
        evaluation,
        EvidenceViolationKind::BelowMinCount,
        format!("{found} evidence ref(s), registry requires at least {required}"),
    ))
}

fn check_required_refs(
    evaluation: &StateEvaluation,
    requirements: &EvidenceRequirements,
) -> Vec<EvidenceViolation> {
    let present_roles: BTreeSet<String> = evaluation
        .evidence_refs
        .iter()
        .map(|evidence_ref| required_ref_key(&evidence_ref.source_role))
        .collect();
    requirements
        .required_refs
        .iter()
        .filter(|entry| !present_roles.contains(&required_ref_key(entry)))
        .map(|entry| {
            violation(
                evaluation,
                EvidenceViolationKind::MissingRequiredRef,
                format!("no evidence ref's source role matches required ref {entry}"),
            )
        })
        .collect()
}

fn check_hashes(
    evaluation: &StateEvaluation,
    requirements: &EvidenceRequirements,
) -> Vec<EvidenceViolation> {
    if requirements.must_include_hash != Some(true) {
        return Vec::new();
    }
    evaluation
        .evidence_refs
        .iter()
        .filter(|evidence_ref| evidence_ref.hash.as_deref().is_none_or(str::is_empty))
        .map(|evidence_ref| {
            violation(
                evaluation,
                EvidenceViolationKind::MissingHash,
                format!("evidence ref {} carries no hash", evidence_ref.id),
            )
        })
        .collect()
}

fn check_source_roles(
    evaluation: &StateEvaluation,
    requirements: &EvidenceRequirements,
) -> Vec<EvidenceViolation> {
    if requirements.must_include_source_role != Some(true) {
        return Vec::new();
    }
    evaluation
        .evidence_refs
        .iter()
        .filter(|evidence_ref| evidence_ref.source_role.is_empty())
        .map(|evidence_ref| {
            violation(
                evaluation,
                EvidenceViolationKind::MissingSourceRole,
                format!("evidence ref {} carries no source role", evidence_ref.id),
            )
        })
        .collect()
}

fn check_freshness(
    evaluation: &StateEvaluation,
    requirements: &EvidenceRequirements,
) -> Vec<EvidenceViolation> {
    let Some(sla_hours) = requirements.freshness_sla_hours else {
        return Vec::new();
    };
    let evaluated_at = match parse_rfc3339(&evaluation.evaluated_at) {
        Ok(evaluated_at) => evaluated_at,
        Err(_) => {
            return vec![violation(
                evaluation,
                EvidenceViolationKind::MalformedTimestamp,
                format!("evaluated_at {} is not RFC 3339", evaluation.evaluated_at),
            )];
        }
    };
    evaluation
        .evidence_refs
        .iter()
        .filter_map(|evidence_ref| {
            check_ref_freshness(evaluation, evidence_ref, evaluated_at, sla_hours)
        })
        .collect()
}

fn check_ref_freshness(
    evaluation: &StateEvaluation,
    evidence_ref: &EvidenceRef,
    evaluated_at: OffsetDateTime,
    sla_hours: u64,
) -> Option<EvidenceViolation> {
    let Some(created_at) = evidence_ref.created_at.as_deref() else {
        return Some(violation(
            evaluation,
            EvidenceViolationKind::MissingCreatedAt,
            format!(
                "evidence ref {} carries no created_at under a {sla_hours}h freshness SLA",
                evidence_ref.id
            ),
        ));
    };
    let Ok(created) = parse_rfc3339(created_at) else {
        return Some(violation(
            evaluation,
            EvidenceViolationKind::MalformedTimestamp,
            format!(
                "evidence ref {} created_at {created_at} is not RFC 3339",
                evidence_ref.id
            ),
        ));
    };
    let age_seconds = i128::from((evaluated_at - created).whole_seconds());
    let sla_seconds = i128::from(sla_hours) * SECONDS_PER_HOUR;
    (age_seconds > sla_seconds).then(|| {
        violation(
            evaluation,
            EvidenceViolationKind::StaleEvidence,
            format!(
                "evidence ref {} is {}h old, freshness SLA is {sla_hours}h",
                evidence_ref.id,
                age_seconds / SECONDS_PER_HOUR
            ),
        )
    })
}

// ------------------------------------------------------------------ Layer 2

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
