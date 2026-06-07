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

use minibench_kernel::{ComparisonCell, ComparisonMatrix, compare, summarize};
use workbench_contract::{StateStatus, WorkbenchSnapshotV1};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match (args.first().map(String::as_str), args.get(1)) {
        (Some("compare"), Some(dir)) => run_compare(Path::new(dir)),
        (Some("summarize"), Some(file)) => run_summarize(Path::new(file)),
        _ => {
            eprintln!("{}", strings::USAGE);
            ExitCode::FAILURE
        }
    }
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

fn run_compare(dir: &Path) -> ExitCode {
    let snapshots = match load_dir(dir) {
        Ok(snapshots) if snapshots.is_empty() => {
            eprintln!("{} {}", strings::NO_SNAPSHOTS_FOUND, dir.display());
            return ExitCode::FAILURE;
        }
        Ok(snapshots) => snapshots,
        Err(message) => {
            eprintln!("{} {message}", strings::ERROR_PREFIX);
            return ExitCode::FAILURE;
        }
    };
    print_matrix(&compare(&snapshots));
    ExitCode::SUCCESS
}

fn read_snapshot(file: &Path) -> Result<WorkbenchSnapshotV1, String> {
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
        Some(c) => format!("{} {:.2}", status_str(c.status), c.score),
        None => strings::ABSENT_CELL.to_string(),
    }
}

fn print_matrix(matrix: &ComparisonMatrix) {
    // Build the grid as strings, then pad each column to its widest cell.
    let mut header: Vec<String> = vec![strings::COL_STATE_VARIABLE.to_string()];
    header.extend(matrix.variants.iter().cloned());
    header.push(strings::COL_SPREAD.to_string());
    header.push(strings::COL_DIVERGE.to_string());

    let mut grid: Vec<Vec<String>> = Vec::with_capacity(matrix.rows.len() + 1);
    grid.push(header.clone());
    for row in &matrix.rows {
        let mut line = vec![row.state_var_id.clone()];
        line.extend(row.cells.iter().map(cell_str));
        line.push(format!("{:.2}", row.score_spread));
        line.push(
            if row.status_divergence {
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
