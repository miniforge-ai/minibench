// Title: Minibench
// Subtitle: user-facing strings for the CLI
// Author: Christopher Lester
// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

//! User-facing and advisory text for the minibench CLI, centralized per
//! `miniforge-standards languages/rust` (230) § String constants. Domain
//! logic references `crate::strings::*` rather than inlining literals.

pub const USAGE: &str =
    "usage: minibench <compare <dir> | summarize <file.json> | diff <baseline-dir> <current-dir>>";
pub const ERROR_PREFIX: &str = "error:";
pub const WARNING_PREFIX: &str = "warning:";
pub const NO_SNAPSHOTS_FOUND: &str = "no *.json snapshots found in";

// diff / regression gate
pub const NO_REGRESSIONS: &str = "no regressions vs baseline";
pub const REGRESSIONS_HEADER: &str = "regressions:";
/// Exit code when `diff` finds regressions — non-zero so CI fails the build.
pub const REGRESSION_EXIT_CODE: u8 = 3;

// compare matrix
pub const EXPERIMENT_PREFIX: &str = "experiment:";
pub const COL_STATE_VARIABLE: &str = "state variable";
pub const COL_SPREAD: &str = "spread";
pub const COL_WITHIN: &str = "within";
pub const COL_CONFIDENCE: &str = "confidence";
pub const COL_SPREAD_SIGNAL: &str = "spread signal";
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
