// Title: Minibench
// Subtitle: user-facing strings for the CLI
// Author: Christopher Lester
// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

//! User-facing and advisory text for the minibench CLI, centralized per
//! `miniforge-standards languages/rust` (230) § String constants. Domain
//! logic references `crate::strings::*` rather than inlining literals.

pub const USAGE: &str = "usage: minibench <compare <dir> | summarize <file.json>>";
pub const ERROR_PREFIX: &str = "error:";
pub const NO_SNAPSHOTS_FOUND: &str = "no *.json snapshots found in";

// compare matrix
pub const EXPERIMENT_PREFIX: &str = "experiment:";
pub const COL_STATE_VARIABLE: &str = "state variable";
pub const COL_SPREAD: &str = "spread";
pub const COL_DIVERGE: &str = "diverge";
pub const DIVERGENCE_MARK: &str = "◆";
pub const ABSENT_CELL: &str = "—";

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
