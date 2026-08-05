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
use crate::provenance::{PROVENANCE_KEY_EVALUATOR_VERSION, PROVENANCE_KEY_POLICY_HASH};
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
    // One more than the fixture carries, so a fixture regen that adds
    // refs cannot silently satisfy the requirement under test.
    let demanded = snap.evaluations[0].evidence_refs.len() as u32 + 1;
    let registry = registry_with_requirements(
        &snap,
        EvidenceRequirements {
            min_count: Some(demanded),
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
    // "lens-verdict"; "DataQualityReport" has no matching ref.
    let registry = registry_with_requirements(
        &snap,
        EvidenceRequirements {
            required_refs: vec!["LensVerdict".to_string(), "DataQualityReport".to_string()],
            ..no_requirements()
        },
    );

    let report = validate(&snap, &registry).expect("matching registry");

    assert_eq!(
        violation_kinds(&report),
        vec![EvidenceViolationKind::MissingRequiredRef]
    );
    assert!(
        report.violations[0].message.contains("DataQualityReport"),
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
        vec![EvidenceViolationKind::MissingHash; snap.evaluations[0].evidence_refs.len()],
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
        vec![EvidenceViolationKind::MissingCreatedAt; snap.evaluations[0].evidence_refs.len()],
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
    // Refs stamped at the evaluation instant are trivially inside any
    // SLA. Read the instant off the fixture rather than pinning a date:
    // `bb regen-fixtures` restamps `evaluated_at` with the wall clock.
    let evaluated_at = snap.evaluations[0].evaluated_at.clone();
    for evidence_ref in &mut snap.evaluations[0].evidence_refs {
        evidence_ref.created_at = Some(evaluated_at.clone());
    }
    let report = validate(&snap, &registry).expect("matching registry");
    assert!(report.is_clean());

    snap.evaluations[0].evidence_refs[0].created_at = Some("2020-01-01T00:00:00Z".to_string());
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
fn validate_exempts_not_applicable_with_zero_refs() {
    let mut snap = opus_snapshot();
    let registry = registry_with_requirements(
        &snap,
        EvidenceRequirements {
            required_refs: vec!["DataQualityReport".to_string()],
            min_count: Some(0),
            must_include_source_role: Some(true),
            ..no_requirements()
        },
    );
    snap.evaluations[0].status = StateStatus::NotApplicable;
    snap.evaluations[0].evidence_refs.clear();

    let report = validate(&snap, &registry).expect("matching registry");

    assert!(
        report.is_clean(),
        "a variable that does not apply has nothing to evidence"
    );
}

#[test]
fn validate_checks_refs_carried_by_not_applicable_evaluations() {
    let mut snap = opus_snapshot();
    let registry = registry_with_requirements(
        &snap,
        EvidenceRequirements {
            must_include_hash: Some(true),
            ..no_requirements()
        },
    );
    snap.evaluations[0].status = StateStatus::NotApplicable;

    let report = validate(&snap, &registry).expect("matching registry");

    assert_eq!(
        violation_kinds(&report),
        vec![EvidenceViolationKind::MissingHash; snap.evaluations[0].evidence_refs.len()],
        "refs present on a not_applicable evaluation still get per-rule checks"
    );
}

#[test]
fn validate_treats_min_count_zero_as_waiver() {
    let mut snap = opus_snapshot();
    let registry = registry_with_requirements(
        &snap,
        EvidenceRequirements {
            min_count: Some(0),
            ..no_requirements()
        },
    );
    snap.evaluations[0].status = StateStatus::Fail;
    snap.evaluations[0].evidence_refs.clear();

    let report = validate(&snap, &registry).expect("matching registry");

    assert!(report.is_clean(), "Some(0) waives, not requires");
}

#[test]
fn validate_min_count_zero_does_not_waive_required_refs_for_scored_statuses() {
    let mut snap = opus_snapshot();
    let registry = registry_with_requirements(
        &snap,
        EvidenceRequirements {
            required_refs: vec!["DataQualityReport".to_string()],
            min_count: Some(0),
            ..no_requirements()
        },
    );
    snap.evaluations[0].status = StateStatus::Fail;
    snap.evaluations[0].evidence_refs.clear();

    let report = validate(&snap, &registry).expect("matching registry");

    assert_eq!(
        violation_kinds(&report),
        vec![EvidenceViolationKind::MissingEvidence],
        "a scored evaluation still owes its required ref types"
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

fn correction_for_grounded_cell(
    expected_status: StateStatus,
    expected_score: Option<f64>,
) -> CorrectionV1 {
    CorrectionV1 {
        experiment_id: "career.lens.acme-l4-eval".to_string(),
        variant_label: "opus+semantic".to_string(),
        state_var_id: "career.lens.report_grounded".to_string(),
        expected_status,
        expected_score,
        rationale: "reviewed the verdicts; this is the accepted read".to_string(),
        corrected_by: "reviewer@test".to_string(),
        corrected_at: "2026-07-19T00:00:00Z".to_string(),
        snapshot_id: Some("wb-opus+semantic".to_string()),
    }
}

#[test]
fn correction_flips_a_regression_into_accepted() {
    let baseline: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("decode baseline");
    let mut current = baseline.clone();
    current.evaluations[0].status = StateStatus::Fail;
    current.evaluations[0].score = 0.10;

    // Raw baseline flags the drop; the human-labeled expectation
    // accepts it, and the report records the provenance.
    let raw = diff(
        std::slice::from_ref(&baseline),
        std::slice::from_ref(&current),
    );
    assert_eq!(raw.regressions.len(), 1);

    let corrections = CorrectionSet::new(vec![correction_for_grounded_cell(
        StateStatus::Fail,
        Some(0.10),
    )])
    .expect("valid correction");
    let report = diff_with_corrections(
        std::slice::from_ref(&baseline),
        std::slice::from_ref(&current),
        &corrections,
    );

    assert!(report.is_clean());
    assert!(report.stale.is_empty());
    assert_eq!(
        report.applied,
        vec![correction_for_grounded_cell(StateStatus::Fail, None).key()]
    );
}

#[test]
fn correction_raises_the_bar_above_the_raw_baseline() {
    let baseline: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("decode baseline");
    let current = baseline.clone();

    // Identical current passes the raw baseline (0.88 pass)...
    assert!(
        diff(
            std::slice::from_ref(&baseline),
            std::slice::from_ref(&current)
        )
        .is_clean()
    );

    // ...but fails the corrected expectation of a 0.95 floor.
    let corrections = CorrectionSet::new(vec![correction_for_grounded_cell(
        StateStatus::Pass,
        Some(0.95),
    )])
    .expect("valid correction");
    let report = diff_with_corrections(
        std::slice::from_ref(&baseline),
        std::slice::from_ref(&current),
        &corrections,
    );

    assert!(!report.is_clean());
    assert_eq!(report.regressions.len(), 1);
    let regression = &report.regressions[0];
    assert!(regression.corrected);
    assert_eq!(regression.expected_status, StateStatus::Pass);
    assert_eq!(regression.expected_score, Some(0.95));
    assert_eq!(regression.current_status, StateStatus::Pass);
}

#[test]
fn status_only_correction_keeps_the_baseline_score_floor() {
    let baseline: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("decode baseline");
    let mut current = baseline.clone();
    current.evaluations[0].score = 0.50;

    // The correction overrides only the status; the 0.88 baseline
    // score remains the floor, so the drop to 0.50 still regresses.
    let corrections =
        CorrectionSet::new(vec![correction_for_grounded_cell(StateStatus::Pass, None)])
            .expect("valid correction");
    let report = diff_with_corrections(
        std::slice::from_ref(&baseline),
        std::slice::from_ref(&current),
        &corrections,
    );

    assert_eq!(report.regressions.len(), 1);
    assert_eq!(report.regressions[0].expected_score, Some(0.88));
}

#[test]
fn stale_correction_surfaces_as_a_warning_not_a_failure() {
    let baseline: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/opus-semantic.json"
    ))
    .expect("decode baseline");
    let mut stale = correction_for_grounded_cell(StateStatus::Pass, None);
    stale.state_var_id = "career.lens.renamed_away".to_string();

    let corrections = CorrectionSet::new(vec![stale.clone()]).expect("valid correction");
    let same = std::slice::from_ref(&baseline);
    let report = diff_with_corrections(same, same, &corrections);

    assert!(report.is_clean(), "stale corrections warn, never fail");
    assert!(report.applied.is_empty());
    assert_eq!(report.stale, vec![stale.key()]);
}

#[test]
fn duplicate_correction_keys_are_a_load_error() {
    let first = correction_for_grounded_cell(StateStatus::Pass, None);
    let second = correction_for_grounded_cell(StateStatus::Fail, Some(0.10));

    let err = CorrectionSet::new(vec![first, second]).expect_err("duplicate key rejected");

    assert!(matches!(err, CorrectionError::DuplicateKey { .. }));
}

#[test]
fn correction_without_a_rationale_is_refused() {
    let mut correction = correction_for_grounded_cell(StateStatus::Pass, None);
    correction.rationale = "   ".to_string();

    let err = CorrectionSet::new(vec![correction]).expect_err("blank rationale refused");

    assert!(matches!(err, CorrectionError::EmptyRationale { .. }));
}

#[test]
fn correction_against_committed_baseline_changes_the_diff_outcome() {
    // A correction accepting a 0.40 grounding floor for the mechanical
    // variant (committed baseline reads 0.43). A current run at 0.41
    // regresses against the raw baseline but passes the labeled
    // expectation — the correction demonstrably changes the outcome.
    // Constructed in-test so the assertion owns its own inputs; the
    // shipped correction for this same cell is exercised separately by
    // `committed_correction_fixture_targets_a_live_cell`.
    let correction: CorrectionV1 = serde_json::from_str(
        r#"{
          "experiment_id": "career.lens.acme-l4-eval",
          "variant_label": "haiku+mechanical",
          "state_var_id": "career.lens.report_grounded",
          "expected_status": "fail",
          "expected_score": 0.4,
          "rationale": "test-local example: accepted grounding floor for the mechanical method",
          "corrected_by": "test@example.com",
          "corrected_at": "2026-07-19T00:00:00Z",
          "snapshot_id": "wb-haiku+mechanical"
        }"#,
    )
    .expect("decode correction");
    let corrections = CorrectionSet::new(vec![correction]).expect("valid correction");
    let baseline: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/baseline/haiku-mechanical.json"
    ))
    .expect("decode baseline");
    let mut current = baseline.clone();
    current.evaluations[0].score = 0.41;

    let raw = diff(
        std::slice::from_ref(&baseline),
        std::slice::from_ref(&current),
    );
    assert_eq!(raw.regressions.len(), 1, "raw baseline flags 0.41 < 0.43");

    let report = diff_with_corrections(
        std::slice::from_ref(&baseline),
        std::slice::from_ref(&current),
        &corrections,
    );
    assert!(report.is_clean(), "corrected floor 0.40 accepts 0.41");
    assert_eq!(report.applied.len(), 1);
    assert!(report.stale.is_empty());
}

#[test]
fn committed_correction_fixture_targets_a_live_cell() {
    // The correction shipped under fixtures/corrections is what `bb
    // regression-gate` loads in CI. Its key is three strings that no
    // compiler checks: rename the state variable, relabel the variant, or
    // retire the experiment, and the correction silently becomes a
    // no-op that the gate reports only as a `stale` warning. Pin it to
    // the committed fixtures here so that drift fails the build instead.
    let correction: CorrectionV1 = serde_json::from_str(include_str!(concat!(
        "../../../fixtures/corrections/",
        "career.lens.acme-l4-eval__haiku-mechanical__career.lens.report_grounded.json"
    )))
    .expect("decode committed correction");
    let key = correction.key();
    let corrections = CorrectionSet::new(vec![correction]).expect("valid committed correction");
    let baseline: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/baseline/haiku-mechanical.json"
    ))
    .expect("decode baseline");
    let current: WorkbenchSnapshotV1 = serde_json::from_str(include_str!(
        "../../../fixtures/experiments/haiku-mechanical.json"
    ))
    .expect("decode current");

    let report = diff_with_corrections(
        std::slice::from_ref(&baseline),
        std::slice::from_ref(&current),
        &corrections,
    );

    assert!(
        report.stale.is_empty(),
        "committed correction matches no cell in the committed fixtures: {:?}",
        report.stale
    );
    assert_eq!(
        report.applied,
        vec![key.clone()],
        "the gate should judge exactly this cell against the committed correction"
    );
    // Scoped to the corrected cell, not `is_clean()`: an unrelated cell
    // regressing is the regression gate's business, not this test's.
    let target = report.regressions.iter().find(|regression| {
        regression.experiment_id == key.experiment_id
            && regression.variant == key.variant_label
            && regression.state_var_id == key.state_var_id
    });
    assert!(
        target.is_none(),
        "committed fixtures fall below the committed correction's floor: {target:?}"
    );
}
