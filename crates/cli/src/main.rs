// Title: Minibench
// Subtitle: cli — print the comparison matrix for a directory of snapshots
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

//! `minibench compare <dir>` — load every `*.json` workbench snapshot in
//! a directory, lay them out as a run matrix (state variables × variants),
//! and print it, marking the rows where the variants diverge.
//! `minibench summarize <file>` — print one snapshot's roll-up.
//! `minibench diff` — regression gate vs the frozen baseline, optionally
//! judging human-corrected cells against their recorded expectation.
//! `minibench correct` — record one human correction as a JSON file.
//! `minibench validate <snapshot.json|dir> <registry.json>` — check every
//! evaluation's evidence refs against the registry's declared
//! evidence requirements. This is the terminal view of the permutation
//! harness; the Swift shell will render the same `ValidationReport` later.

mod strings;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use minibench_kernel::{
    CompareError, CompareWarning, ComparisonCell, ComparisonMatrix, CorrectedDiffReport,
    CorrectionKey, CorrectionSet, CorrectionV1, EvidenceViolationKind, RegressionReport,
    SpreadSignal, ValidationReport, compare, compare_with_registry, diff, diff_with_corrections,
    summarize, validate,
};
use workbench_contract::{StateStatus, StateVarRegistry, WorkbenchSnapshotV1};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("compare") => match args.get(1) {
            Some(dir) => run_compare(Path::new(dir), args.get(2).map(Path::new)),
            None => usage(),
        },
        Some("summarize") => match args.get(1) {
            Some(file) => run_summarize(Path::new(file)),
            None => usage(),
        },
        Some("diff") => match (args.get(1), args.get(2), corrections_dir(&args[1..])) {
            (Some(baseline), Some(current), Ok(corrections)) => run_diff(
                Path::new(baseline),
                Path::new(current),
                corrections.as_deref(),
            ),
            _ => usage(),
        },
        Some("correct") => run_correct(&args[1..]),
        Some("validate") => match (args.get(1), args.get(2)) {
            (Some(target), Some(registry)) => run_validate(Path::new(target), Path::new(registry)),
            _ => usage(),
        },
        _ => usage(),
    }
}

/// Extract an optional `--corrections <dir>` from the arguments after
/// `diff <baseline> <current>`; anything else trailing is a usage error.
fn corrections_dir(args: &[String]) -> Result<Option<PathBuf>, ()> {
    match args.get(2).map(String::as_str) {
        None => Ok(None),
        Some(flag) if flag == strings::ARG_CORRECTIONS => match (args.get(3), args.get(4)) {
            (Some(dir), None) => Ok(Some(PathBuf::from(dir))),
            _ => Err(()),
        },
        Some(_) => Err(()),
    }
}

fn usage() -> ExitCode {
    eprintln!("{}", strings::USAGE);
    ExitCode::FAILURE
}

fn run_summarize(file: &Path) -> ExitCode {
    let snapshot = match read_snapshot(file) {
        Ok(snapshot) => snapshot,
        Err(message) => {
            eprintln!("{} {message}", strings::ERROR_PREFIX);
            return ExitCode::FAILURE;
        }
    };
    let summary = summarize(&snapshot);
    let variant = snapshot
        .variant
        .as_ref()
        .map(|v| format!(" [{}]", v.label))
        .unwrap_or_default();
    println!("{} {}{}", summary.product, summary.snapshot_id, variant);
    println!(
        "  {} {} {} {}, {} {}, {} {}, {} {}",
        summary.total,
        strings::EVAL_COUNT_SUFFIX,
        summary.pass,
        strings::TALLY_PASS,
        summary.warn,
        strings::TALLY_WARN,
        summary.fail,
        strings::TALLY_FAIL,
        summary.blocked,
        strings::TALLY_BLOCKED,
    );
    if !summary.blocking.is_empty() {
        println!(
            "  {} {}",
            strings::BLOCKING_LABEL,
            summary.blocking.join(", ")
        );
    }
    println!();
    for ev in &snapshot.evaluations {
        println!(
            "  {:<34} {:<5} {:.2}",
            ev.state_var_id,
            status_str(ev.status),
            ev.score
        );
    }
    ExitCode::SUCCESS
}

fn run_compare(dir: &Path, registry_path: Option<&Path>) -> ExitCode {
    let snapshots = match load_nonempty(dir) {
        Ok(snapshots) => snapshots,
        Err(code) => return code,
    };
    let registry = match registry_path.map(read_registry).transpose() {
        Ok(registry) => registry,
        Err(message) => {
            eprintln!("{} {message}", strings::ERROR_PREFIX);
            return ExitCode::FAILURE;
        }
    };
    let matrix = match registry.as_ref() {
        Some(registry) => compare_with_registry(&snapshots, registry),
        None => compare(&snapshots),
    };
    let matrix = match matrix {
        Ok(matrix) => matrix,
        Err(err) => {
            eprintln!("{} {}", strings::ERROR_PREFIX, compare_error_str(&err));
            return ExitCode::FAILURE;
        }
    };
    print_matrix(&matrix);
    ExitCode::SUCCESS
}

/// `minibench diff <baseline-dir> <current-dir> [--corrections <dir>]` —
/// report every state variable that regressed vs the baseline (status
/// worse or score dropped) and exit non-zero when any did, so CI can fail
/// the build. With `--corrections`, human-corrected cells are judged
/// against their recorded expectation instead of the raw baseline. This
/// closes the loop: the harness can say a run is *worse*, not merely
/// *different*. Both directories must hold snapshots — an empty one is an
/// error, not a clean run, so a wrong path or missing artifacts can't
/// make the gate vacuous. The same holds for an explicit corrections dir.
fn run_diff(baseline_dir: &Path, current_dir: &Path, corrections_dir: Option<&Path>) -> ExitCode {
    let baseline = match load_nonempty(baseline_dir) {
        Ok(snapshots) => snapshots,
        Err(code) => return code,
    };
    let current = match load_nonempty(current_dir) {
        Ok(snapshots) => snapshots,
        Err(code) => return code,
    };

    let clean = match corrections_dir {
        None => {
            let report = diff(&baseline, &current);
            print_regressions(&report);
            report.is_clean()
        }
        Some(dir) => {
            let corrections = match load_corrections(dir) {
                Ok(corrections) => corrections,
                Err(message) => {
                    eprintln!("{} {message}", strings::ERROR_PREFIX);
                    return ExitCode::FAILURE;
                }
            };
            let report = diff_with_corrections(&baseline, &current, &corrections);
            print_corrected_report(&report);
            report.is_clean()
        }
    };
    if clean {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(strings::REGRESSION_EXIT_CODE)
    }
}

/// `minibench validate <snapshot.json|dir> <registry.json>` — check every
/// evaluation's evidence refs against the registry's declared
/// `evidence_requirements` and exit non-zero when any fall short. The
/// registry argument is required: the requirements are the yardstick. A
/// directory target validates every decodable `*.json` snapshot in it;
/// files that do not decode as snapshots are skipped with a warning so a
/// registry sitting beside its snapshots does not abort the gate.
fn run_validate(target: &Path, registry_path: &Path) -> ExitCode {
    let registry = match read_registry(registry_path) {
        Ok(registry) => registry,
        Err(message) => {
            eprintln!("{} {message}", strings::ERROR_PREFIX);
            return ExitCode::FAILURE;
        }
    };
    let snapshots = if target.is_dir() {
        match load_decodable(target) {
            Ok(snapshots) => snapshots,
            Err(code) => return code,
        }
    } else {
        match read_snapshot(target) {
            Ok(snapshot) => vec![snapshot],
            Err(message) => {
                eprintln!("{} {message}", strings::ERROR_PREFIX);
                return ExitCode::FAILURE;
            }
        }
    };

    let mut clean = true;
    for snapshot in &snapshots {
        let report = match validate(snapshot, &registry) {
            Ok(report) => report,
            Err(err) => {
                eprintln!("{} {err}", strings::ERROR_PREFIX);
                return ExitCode::FAILURE;
            }
        };
        print_validation(&report);
        clean &= report.is_clean();
    }
    if clean {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(strings::VIOLATION_EXIT_CODE)
    }
}

/// Load every snapshot in `dir`, treating an empty directory as an error —
/// a missing or wrong path must not read as a clean/empty result.
fn load_nonempty(dir: &Path) -> Result<Vec<WorkbenchSnapshotV1>, ExitCode> {
    match load_dir(dir) {
        Ok(snapshots) if snapshots.is_empty() => {
            eprintln!("{} {}", strings::NO_SNAPSHOTS_FOUND, dir.display());
            Err(ExitCode::FAILURE)
        }
        Ok(snapshots) => Ok(snapshots),
        Err(message) => {
            eprintln!("{} {message}", strings::ERROR_PREFIX);
            Err(ExitCode::FAILURE)
        }
    }
}

fn print_regressions(report: &RegressionReport) {
    if report.is_clean() {
        println!("{}", strings::NO_REGRESSIONS);
        return;
    }
    println!(
        "{} {}",
        strings::REGRESSIONS_HEADER,
        report.regressions.len()
    );
    for regression in &report.regressions {
        println!(
            "  {} [{}] {:<34} {} {:.2} -> {} {:.2}",
            regression.experiment_id,
            regression.variant,
            regression.state_var_id,
            status_str(regression.baseline_status),
            regression.baseline_score,
            status_str(regression.current_status),
            regression.current_score,
        );
    }
}

fn print_corrected_report(report: &CorrectedDiffReport) {
    for stale in &report.stale {
        println!(
            "{} {} {stale}",
            strings::WARNING_PREFIX,
            strings::STALE_CORRECTION
        );
    }
    for applied in &report.applied {
        println!("{} {applied}", strings::CORRECTION_APPLIED);
    }
    if report.is_clean() {
        println!("{}", strings::NO_REGRESSIONS);
        return;
    }
    println!(
        "{} {}",
        strings::REGRESSIONS_HEADER,
        report.regressions.len()
    );
    for regression in &report.regressions {
        let expected_score = regression
            .expected_score
            .map(|score| format!("{score:.2}"))
            .unwrap_or_else(|| strings::ABSENT_CELL.to_string());
        let corrected_mark = if regression.corrected {
            format!(" {}", strings::CORRECTED_MARK)
        } else {
            String::new()
        };
        println!(
            "  {} [{}] {:<34} {} {} -> {} {:.2}{}",
            regression.experiment_id,
            regression.variant,
            regression.state_var_id,
            status_str(regression.expected_status),
            expected_score,
            status_str(regression.current_status),
            regression.current_score,
            corrected_mark,
        );
    }
}

/// `minibench correct <corrections-dir> --experiment … --variant …
/// --state-var … --status … --rationale … --by … [--score …]
/// [--snapshot-id …]` — record one human correction as a JSON file whose
/// name derives from the cell key, so re-correcting the same cell
/// overwrites the previous record instead of accumulating duplicates.
fn run_correct(args: &[String]) -> ExitCode {
    let correction = match parse_correct_args(args) {
        Ok(parsed) => parsed,
        Err(None) => return usage(),
        Err(Some(message)) => {
            eprintln!("{} {message}", strings::ERROR_PREFIX);
            return ExitCode::FAILURE;
        }
    };
    let (dir, record) = correction;
    if let Err(err) = record.validate() {
        eprintln!("{} {err}", strings::ERROR_PREFIX);
        return ExitCode::FAILURE;
    }
    match write_correction(&dir, &record) {
        Ok(path) => {
            println!("{} {}", strings::CORRECTION_RECORDED, path.display());
            println!(
                "  {} {} {}",
                record.key(),
                status_str(record.expected_status),
                record
                    .expected_score
                    .map(|score| format!("{score:.2}"))
                    .unwrap_or_else(|| strings::ABSENT_CELL.to_string()),
            );
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("{} {message}", strings::ERROR_PREFIX);
            ExitCode::FAILURE
        }
    }
}

/// Parse `correct` arguments. `Err(None)` is a usage error (print usage);
/// `Err(Some(message))` is a diagnosable input error.
fn parse_correct_args(args: &[String]) -> Result<(PathBuf, CorrectionV1), Option<String>> {
    let Some(dir) = args.first() else {
        return Err(None);
    };
    let mut experiment_id = None;
    let mut variant_label = None;
    let mut state_var_id = None;
    let mut status = None;
    let mut score = None;
    let mut rationale = None;
    let mut corrected_by = None;
    let mut snapshot_id = None;

    let mut rest = args[1..].iter();
    while let Some(flag) = rest.next() {
        let value = rest
            .next()
            .ok_or_else(|| Some(format!("{} {flag}", strings::MISSING_ARGUMENT_VALUE)))?
            .clone();
        match flag.as_str() {
            strings::ARG_EXPERIMENT => experiment_id = Some(value),
            strings::ARG_VARIANT => variant_label = Some(value),
            strings::ARG_STATE_VAR => state_var_id = Some(value),
            strings::ARG_STATUS => status = Some(parse_status(&value)?),
            strings::ARG_SCORE => {
                score = Some(
                    value
                        .parse::<f64>()
                        .map_err(|_| Some(format!("{} {value}", strings::INVALID_SCORE)))?,
                );
            }
            strings::ARG_RATIONALE => rationale = Some(value),
            strings::ARG_BY => corrected_by = Some(value),
            strings::ARG_SNAPSHOT_ID => snapshot_id = Some(value),
            _ => return Err(Some(format!("{} {flag}", strings::UNKNOWN_ARGUMENT))),
        }
    }

    let record = CorrectionV1 {
        experiment_id: required(experiment_id, strings::ARG_EXPERIMENT)?,
        variant_label: required(variant_label, strings::ARG_VARIANT)?,
        state_var_id: required(state_var_id, strings::ARG_STATE_VAR)?,
        expected_status: required(status, strings::ARG_STATUS)?,
        expected_score: score,
        rationale: required(rationale, strings::ARG_RATIONALE)?,
        corrected_by: required(corrected_by, strings::ARG_BY)?,
        corrected_at: now_rfc3339()?,
        snapshot_id,
    };
    Ok((PathBuf::from(dir), record))
}

fn required<T>(value: Option<T>, flag: &str) -> Result<T, Option<String>> {
    value.ok_or_else(|| Some(format!("{} {flag}", strings::MISSING_REQUIRED_ARGUMENT)))
}

/// Parse a status through the contract enum's wire form, so only the
/// closed status vocabulary is accepted — never a raw string.
fn parse_status(raw: &str) -> Result<StateStatus, Option<String>> {
    serde_json::from_value(serde_json::Value::String(raw.to_string()))
        .map_err(|_| Some(format!("{} {raw}", strings::INVALID_STATUS)))
}

fn now_rfc3339() -> Result<String, Option<String>> {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|err| Some(format!("{} {err}", strings::TIMESTAMP_FORMAT_FAILED)))
}

/// Deterministic file name for a correction: the sanitized cell key, so
/// the same key always maps to the same file (overwrite, not duplicate).
fn correction_file_name(key: &CorrectionKey) -> String {
    format!(
        "{}__{}__{}.json",
        sanitize_component(&key.experiment_id),
        sanitize_component(&key.variant_label),
        sanitize_component(&key.state_var_id),
    )
}

/// Keep filename-safe characters; everything else becomes `-`.
fn sanitize_component(component: &str) -> String {
    component
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn write_correction(dir: &Path, record: &CorrectionV1) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(correction_file_name(&record.key()));
    let json =
        serde_json::to_string_pretty(record).map_err(|e| format!("{}: {e}", path.display()))?;
    std::fs::write(&path, format!("{json}\n")).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

/// Load every correction in `dir` into a validated set. An empty
/// directory is an error — an explicitly passed corrections dir that
/// contributes nothing means a wrong path, not a clean overlay.
fn load_corrections(dir: &Path) -> Result<CorrectionSet, String> {
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("json"))
        .collect();
    paths.sort();
    if paths.is_empty() {
        return Err(format!(
            "{} {}",
            strings::NO_CORRECTIONS_FOUND,
            dir.display()
        ));
    }

    let mut corrections = Vec::with_capacity(paths.len());
    for path in paths {
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let record: CorrectionV1 =
            serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        corrections.push(record);
    }
    CorrectionSet::new(corrections).map_err(|e| e.to_string())
}

fn read_snapshot(file: &Path) -> Result<WorkbenchSnapshotV1, String> {
    let bytes = std::fs::read(file).map_err(|e| format!("{}: {e}", file.display()))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", file.display()))
}

fn read_registry(file: &Path) -> Result<StateVarRegistry, String> {
    let bytes = std::fs::read(file).map_err(|e| format!("{}: {e}", file.display()))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", file.display()))
}

fn json_paths(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("json"))
        .collect();
    paths.sort();
    Ok(paths)
}

fn load_dir(dir: &Path) -> Result<Vec<WorkbenchSnapshotV1>, String> {
    let paths = json_paths(dir)?;
    let mut snapshots = Vec::with_capacity(paths.len());
    for path in paths {
        snapshots.push(read_snapshot(&path)?);
    }
    Ok(snapshots)
}

/// Load every `*.json` in `dir` that decodes as a snapshot, warning on
/// the ones that do not; zero decodable snapshots is an error, not a
/// clean gate.
fn load_decodable(dir: &Path) -> Result<Vec<WorkbenchSnapshotV1>, ExitCode> {
    let paths = match json_paths(dir) {
        Ok(paths) => paths,
        Err(message) => {
            eprintln!("{} {message}", strings::ERROR_PREFIX);
            return Err(ExitCode::FAILURE);
        }
    };
    let mut snapshots = Vec::with_capacity(paths.len());
    for path in paths {
        match read_snapshot(&path) {
            Ok(snapshot) => snapshots.push(snapshot),
            Err(message) => eprintln!(
                "{} {} {message}",
                strings::WARNING_PREFIX,
                strings::SKIPPING_UNDECODABLE
            ),
        }
    }
    if snapshots.is_empty() {
        eprintln!(
            "{} {} {}",
            strings::ERROR_PREFIX,
            strings::NO_DECODABLE_SNAPSHOTS,
            dir.display()
        );
        return Err(ExitCode::FAILURE);
    }
    Ok(snapshots)
}

fn print_validation(report: &ValidationReport) {
    println!("{} {}", report.product, report.snapshot_id);
    if report.is_clean() {
        println!("  {}", strings::NO_EVIDENCE_VIOLATIONS);
        return;
    }
    println!(
        "  {} {}",
        strings::VIOLATIONS_HEADER,
        report.violations.len()
    );
    for violation in &report.violations {
        println!(
            "    {:<34} {:<22} {}",
            violation.state_var_id,
            violation_kind_str(violation.kind),
            violation.message
        );
    }
}

fn violation_kind_str(kind: EvidenceViolationKind) -> &'static str {
    match kind {
        EvidenceViolationKind::MissingEvidence => strings::VIOLATION_MISSING_EVIDENCE,
        EvidenceViolationKind::PassWithoutEvidence => strings::VIOLATION_PASS_WITHOUT_EVIDENCE,
        EvidenceViolationKind::BelowMinCount => strings::VIOLATION_BELOW_MIN_COUNT,
        EvidenceViolationKind::MissingRequiredRef => strings::VIOLATION_MISSING_REQUIRED_REF,
        EvidenceViolationKind::MissingHash => strings::VIOLATION_MISSING_HASH,
        EvidenceViolationKind::MissingSourceRole => strings::VIOLATION_MISSING_SOURCE_ROLE,
        EvidenceViolationKind::StaleEvidence => strings::VIOLATION_STALE_EVIDENCE,
        EvidenceViolationKind::MissingCreatedAt => strings::VIOLATION_MISSING_CREATED_AT,
        EvidenceViolationKind::MalformedTimestamp => strings::VIOLATION_MALFORMED_TIMESTAMP,
        EvidenceViolationKind::UnknownStateVar => strings::VIOLATION_UNKNOWN_STATE_VAR,
    }
}

fn status_str(status: StateStatus) -> &'static str {
    match status {
        StateStatus::Pass => strings::STATUS_PASS,
        StateStatus::Warn => strings::STATUS_WARN,
        StateStatus::Fail => strings::STATUS_FAIL,
        StateStatus::Blocked => strings::STATUS_BLOCKED,
        StateStatus::NotApplicable => strings::STATUS_NOT_APPLICABLE,
        StateStatus::Unknown => strings::STATUS_UNKNOWN,
    }
}

fn cell_str(cell: &Option<ComparisonCell>) -> String {
    match cell {
        Some(c) if c.replicate_count > 1 => format!(
            "{} {:.2} {}{:.2} [{:.2}-{:.2}] {}/{}",
            status_str(c.status),
            c.score,
            strings::CONFIDENCE_PREFIX,
            c.confidence,
            c.score_min,
            c.score_max,
            c.present_count,
            c.replicate_count
        ),
        Some(c) => format!(
            "{} {:.2} {}{:.2}",
            status_str(c.status),
            c.score,
            strings::CONFIDENCE_PREFIX,
            c.confidence
        ),
        None => strings::ABSENT_CELL.to_string(),
    }
}

fn compare_error_str(err: &CompareError) -> String {
    err.to_string()
}

fn compare_warning_str(warning: &CompareWarning) -> &'static str {
    match warning {
        CompareWarning::MissingSourceHashes => strings::WARNING_MISSING_SOURCE_HASHES,
        CompareWarning::MissingPolicyProvenance => strings::WARNING_MISSING_POLICY_PROVENANCE,
        CompareWarning::MissingEvaluatorProvenance => strings::WARNING_MISSING_EVALUATOR_PROVENANCE,
    }
}

fn spread_signal_str(signal: SpreadSignal) -> &'static str {
    match signal {
        SpreadSignal::NoSpread => strings::SPREAD_SIGNAL_NONE,
        SpreadSignal::SingleRun => strings::SPREAD_SIGNAL_SINGLE_RUN,
        SpreadSignal::BetweenExceedsWithin => strings::SPREAD_SIGNAL_BETWEEN,
        SpreadSignal::WithinMatchesBetween => strings::SPREAD_SIGNAL_WITHIN,
    }
}

fn confidence_range_str(min: f64, max: f64, spread: f64) -> String {
    if spread == 0.0 {
        format!("{min:.2}")
    } else {
        format!("{min:.2}-{max:.2}")
    }
}

fn print_matrix(matrix: &ComparisonMatrix) {
    // Build the grid as strings, then pad each column to its widest cell.
    let mut header: Vec<String> = vec![strings::COL_STATE_VARIABLE.to_string()];
    header.extend(matrix.variants.iter().enumerate().map(|(idx, variant)| {
        let replicates = matrix.variant_replicates[idx];
        if replicates > 1 {
            format!("{variant} (n={replicates})")
        } else {
            variant.clone()
        }
    }));
    header.push(strings::COL_SPREAD.to_string());
    header.push(strings::COL_WITHIN.to_string());
    header.push(strings::COL_CONFIDENCE.to_string());
    header.push(strings::COL_SPREAD_SIGNAL.to_string());
    header.push(strings::COL_MEANINGFUL.to_string());
    header.push(strings::COL_DIVERGE.to_string());
    header.push(strings::COL_COVERAGE.to_string());
    header.push(strings::COL_UNSTABLE.to_string());

    let mut grid: Vec<Vec<String>> = Vec::with_capacity(matrix.rows.len() + 1);
    grid.push(header.clone());
    for row in &matrix.rows {
        let mut line = vec![row.state_var_id.clone()];
        line.extend(row.cells.iter().map(cell_str));
        line.push(format!("{:.2}", row.score_spread));
        line.push(format!("{:.2}", row.within_score_spread));
        line.push(confidence_range_str(
            row.confidence_min,
            row.confidence_max,
            row.confidence_spread,
        ));
        line.push(spread_signal_str(row.spread_signal).to_string());
        line.push(
            if row.meaningful_score_spread {
                strings::DIVERGENCE_MARK
            } else {
                ""
            }
            .to_string(),
        );
        line.push(
            if row.status_divergence {
                strings::DIVERGENCE_MARK
            } else {
                ""
            }
            .to_string(),
        );
        line.push(
            if row.coverage_divergence {
                strings::DIVERGENCE_MARK
            } else {
                ""
            }
            .to_string(),
        );
        line.push(
            if row.status_unstable {
                strings::DIVERGENCE_MARK
            } else {
                ""
            }
            .to_string(),
        );
        grid.push(line);
    }

    let cols = header.len();
    let widths: Vec<usize> = (0..cols)
        .map(|c| grid.iter().map(|r| r[c].chars().count()).max().unwrap_or(0))
        .collect();

    println!("{} {}", strings::EXPERIMENT_PREFIX, matrix.experiment_id);
    for warning in &matrix.warnings {
        println!(
            "{} {}",
            strings::WARNING_PREFIX,
            compare_warning_str(warning)
        );
    }
    println!();
    for (i, line) in grid.iter().enumerate() {
        let rendered: Vec<String> = line
            .iter()
            .enumerate()
            .map(|(c, cell)| {
                let pad = widths[c] - cell.chars().count();
                format!("{cell}{}", " ".repeat(pad))
            })
            .collect();
        println!("{}", rendered.join("  ").trim_end());
        if i == 0 {
            let rule: Vec<String> = widths.iter().map(|w| "─".repeat(*w)).collect();
            println!("{}", rule.join("  "));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_correction() -> CorrectionV1 {
        CorrectionV1 {
            experiment_id: "career.lens.acme-l4-eval".to_string(),
            variant_label: "haiku+mechanical".to_string(),
            state_var_id: "career.lens.report_grounded".to_string(),
            expected_status: StateStatus::Fail,
            expected_score: Some(0.40),
            rationale: "accepted grounding floor after review".to_string(),
            corrected_by: "reviewer@test".to_string(),
            corrected_at: "2026-07-19T00:00:00Z".to_string(),
            snapshot_id: Some("wb-haiku+mechanical".to_string()),
        }
    }

    /// Fresh per-test directory under the OS temp dir, removed on drop.
    struct TempCorrectionsDir(PathBuf);

    impl TempCorrectionsDir {
        fn new(label: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "minibench-corrections-{label}-{}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            Self(dir)
        }
    }

    impl Drop for TempCorrectionsDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn written_correction_round_trips_through_the_loader() {
        let dir = TempCorrectionsDir::new("round-trip");
        let record = sample_correction();

        let path = write_correction(&dir.0, &record).expect("write correction");
        let loaded = load_corrections(&dir.0).expect("load corrections");

        assert_eq!(
            loaded,
            CorrectionSet::new(vec![record.clone()]).expect("valid record"),
            "the loaded set equals the written record"
        );
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some(concat!(
                "career.lens.acme-l4-eval__haiku-mechanical__",
                "career.lens.report_grounded.json"
            )),
            "filename derives from the sanitized cell key"
        );
    }

    #[test]
    fn recorrecting_the_same_cell_overwrites_the_previous_file() {
        let dir = TempCorrectionsDir::new("overwrite");
        let first = sample_correction();
        let mut second = sample_correction();
        second.expected_score = Some(0.45);
        second.rationale = "raised the accepted floor after re-review".to_string();

        write_correction(&dir.0, &first).expect("write first");
        write_correction(&dir.0, &second).expect("write second");

        let files: Vec<_> = std::fs::read_dir(&dir.0)
            .expect("read corrections dir")
            .filter_map(Result::ok)
            .collect();
        assert_eq!(files.len(), 1, "same key maps to the same file");
        let loaded = load_corrections(&dir.0).expect("load corrections");
        assert_eq!(loaded.len(), 1);
    }

    #[test]
    fn loader_refuses_an_empty_corrections_dir() {
        let dir = TempCorrectionsDir::new("empty");
        std::fs::create_dir_all(&dir.0).expect("create empty dir");

        let err = load_corrections(&dir.0).expect_err("empty dir refused");

        assert!(err.starts_with(strings::NO_CORRECTIONS_FOUND));
    }

    #[test]
    fn parse_status_accepts_only_the_contract_vocabulary() {
        assert_eq!(parse_status("pass"), Ok(StateStatus::Pass));
        assert_eq!(
            parse_status("not_applicable"),
            Ok(StateStatus::NotApplicable)
        );
        assert!(parse_status("PASS").is_err());
        assert!(parse_status("approved").is_err());
    }
}
