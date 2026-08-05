// Title: Minibench
// Subtitle: kernel — tenant-agnostic roll-up of a workbench snapshot
// Author: Christopher Lester
// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

//! The generic kernel: tenant-agnostic operations over a
//! [`WorkbenchSnapshotV1`]. It reads only the contract — never a
//! product's domain crate — so the same code summarizes a portfolio,
//! career, or miniforge snapshot identically. That genericity is the
//! whole point: product-specific scoring happens in the product's
//! adapter; the kernel does the cross-cutting roll-up, regression
//! diff, and narration-packet assembly the shell renders.
//!
//! Ships the run summary, comparison matrix, regression diff, and
//! evidence-requirements validation; narration-packet assembly lands in
//! a later slice.
//!
//! Thin index per `standards/miniforge/languages/rust.mdc`: each bounded
//! operation lives in its own stratified module; this file only wires
//! the public surface (pass-through delegation only, like a Polylith
//! `interface`).

mod checks;
mod compare;
mod corrections;
mod evidence;
mod provenance;
mod regression;
mod snapshot;
mod stats;
mod summary;
mod violations;

use workbench_contract::{StateVarRegistry, WorkbenchSnapshotV1};

pub use compare::{CompareError, CompareWarning, ComparisonCell, ComparisonMatrix, ComparisonRow};
pub use corrections::{
    CorrectedDiffReport, CorrectedRegression, CorrectionError, CorrectionKey, CorrectionSet,
    CorrectionV1, diff_with_corrections,
};
pub use evidence::{ValidateError, ValidationReport, validate};
pub use regression::{Regression, RegressionReport, diff};
pub use stats::SpreadSignal;
pub use summary::{RunSummary, SummaryError, summarize, summarize_with_registry};
pub use violations::{EvidenceViolation, EvidenceViolationKind};

/// Lay out the snapshots of one experiment as a comparison matrix.
/// Variant columns keep first-seen order; snapshots with the same
/// experiment + label and distinct run ids are grouped as replicates.
/// State-variable rows keep first-seen order across snapshots so the
/// matrix is stable.
///
/// # Errors
///
/// Returns [`CompareError`] when the snapshots are not comparable.
pub fn compare(snapshots: &[WorkbenchSnapshotV1]) -> Result<ComparisonMatrix, CompareError> {
    compare::compare_inner(snapshots, None)
}

/// Registry-aware comparison that can flag same-status score spread as
/// meaningful when it crosses the state variable's threshold-band width.
///
/// # Errors
///
/// Returns [`CompareError`] when the snapshots are not comparable or the
/// registry does not match them.
pub fn compare_with_registry(
    snapshots: &[WorkbenchSnapshotV1],
    registry: &StateVarRegistry,
) -> Result<ComparisonMatrix, CompareError> {
    compare::compare_inner(snapshots, Some(registry))
}

#[cfg(test)]
mod tests;
