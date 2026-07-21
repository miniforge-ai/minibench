// Title: Minibench
// Subtitle: user-facing strings for the CLI
// Author: Christopher Lester
// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

//! User-facing and advisory text for the minibench CLI, centralized per
//! `miniforge-standards languages/rust` (230) § String constants. Domain
//! logic references `crate::strings::*` rather than inlining literals.

pub const USAGE: &str = concat!(
    "usage: minibench <compare <dir> [registry.json] | ",
    "summarize <file.json> | ",
    "diff <baseline-dir> <current-dir> [--corrections <dir>] | ",
    "correct <corrections-dir> --experiment <id> --variant <label> ",
    "--state-var <id> --status <status> --rationale <why> --by <who> ",
    "[--score <n>] [--snapshot-id <id>] | ",
    "validate <snapshot.json|dir> <registry.json>>"
);
pub const ERROR_PREFIX: &str = "error:";
pub const WARNING_PREFIX: &str = "warning:";
pub const NO_SNAPSHOTS_FOUND: &str = "no *.json snapshots found in";

// diff / regression gate
pub const NO_REGRESSIONS: &str = "no regressions vs baseline";
pub const REGRESSIONS_HEADER: &str = "regressions:";
/// Exit code when `diff` finds regressions — non-zero so CI fails the build.
pub const REGRESSION_EXIT_CODE: u8 = 3;

// correction loop (correct subcommand + corrections-aware diff)
pub const ARG_CORRECTIONS: &str = "--corrections";
pub const ARG_EXPERIMENT: &str = "--experiment";
pub const ARG_VARIANT: &str = "--variant";
pub const ARG_STATE_VAR: &str = "--state-var";
pub const ARG_STATUS: &str = "--status";
pub const ARG_SCORE: &str = "--score";
pub const ARG_RATIONALE: &str = "--rationale";
pub const ARG_BY: &str = "--by";
pub const ARG_SNAPSHOT_ID: &str = "--snapshot-id";
pub const NO_CORRECTIONS_FOUND: &str = "no *.json corrections found in";
pub const CORRECTION_RECORDED: &str = "recorded correction:";
pub const CORRECTION_APPLIED: &str = "corrected expectation applied:";
pub const STALE_CORRECTION: &str = "stale correction matches no baseline or current cell:";
pub const CORRECTED_MARK: &str = "(corrected)";
pub const INVALID_STATUS: &str =
    "invalid status (expected pass|warn|fail|blocked|not_applicable|unknown):";
pub const INVALID_SCORE: &str = "invalid score:";
pub const UNKNOWN_ARGUMENT: &str = "unknown argument:";
pub const MISSING_ARGUMENT_VALUE: &str = "missing value for";
pub const MISSING_REQUIRED_ARGUMENT: &str = "missing required argument:";
pub const TIMESTAMP_FORMAT_FAILED: &str = "could not format the current time as RFC 3339:";

// validate / evidence gate
pub const NO_EVIDENCE_VIOLATIONS: &str = "no evidence violations";
pub const VIOLATIONS_HEADER: &str = "violations:";
/// Exit code when `validate` finds violations — non-zero so CI fails the
/// build, distinct from the `diff` regression code.
pub const VIOLATION_EXIT_CODE: u8 = 4;
pub const NO_DECODABLE_SNAPSHOTS: &str = "no decodable *.json snapshots found in";
pub const SKIPPING_UNDECODABLE: &str = "skipping undecodable *.json:";
/// Machine-greppable violation-kind labels, mirroring the kernel's
/// snake_case serde names.
pub const VIOLATION_MISSING_EVIDENCE: &str = "missing_evidence";
pub const VIOLATION_PASS_WITHOUT_EVIDENCE: &str = "pass_without_evidence";
pub const VIOLATION_BELOW_MIN_COUNT: &str = "below_min_count";
pub const VIOLATION_MISSING_REQUIRED_REF: &str = "missing_required_ref";
pub const VIOLATION_MISSING_HASH: &str = "missing_hash";
pub const VIOLATION_MISSING_SOURCE_ROLE: &str = "missing_source_role";
pub const VIOLATION_STALE_EVIDENCE: &str = "stale_evidence";
pub const VIOLATION_MISSING_CREATED_AT: &str = "missing_created_at";
pub const VIOLATION_MALFORMED_TIMESTAMP: &str = "malformed_timestamp";
pub const VIOLATION_UNKNOWN_STATE_VAR: &str = "unknown_state_var";

// compare matrix
pub const EXPERIMENT_PREFIX: &str = "experiment:";
pub const COL_STATE_VARIABLE: &str = "state variable";
pub const COL_SPREAD: &str = "spread";
pub const COL_WITHIN: &str = "within";
pub const COL_CONFIDENCE: &str = "confidence";
pub const COL_SPREAD_SIGNAL: &str = "spread signal";
pub const COL_MEANINGFUL: &str = "meaningful";
pub const COL_DIVERGE: &str = "diverge";
pub const COL_COVERAGE: &str = "coverage";
pub const COL_UNSTABLE: &str = "unstable";
pub const DIVERGENCE_MARK: &str = "◆";
pub const ABSENT_CELL: &str = "—";
pub const CONFIDENCE_PREFIX: &str = "c";
pub const WARNING_MISSING_SOURCE_HASHES: &str =
    "source_hashes missing; input equality could not be verified";
pub const WARNING_MISSING_POLICY_PROVENANCE: &str =
    "policy provenance missing; scoring yardstick could not be verified";
pub const WARNING_MISSING_EVALUATOR_PROVENANCE: &str =
    "evaluator provenance missing; evaluator implementation could not be verified";
pub const SPREAD_SIGNAL_NONE: &str = "none";
pub const SPREAD_SIGNAL_SINGLE_RUN: &str = "single-run";
pub const SPREAD_SIGNAL_BETWEEN: &str = "between";
pub const SPREAD_SIGNAL_WITHIN: &str = "within≈between";

// status display labels
pub const STATUS_PASS: &str = "pass";
pub const STATUS_WARN: &str = "warn";
pub const STATUS_FAIL: &str = "fail";
pub const STATUS_BLOCKED: &str = "blkd";
pub const STATUS_NOT_APPLICABLE: &str = "n/a";
pub const STATUS_UNKNOWN: &str = "?";

// summarize view
pub const EVAL_COUNT_SUFFIX: &str = "eval(s):";
pub const TALLY_PASS: &str = "pass";
pub const TALLY_WARN: &str = "warn";
pub const TALLY_FAIL: &str = "fail";
pub const TALLY_BLOCKED: &str = "blocked";
pub const BLOCKING_LABEL: &str = "blocking:";
