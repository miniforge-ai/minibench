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
//! the public surface.

mod compare;
mod evidence;
mod regression;
mod snapshot;
mod summary;

pub use compare::{
    CompareError, CompareWarning, ComparisonCell, ComparisonMatrix, ComparisonRow, SpreadSignal,
    compare, compare_with_registry,
};
pub use evidence::{
    EvidenceViolation, EvidenceViolationKind, ValidateError, ValidationReport, validate,
};
pub use regression::{Regression, RegressionReport, diff};
pub use summary::{RunSummary, SummaryError, summarize, summarize_with_registry};

#[cfg(test)]
mod tests;
