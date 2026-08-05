// Title: Minibench
// Subtitle: kernel — evidence violation vocabulary
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

//! The violation record every evidence check produces, plus the small
//! parsing/matching vocabulary the checks share.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use workbench_contract::StateEvaluation;

// ------------------------------------------------------------------ Layer 0

/// Why an evaluation's evidence falls short of its registry
/// requirements. Fieldless so reports stay machine-greppable; the
/// specifics ride in the violation's human message.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceViolationKind {
    /// The state variable declares evidence requirements but the
    /// evaluation carries zero evidence refs. `not_applicable`
    /// evaluations are exempt: the requirements describe what a SCORED
    /// evaluation must cite, and a variable that does not apply has
    /// nothing to evidence — demanding refs there would push adapters
    /// toward fabricating them.
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

/// Case/format-insensitive key matching a registry `required_refs` entry
/// ("LensVerdict") against an evidence ref's `source_role`
/// ("lens-verdict"): both sides reduce to lowercase alphanumerics.
pub(crate) fn required_ref_key(value: &str) -> String {
    value
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

pub(crate) fn parse_rfc3339(value: &str) -> Result<OffsetDateTime, time::error::Parse> {
    OffsetDateTime::parse(value, &Rfc3339)
}

pub(crate) fn violation(
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
