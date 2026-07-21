// Title: Minibench
// Subtitle: kernel — behavior tests over the public API
// Author: Christopher Lester
// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

use std::collections::BTreeMap;

use workbench_contract::{
    EvidenceRequirements, Lifecycle, STATE_VAR_REGISTRY_V1, StateStatus, StateVarKind,
    StateVarRegistry, StateVariable, ValueType, WorkbenchSnapshotV1,
};

use super::*;
use crate::compare::{PROVENANCE_KEY_EVALUATOR_VERSION, PROVENANCE_KEY_POLICY_HASH};
use crate::summary::{
    GATE_EFFECT_BLOCKS_TRANSITION, STATUS_KEY_FAIL, STATUS_KEY_PASS, STATUS_KEY_WARN,
};

#[test]
fn summarizes_any_tenant_snapshot() {
    // A real miniforge snapshot (one pass, one blocking fail).
    let snap: WorkbenchSnapshotV1 =
        serde_json::from_str(include_str!("../../../fixtures/sample-snapshot.json"))
            .expect("decode fixture");
    let summary = summarize(&snap);
    assert_eq!(summary.total, snap.evaluations.len());
    assert_eq!(
        summary.pass
            + summary.warn
            + summary.fail
            + summary.blocked
            + summary.not_applicable
            + summary.unknown,
        summary.total
    );
    assert!(summary.fail >= 1, "fixture has a failing evaluation");
    assert!(
        summary
            .blocking
            .iter()
            .any(|id| id.contains("bundle_complete")),
        "the failing evaluation blocks progression"
    );
}

fn missing_provenance_warnings() -> Vec<CompareWarning> {
    vec![
        CompareWarning::MissingSourceHashes,
        CompareWarning::MissingPolicyProvenance,
        CompareWarning::MissingEvaluatorProvenance,
    ]
}

fn stamp_variant_axes_provenance(snapshot: &mut WorkbenchSnapshotV1) {
    let variant = snapshot.variant.as_mut().expect("fixture has variant");
    variant.axes.insert(
        PROVENANCE_KEY_POLICY_HASH.to_string(),
        "sha256:policy".to_string(),
    );
    variant.axes.insert(
        PROVENANCE_KEY_EVALUATOR_VERSION.to_string(),
        "career-evaluator/2026.07.03".to_string(),
    );
}

#[test]
fn summarize_only_blocks_failing_or_blocked_exact_gate_effects() {
    let mut snap: WorkbenchSnapshotV1 =
        serde_json::from_str(include_str!("../../../fixtures/sample-snapshot.json"))
            .expect("decode fixture");
    snap.evaluations[0].gate_effect = GATE_EFFECT_BLOCKS_TRANSITION.to_string();
    snap.evaluations[1].gate_effect = "unblocked_review".to_string();

    let summary = summarize(&snap);

    assert!(
        summary.blocking.is_empty(),
        "pass status and non-canonical strings must not block"
    );
}

#[test]
fn summarize_with_registry_resolves_gate_effects_from_registry() {
    let mut snap: WorkbenchSnapshotV1 =
        serde_json::from_str(include_str!("../../../fixtures/sample-snapshot.json"))
            .expect("decode fixture");
    let registry = registry_for_snapshot(&snap);
    snap.evaluations[1].gate_effect = "none".to_string();

    let summary = summarize_with_registry(&snap, &registry).expect("matching registry");

    assert_eq!(summary.blocking, vec!["miniforge.evidence.bundle_complete"]);
}

#[test]
fn summarize_with_registry_rejects_wrong_registry() {
    let snap: WorkbenchSnapshotV1 =
        serde_json::from_str(include_str!("../../../fixtures/sample-snapshot.json"))
            .expect("decode fixture");
    let mut registry = registry_for_snapshot(&snap);
    registry.version = "2026.06.06.2".to_string();

    let err = summarize_with_registry(&snap, &registry).expect_err("wrong registry rejected");

    assert!(matches!(err, SummaryError::RegistryRefMismatch { .. }));
}

fn registry_for_snapshot(snapshot: &WorkbenchSnapshotV1) -> StateVarRegistry {
    StateVarRegistry {
        schema_version: STATE_VAR_REGISTRY_V1.to_string(),
        registry_id: snapshot.registry_ref.registry_id.clone(),
        version: snapshot.registry_ref.version.clone(),
        product: snapshot.product.clone(),
        state_vars: snapshot
            .evaluations
            .iter()
            .map(|evaluation| {
                let mut gate_effects = BTreeMap::new();
                gate_effects.insert(STATUS_KEY_FAIL.to_string(), evaluation.gate_effect.clone());
                StateVariable {
                    id: evaluation.state_var_id.clone(),
                    version: snapshot.registry_ref.version.clone(),
                    product: snapshot.product.clone(),
                    area: "test".to_string(),
                    kind: StateVarKind::Quality,
                    description: "test state variable".to_string(),
                    value_type: ValueType::Number,
                    thresholds: BTreeMap::new(),
                    evidence_requirements: no_requirements(),
                    score_components: Vec::new(),
                    gate_effects,
                    lifecycle: Lifecycle::Active,
                    owner: None,
                    notes: None,
                }
            })
            .collect(),
    }
}

fn registry_with_thresholds(snapshot: &WorkbenchSnapshotV1) -> StateVarRegistry {
    let mut registry = registry_for_snapshot(snapshot);
    for state_var in &mut registry.state_vars {
        state_var
            .thresholds
            .insert(STATUS_KEY_WARN.to_string(), 0.65);
        state_var
            .thresholds
            .insert(STATUS_KEY_PASS.to_string(), 0.85);
    }
    registry
}

#[test]
fn compares_permutations_and_flags_divergence() {
    // Same task (career.lens.acme-l4-eval), two variants.
    let opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("decode opus variant");
    let haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/haiku-mechanical.json"
    ))
    .expect("decode haiku variant");

    let matrix = compare(&[opus, haiku]).expect("valid comparison");

    assert_eq!(matrix.experiment_id, "career.lens.acme-l4-eval");
    assert_eq!(matrix.variants, vec!["opus+semantic", "haiku+mechanical"]);
    assert_eq!(matrix.variant_replicates, vec![1, 1]);
    assert_eq!(matrix.warnings, missing_provenance_warnings());

    // The grounding row diverges (pass vs fail), with a real spread;
    // that's the signal "semantic vs mechanical changed the outcome".
    let grounded = matrix
        .rows
        .iter()
        .find(|r| r.state_var_id == "career.lens.report_grounded")
        .expect("grounding row present");
    assert!(grounded.status_divergence, "pass vs fail across variants");
    assert!(
        (grounded.score_spread - 0.45).abs() < 1e-9,
        "0.88 vs 0.43 spread"
    );
    assert_eq!(grounded.cells.len(), 2);
    assert!(grounded.cells.iter().all(Option::is_some));
    assert_eq!(grounded.spread_signal, SpreadSignal::SingleRun);
    assert!(!grounded.coverage_divergence);
    assert!(!grounded.status_unstable);
}

#[test]
fn groups_replicates_and_reports_within_variant_spread() {
    let opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("decode opus variant");
    let haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/haiku-mechanical.json"
    ))
    .expect("decode haiku variant");
    let mut opus_replicate = opus.clone();
    opus_replicate.snapshot_id = "wb-career-opus-semantic-0002".to_string();
    opus_replicate.run_id = "run-opus-0002".to_string();
    opus_replicate.evaluations[0].score = 0.92;
    opus_replicate.evaluations[0].confidence = 0.9;

    let matrix = compare(&[opus, opus_replicate, haiku]).expect("valid comparison");

    assert_eq!(matrix.variants, vec!["opus+semantic", "haiku+mechanical"]);
    assert_eq!(matrix.variant_replicates, vec![2, 1]);

    let grounded = matrix
        .rows
        .iter()
        .find(|r| r.state_var_id == "career.lens.report_grounded")
        .expect("grounding row present");
    let opus_cell = grounded.cells[0]
        .as_ref()
        .expect("opus aggregate cell present");
    assert_eq!(opus_cell.status, StateStatus::Pass);
    assert_eq!(opus_cell.present_count, 2);
    assert_eq!(opus_cell.replicate_count, 2);
    assert!((opus_cell.score - 0.9).abs() < 1e-9);
    assert!((opus_cell.score_min - 0.88).abs() < 1e-9);
    assert!((opus_cell.score_max - 0.92).abs() < 1e-9);
    assert!((grounded.within_score_spread - 0.04).abs() < 1e-9);
    assert!((grounded.score_spread - 0.47).abs() < 1e-9);
    assert_eq!(grounded.spread_signal, SpreadSignal::BetweenExceedsWithin);
}

#[test]
fn flags_spread_confounded_by_within_variant_noise() {
    let opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("decode opus variant");
    let haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/haiku-mechanical.json"
    ))
    .expect("decode haiku variant");
    let mut opus_noisy_replicate = opus.clone();
    opus_noisy_replicate.snapshot_id = "wb-career-opus-semantic-0002".to_string();
    opus_noisy_replicate.run_id = "run-opus-0002".to_string();
    opus_noisy_replicate.evaluations[0].score = 0.10;

    let matrix = compare(&[opus, opus_noisy_replicate, haiku]).expect("valid comparison");

    let grounded = matrix
        .rows
        .iter()
        .find(|r| r.state_var_id == "career.lens.report_grounded")
        .expect("grounding row present");
    assert_eq!(grounded.spread_signal, SpreadSignal::WithinMatchesBetween);
    assert!(
        grounded.within_score_spread >= grounded.score_spread,
        "within-variant spread should dominate the between-variant mean spread"
    );
}

#[test]
fn row_spread_uses_present_replicates_not_variant_replicates() {
    let opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("decode opus variant");
    let haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/haiku-mechanical.json"
    ))
    .expect("decode haiku variant");
    let mut opus_missing_replicate = opus.clone();
    opus_missing_replicate.snapshot_id = "wb-career-opus-semantic-0002".to_string();
    opus_missing_replicate.run_id = "run-opus-0002".to_string();
    opus_missing_replicate.evaluations.clear();

    let matrix = compare(&[opus, opus_missing_replicate, haiku]).expect("valid comparison");

    let grounded = matrix
        .rows
        .iter()
        .find(|r| r.state_var_id == "career.lens.report_grounded")
        .expect("grounding row present");
    assert!(
        grounded.coverage_divergence,
        "one variant replicate is missing the row"
    );
    assert_eq!(grounded.spread_signal, SpreadSignal::SingleRun);
}

#[test]
fn missing_evaluations_are_coverage_divergence() {
    let opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("decode opus variant");
    let mut haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/haiku-mechanical.json"
    ))
    .expect("decode haiku variant");
    haiku
        .evaluations
        .retain(|ev| ev.state_var_id != "career.lens.report_grounded");

    let matrix = compare(&[opus, haiku]).expect("valid comparison");

    let traceability = matrix
        .rows
        .iter()
        .find(|r| r.state_var_id == "career.lens.report_grounded")
        .expect("grounding row present");
    assert!(
        traceability.coverage_divergence,
        "missing evaluation is a result"
    );
    assert!(
        !traceability.status_divergence,
        "coverage has its own signal"
    );
    assert!(traceability.cells[1].is_none());
}

#[test]
fn rows_report_confidence_range_across_variants() {
    let mut opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("decode opus variant");
    let mut haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/haiku-mechanical.json"
    ))
    .expect("decode haiku variant");
    opus.evaluations[0].confidence = 0.51;
    haiku.evaluations[0].confidence = 0.99;

    let matrix = compare(&[opus, haiku]).expect("valid comparison");

    let grounded = matrix
        .rows
        .iter()
        .find(|r| r.state_var_id == "career.lens.report_grounded")
        .expect("grounding row present");
    assert!((grounded.confidence_min - 0.51).abs() < 1e-9);
    assert!((grounded.confidence_max - 0.99).abs() < 1e-9);
    assert!((grounded.confidence_spread - 0.48).abs() < 1e-9);
}

#[test]
fn compare_without_registry_does_not_mark_meaningful_score_spread() {
    let mut opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("decode opus variant");
    let mut haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/haiku-mechanical.json"
    ))
    .expect("decode haiku variant");
    opus.evaluations[0].score = 0.95;
    haiku.evaluations[0].score = 0.75;
    haiku.evaluations[0].status = StateStatus::Pass;

    let matrix = compare(&[opus, haiku]).expect("valid comparison");

    let grounded = matrix
        .rows
        .iter()
        .find(|r| r.state_var_id == "career.lens.report_grounded")
        .expect("grounding row present");
    assert!(!grounded.status_divergence);
    assert!(!grounded.meaningful_score_spread);
}

#[test]
fn compare_with_registry_flags_same_status_meaningful_score_spread() {
    let mut opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("decode opus variant");
    let mut haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/haiku-mechanical.json"
    ))
    .expect("decode haiku variant");
    let registry = registry_with_thresholds(&opus);
    opus.evaluations[0].score = 0.95;
    haiku.evaluations[0].score = 0.75;
    haiku.evaluations[0].status = StateStatus::Pass;

    let matrix =
        compare_with_registry(&[opus, haiku], &registry).expect("registry-aware comparison");

    let grounded = matrix
        .rows
        .iter()
        .find(|r| r.state_var_id == "career.lens.report_grounded")
        .expect("grounding row present");
    assert!(!grounded.status_divergence);
    assert!(grounded.meaningful_score_spread);
}

#[test]
fn compare_with_registry_rejects_wrong_registry() {
    let opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("decode opus variant");
    let haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/haiku-mechanical.json"
    ))
    .expect("decode haiku variant");
    let mut registry = registry_with_thresholds(&opus);
    registry.version = "2026.06.06.2".to_string();

    let err =
        compare_with_registry(&[opus, haiku], &registry).expect_err("wrong registry rejected");

    assert!(matches!(err, CompareError::RegistryRefMismatch { .. }));
}

#[test]
fn rejects_mixed_experiments() {
    let opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("decode opus variant");
    let mut haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/haiku-mechanical.json"
    ))
    .expect("decode haiku variant");
    haiku
        .variant
        .as_mut()
        .expect("fixture has variant")
        .experiment_id = "career.lens.other-eval".to_string();

    let err = compare(&[opus, haiku]).expect_err("mixed experiments rejected");

    assert!(matches!(err, CompareError::MixedExperimentIds { .. }));
}

#[test]
fn rejects_mixed_registry_refs() {
    let opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("decode opus variant");
    let mut haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/haiku-mechanical.json"
    ))
    .expect("decode haiku variant");
    haiku.registry_ref.version = "2026.06.06.2".to_string();

    let err = compare(&[opus, haiku]).expect_err("mixed registries rejected");

    assert!(matches!(err, CompareError::MixedRegistryRefs { .. }));
}

#[test]
fn rejects_duplicate_state_var_ids() {
    let mut opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("decode opus variant");
    opus.evaluations.push(opus.evaluations[0].clone());

    let err = compare(&[opus]).expect_err("duplicate evaluations rejected");

    assert!(matches!(err, CompareError::DuplicateStateVarId { .. }));
}

#[test]
fn rejects_duplicate_run_ids_inside_variant_replicates() {
    let opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("decode opus variant");
    let duplicate = opus.clone();

    let err = compare(&[opus, duplicate]).expect_err("duplicate run ids rejected");

    assert!(matches!(err, CompareError::DuplicateVariantRun { .. }));
}

#[test]
fn rejects_mismatched_source_hashes() {
    let mut opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("decode opus variant");
    let mut haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/haiku-mechanical.json"
    ))
    .expect("decode haiku variant");
    opus.source_hashes = Some(vec!["sha256:aaaaaaaa".to_string()]);
    haiku.source_hashes = Some(vec!["sha256:bbbbbbbb".to_string()]);

    let err = compare(&[opus, haiku]).expect_err("mismatched inputs rejected");

    assert!(matches!(err, CompareError::MixedSourceHashes { .. }));
}

#[test]
fn accepts_uniform_provenance_in_variant_axes() {
    let mut opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("decode opus variant");
    let mut haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/haiku-mechanical.json"
    ))
    .expect("decode haiku variant");
    stamp_variant_axes_provenance(&mut opus);
    stamp_variant_axes_provenance(&mut haiku);

    let matrix = compare(&[opus, haiku]).expect("uniform provenance is comparable");

    assert_eq!(matrix.warnings, vec![CompareWarning::MissingSourceHashes]);
}

#[test]
fn accepts_uniform_provenance_in_metadata() {
    let mut opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("decode opus variant");
    let mut haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/haiku-mechanical.json"
    ))
    .expect("decode haiku variant");
    let metadata = serde_json::json!({
        "provenance": {
            "policy_version": "career-policy/2026.07.03",
            "evaluator_hash": "sha256:evaluator"
        }
    });
    opus.metadata = Some(metadata.clone());
    haiku.metadata = Some(metadata);

    let matrix = compare(&[opus, haiku]).expect("uniform metadata provenance is comparable");

    assert_eq!(matrix.warnings, vec![CompareWarning::MissingSourceHashes]);
}

#[test]
fn rejects_mixed_policy_provenance() {
    let mut opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("decode opus variant");
    let mut haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/haiku-mechanical.json"
    ))
    .expect("decode haiku variant");
    stamp_variant_axes_provenance(&mut opus);
    stamp_variant_axes_provenance(&mut haiku);
    haiku
        .variant
        .as_mut()
        .expect("fixture has variant")
        .axes
        .insert(
            PROVENANCE_KEY_POLICY_HASH.to_string(),
            "sha256:other-policy".to_string(),
        );

    let err = compare(&[opus, haiku]).expect_err("mixed policy provenance rejected");

    assert!(matches!(err, CompareError::MixedProvenance { .. }));
}

#[test]
fn rejects_partially_missing_evaluator_provenance() {
    let opus: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("decode opus variant");
    let mut haiku: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/haiku-mechanical.json"
    ))
    .expect("decode haiku variant");
    haiku
        .variant
        .as_mut()
        .expect("fixture has variant")
        .axes
        .insert(
            PROVENANCE_KEY_EVALUATOR_VERSION.to_string(),
            "career-evaluator/2026.07.03".to_string(),
        );

    let err = compare(&[opus, haiku]).expect_err("partial evaluator provenance rejected");

    assert!(matches!(err, CompareError::MixedProvenance { .. }));
}

#[test]
fn diff_is_clean_against_self_and_flags_a_worsened_cell() {
    let baseline: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("decode baseline");

    // Identical current → nothing regressed.
    let same = std::slice::from_ref(&baseline);
    assert!(diff(same, same).is_clean());

    // Worsen the status and drop the score on a copy → exactly one
    // regression, on the same (experiment, variant, state-var) cell.
    let mut regressed = baseline.clone();
    regressed.evaluations[0].status = StateStatus::Fail;
    regressed.evaluations[0].score = 0.10;
    let report = diff(
        std::slice::from_ref(&baseline),
        std::slice::from_ref(&regressed),
    );

    assert!(!report.is_clean());
    assert_eq!(report.regressions.len(), 1);
    assert_eq!(report.regressions[0].baseline_status, StateStatus::Pass);
    assert_eq!(report.regressions[0].current_status, StateStatus::Fail);
}

fn no_requirements() -> EvidenceRequirements {
    EvidenceRequirements {
        required_refs: Vec::new(),
        min_count: None,
        must_include_hash: None,
        must_include_source_role: None,
        freshness_sla_hours: None,
    }
}

/// The opus variant fixture: one `pass` evaluation with two
/// `lens-verdict` evidence refs (no hashes, no `created_at`).
fn opus_snapshot() -> WorkbenchSnapshotV1 {
    serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("decode opus variant")
}

fn registry_with_requirements(
    snapshot: &WorkbenchSnapshotV1,
    requirements: EvidenceRequirements,
) -> StateVarRegistry {
    let mut registry = registry_for_snapshot(snapshot);
    for state_var in &mut registry.state_vars {
        state_var.evidence_requirements = requirements.clone();
    }
    registry
}

fn violation_kinds(report: &ValidationReport) -> Vec<EvidenceViolationKind> {
    report.violations.iter().map(|v| v.kind).collect()
}

#[test]
fn validate_is_vacuously_clean_without_declared_requirements() {
    let snap = opus_snapshot();
    let registry = registry_for_snapshot(&snap);

    let report = validate(&snap, &registry).expect("matching registry");

    assert!(report.is_clean());
    assert_eq!(report.total, snap.evaluations.len());
}

#[test]
fn validate_treats_waived_must_include_rules_as_no_requirements() {
    let mut snap = opus_snapshot();
    let registry = registry_with_requirements(
        &snap,
        EvidenceRequirements {
            must_include_hash: Some(false),
            must_include_source_role: Some(false),
            ..no_requirements()
        },
    );
    snap.evaluations[0].status = StateStatus::Fail;
    snap.evaluations[0].evidence_refs.clear();

    let report = validate(&snap, &registry).expect("matching registry");

    assert!(report.is_clean(), "Some(false) waives, not requires");
}

#[test]
fn validate_flags_below_min_count() {
    let snap = opus_snapshot();
    let registry = registry_with_requirements(
        &snap,
        EvidenceRequirements {
            min_count: Some(3),
            ..no_requirements()
        },
    );

    let report = validate(&snap, &registry).expect("matching registry");

    assert_eq!(
        violation_kinds(&report),
        vec![EvidenceViolationKind::BelowMinCount]
    );
}

#[test]
fn validate_accepts_min_count_met() {
    let snap = opus_snapshot();
    let registry = registry_with_requirements(
        &snap,
        EvidenceRequirements {
            min_count: Some(2),
            ..no_requirements()
        },
    );

    let report = validate(&snap, &registry).expect("matching registry");

    assert!(report.is_clean());
}

#[test]
fn validate_matches_required_refs_against_source_role() {
    let snap = opus_snapshot();
    // "LensVerdict" must match the fixture's kebab-case source role
    // "lens-verdict"; "Claim" has no matching ref.
    let registry = registry_with_requirements(
        &snap,
        EvidenceRequirements {
            required_refs: vec!["LensVerdict".to_string(), "Claim".to_string()],
            ..no_requirements()
        },
    );

    let report = validate(&snap, &registry).expect("matching registry");

    assert_eq!(
        violation_kinds(&report),
        vec![EvidenceViolationKind::MissingRequiredRef]
    );
    assert!(
        report.violations[0].message.contains("Claim"),
        "the unmatched entry is named"
    );
}

#[test]
fn validate_flags_refs_missing_a_required_hash() {
    let snap = opus_snapshot();
    let registry = registry_with_requirements(
        &snap,
        EvidenceRequirements {
            must_include_hash: Some(true),
            ..no_requirements()
        },
    );

    let report = validate(&snap, &registry).expect("matching registry");

    assert_eq!(
        violation_kinds(&report),
        vec![
            EvidenceViolationKind::MissingHash,
            EvidenceViolationKind::MissingHash
        ],
        "one violation per hashless ref"
    );

    let mut hashed = opus_snapshot();
    for evidence_ref in &mut hashed.evaluations[0].evidence_refs {
        evidence_ref.hash = Some("sha256:cafe".to_string());
    }
    let report = validate(&hashed, &registry).expect("matching registry");
    assert!(report.is_clean());
}

#[test]
fn validate_flags_refs_missing_a_required_source_role() {
    let mut snap = opus_snapshot();
    let registry = registry_with_requirements(
        &snap,
        EvidenceRequirements {
            must_include_source_role: Some(true),
            ..no_requirements()
        },
    );
    let report = validate(&snap, &registry).expect("matching registry");
    assert!(report.is_clean(), "fixture refs carry source roles");

    snap.evaluations[0].evidence_refs[0].source_role = String::new();
    let report = validate(&snap, &registry).expect("matching registry");
    assert_eq!(
        violation_kinds(&report),
        vec![EvidenceViolationKind::MissingSourceRole]
    );
}

#[test]
fn validate_flags_missing_created_at_under_a_freshness_sla() {
    let snap = opus_snapshot();
    let registry = registry_with_requirements(
        &snap,
        EvidenceRequirements {
            freshness_sla_hours: Some(24),
            ..no_requirements()
        },
    );

    let report = validate(&snap, &registry).expect("matching registry");

    assert_eq!(
        violation_kinds(&report),
        vec![
            EvidenceViolationKind::MissingCreatedAt,
            EvidenceViolationKind::MissingCreatedAt
        ],
        "an SLA without a timestamp is unverifiable, so strict"
    );
}

#[test]
fn validate_applies_the_freshness_sla_at_evaluated_at() {
    let mut snap = opus_snapshot();
    let registry = registry_with_requirements(
        &snap,
        EvidenceRequirements {
            freshness_sla_hours: Some(24),
            ..no_requirements()
        },
    );
    // The fixture evaluates at 2026-06-10T16:07:46Z; same-day refs
    // are inside a 24h SLA.
    for evidence_ref in &mut snap.evaluations[0].evidence_refs {
        evidence_ref.created_at = Some("2026-06-10T00:00:00Z".to_string());
    }
    let report = validate(&snap, &registry).expect("matching registry");
    assert!(report.is_clean());

    snap.evaluations[0].evidence_refs[0].created_at = Some("2026-06-01T00:00:00Z".to_string());
    let report = validate(&snap, &registry).expect("matching registry");
    assert_eq!(
        violation_kinds(&report),
        vec![EvidenceViolationKind::StaleEvidence]
    );
}

#[test]
fn validate_flags_malformed_timestamps_under_a_freshness_sla() {
    let mut snap = opus_snapshot();
    let registry = registry_with_requirements(
        &snap,
        EvidenceRequirements {
            freshness_sla_hours: Some(24),
            ..no_requirements()
        },
    );
    snap.evaluations[0].evaluated_at = "yesterday-ish".to_string();

    let report = validate(&snap, &registry).expect("matching registry");

    assert_eq!(
        violation_kinds(&report),
        vec![EvidenceViolationKind::MalformedTimestamp]
    );
}

#[test]
fn validate_flags_declared_requirements_with_zero_refs() {
    let mut snap = opus_snapshot();
    let registry = registry_with_requirements(
        &snap,
        EvidenceRequirements {
            required_refs: vec!["LensVerdict".to_string()],
            min_count: Some(1),
            must_include_hash: Some(true),
            ..no_requirements()
        },
    );
    snap.evaluations[0].status = StateStatus::Fail;
    snap.evaluations[0].evidence_refs.clear();

    let report = validate(&snap, &registry).expect("matching registry");

    assert_eq!(
        violation_kinds(&report),
        vec![EvidenceViolationKind::MissingEvidence],
        "zero refs collapses to one invariant violation, not per-rule noise"
    );
}

#[test]
fn validate_flags_pass_with_zero_refs_even_without_requirements() {
    let mut snap = opus_snapshot();
    let registry = registry_for_snapshot(&snap);
    snap.evaluations[0].evidence_refs.clear();

    let report = validate(&snap, &registry).expect("matching registry");

    assert_eq!(
        violation_kinds(&report),
        vec![EvidenceViolationKind::PassWithoutEvidence]
    );
}

#[test]
fn validate_flags_evaluations_missing_from_registry() {
    let snap = opus_snapshot();
    let registry = registry_for_snapshot(&snap);
    let mut renamed = snap;
    renamed.evaluations[0].state_var_id = "career.lens.unregistered".to_string();

    let report = validate(&renamed, &registry).expect("matching registry");

    assert_eq!(
        violation_kinds(&report),
        vec![EvidenceViolationKind::UnknownStateVar]
    );
}

#[test]
fn validate_rejects_wrong_registry() {
    let snap = opus_snapshot();
    let mut registry = registry_for_snapshot(&snap);
    registry.version = "2026.06.06.2".to_string();

    let err = validate(&snap, &registry).expect_err("wrong registry rejected");

    assert!(matches!(err, ValidateError::RegistryRefMismatch { .. }));
}

#[test]
fn validate_rejects_wrong_registry_product() {
    let snap = opus_snapshot();
    let mut registry = registry_for_snapshot(&snap);
    registry.product = "portfolio".to_string();

    let err = validate(&snap, &registry).expect_err("wrong product rejected");

    assert!(matches!(err, ValidateError::RegistryProductMismatch { .. }));
}
