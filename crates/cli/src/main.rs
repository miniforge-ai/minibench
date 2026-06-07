// Title: Minibench
// Subtitle: cli — print the comparison matrix for a directory of snapshots
// Author: Christopher Lester
// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

//! `minibench compare <dir>` — load every `*.json` workbench snapshot in
//! a directory, lay them out as a run matrix (state variables × variants),
//! and print it, marking the rows where the variants diverge. This is the
//! terminal view of the permutation harness; the Swift shell renders the
//! same `ComparisonMatrix` later.

use std::path::Path;
use std::process::ExitCode;

use minibench_kernel::{compare, ComparisonCell, ComparisonMatrix};
use workbench_contract::{StateStatus, WorkbenchSnapshotV1};

const USAGE: &str = "usage: minibench compare <dir>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("compare") => match args.get(1) {
            Some(dir) => run_compare(Path::new(dir)),
            None => {
                eprintln!("{USAGE}");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

fn run_compare(dir: &Path) -> ExitCode {
    let snapshots = match load_dir(dir) {
        Ok(s) if s.is_empty() => {
            eprintln!("no *.json snapshots found in {}", dir.display());
            return ExitCode::FAILURE;
        }
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    print_matrix(&compare(&snapshots));
    ExitCode::SUCCESS
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
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let snapshot =
            serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        snapshots.push(snapshot);
    }
    Ok(snapshots)
}

fn status_str(status: StateStatus) -> &'static str {
    match status {
        StateStatus::Pass => "pass",
        StateStatus::Warn => "warn",
        StateStatus::Fail => "fail",
        StateStatus::Blocked => "blkd",
        StateStatus::NotApplicable => "n/a",
        StateStatus::Unknown => "?",
    }
}

fn cell_str(cell: &Option<ComparisonCell>) -> String {
    match cell {
        Some(c) => format!("{} {:.2}", status_str(c.status), c.score),
        None => "—".to_string(),
    }
}

fn print_matrix(matrix: &ComparisonMatrix) {
    // Build the grid as strings, then pad each column to its widest cell.
    let mut header: Vec<String> = vec!["state variable".to_string()];
    header.extend(matrix.variants.iter().cloned());
    header.push("spread".to_string());
    header.push("diverge".to_string());

    let mut grid: Vec<Vec<String>> = Vec::with_capacity(matrix.rows.len() + 1);
    grid.push(header.clone());
    for row in &matrix.rows {
        let mut line = vec![row.state_var_id.clone()];
        line.extend(row.cells.iter().map(cell_str));
        line.push(format!("{:.2}", row.score_spread));
        line.push(if row.status_divergence { "◆" } else { "" }.to_string());
        grid.push(line);
    }

    let cols = header.len();
    let widths: Vec<usize> = (0..cols)
        .map(|c| grid.iter().map(|r| r[c].chars().count()).max().unwrap_or(0))
        .collect();

    println!("experiment: {}", matrix.experiment_id);
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
