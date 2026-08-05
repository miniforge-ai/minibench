// Title: Minibench
// Subtitle: kernel — evidence requirement checks
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

//! One check per registry-declared evidence rule; each returns the
//! violations it found and nothing else. All thresholds come from the
//! registry.

use std::collections::BTreeSet;

use time::OffsetDateTime;
use workbench_contract::{EvidenceRef, EvidenceRequirements, StateEvaluation};

use crate::violations::{
    EvidenceViolation, EvidenceViolationKind, parse_rfc3339, required_ref_key, violation,
};

// ------------------------------------------------------------------ Layer 0

/// Seconds per hour — unit conversion for `freshness_sla_hours`.
const SECONDS_PER_HOUR: i128 = 3_600;

pub(crate) fn check_min_count(
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

pub(crate) fn check_required_refs(
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

pub(crate) fn check_hashes(
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

pub(crate) fn check_source_roles(
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

// ------------------------------------------------------------------ Layer 1

pub(crate) fn check_freshness(
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
