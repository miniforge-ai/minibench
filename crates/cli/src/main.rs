// Title: Minibench
// Subtitle: cli — print the comparison matrix for a directory of snapshots
// Author: Christopher Lester
// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

//! `minibench compare <dir>` — load every `*.json` workbench snapshot in
//! a directory, lay them out as a run matrix (state variables × variants),
//! and print it, marking the rows where the variants diverge.
//! `minibench summarize <file>` — print one snapshot's roll-up. This is the
//! terminal view of the permutation harness; the Swift shell renders the
//! same `ComparisonMatrix` later.

mod strings;

use std::path::Path;
use std::process::ExitCode;

use minibench_kernel::{
    CompareError, CompareWarning, ComparisonCell, ComparisonMatrix, RegressionReport, SpreadSignal,
    compare, compare_with_registry, diff, summarize,
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
        Some("diff") => match (args.get(1), args.get(2)) {
            (Some(baseline), Some(current)) => run_diff(Path::new(baseline), Path::new(current)),
            _ => usage(),
        },
        _ => usage(),
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

/// `minibench diff <baseline-dir> <current-dir>` — report every state
/// variable that regressed vs the baseline (status worse or score dropped)
/// and exit non-zero when any did, so CI can fail the build. This closes the
/// loop: the harness can say a run is *worse*, not merely *different*. Both
/// directories must hold snapshots — an empty one is an error, not a clean
/// run, so a wrong path or missing artifacts can't make the gate vacuous.
fn run_diff(baseline_dir: &Path, current_dir: &Path) -> ExitCode {
    let baseline = match load_nonempty(baseline_dir) {
        Ok(snapshots) => snapshots,
        Err(code) => return code,
    };
    let current = match load_nonempty(current_dir) {
        Ok(snapshots) => snapshots,
        Err(code) => return code,
    };

    let report = diff(&baseline, &current);
    print_regressions(&report);
    if report.is_clean() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(strings::REGRESSION_EXIT_CODE)
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

fn read_snapshot(file: &Path) -> Result<WorkbenchSnapshotV1, String> {
    let bytes = std::fs::read(file).map_err(|e| format!("{}: {e}", file.display()))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", file.display()))
}

fn read_registry(file: &Path) -> Result<StateVarRegistry, String> {
    let bytes = std::fs::read(file).map_err(|e| format!("{}: {e}", file.display()))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", file.display()))
}

fn load_dir(dir: &Path) -> Result<Vec<WorkbenchSnapshotV1>, String> {
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("json"))
        .collect();
    paths.sort();

    let mut snapshots = Vec::with_capacity(paths.len());
    for path in paths {
        snapshots.push(read_snapshot(&path)?);
    }
    Ok(snapshots)
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
