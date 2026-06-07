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
//! This first slice ships the run summary. Regression diff and
//! narration-packet assembly land in later slices.

use serde::{Deserialize, Serialize};
use workbench_contract::{StateStatus, WorkbenchSnapshotV1};

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

/// Roll up a snapshot's evaluations by status and collect the
/// state variables whose gate effect blocks progression.
pub fn summarize(snapshot: &WorkbenchSnapshotV1) -> RunSummary {
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
        if ev.gate_effect.contains("block") {
            summary.blocking.push(ev.state_var_id.clone());
        }
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarizes_any_tenant_snapshot() {
        // A real miniforge snapshot (one pass, one blocking fail).
        let snap: WorkbenchSnapshotV1 =
            serde_json::from_str(include_str!("../../../fixtures/sample-snapshot.json"))
                .expect("decode fixture");
        let summary = summarize(&snap);
        assert_eq!(summary.total, snap.evaluations.len());
        assert_eq!(summary.pass + summary.warn + summary.fail, summary.total);
        assert!(summary.fail >= 1, "fixture has a failing evaluation");
        assert!(
            summary
                .blocking
                .iter()
                .any(|id| id.contains("bundle_complete")),
            "the failing evaluation blocks progression"
        );
    }
}
