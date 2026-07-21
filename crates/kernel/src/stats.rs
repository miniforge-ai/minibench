// Title: Minibench
// Subtitle: kernel — replicate statistics
// Author: Christopher Lester
// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

//! Pure statistics over replicate scores and statuses — the numeric
//! vocabulary the comparison matrix is built from.

use std::cmp::Reverse;

use serde::{Deserialize, Serialize};
use workbench_contract::{StateStatus, StateVariable};

// ------------------------------------------------------------------ Layer 0

/// Standard deviation is only meaningful once at least two replicate
/// observations exist.
pub(crate) const MIN_REPLICATES_FOR_SPREAD: usize = 2;
/// Float tolerance for classifying score-spread signals.
pub(crate) const SCORE_SPREAD_EPSILON: f64 = 1e-9;
/// Stable ordering used only to make tied status votes deterministic.
pub(crate) const STATE_STATUS_ORDER: &[StateStatus] = &[
    StateStatus::Pass,
    StateStatus::Warn,
    StateStatus::Fail,
    StateStatus::Blocked,
    StateStatus::NotApplicable,
    StateStatus::Unknown,
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpreadSignal {
    NoSpread,
    SingleRun,
    BetweenExceedsWithin,
    WithinMatchesBetween,
}

pub(crate) fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

pub(crate) fn spread(values: &[f64]) -> f64 {
    if values.len() < MIN_REPLICATES_FOR_SPREAD {
        return 0.0;
    }
    let min = values.iter().copied().reduce(f64::min).unwrap_or(0.0);
    let max = values.iter().copied().reduce(f64::max).unwrap_or(0.0);
    max - min
}

pub(crate) fn spread_signal(
    score_spread: f64,
    within_score_spread: f64,
    row_present_counts: &[usize],
) -> SpreadSignal {
    if score_spread <= SCORE_SPREAD_EPSILON && within_score_spread <= SCORE_SPREAD_EPSILON {
        return SpreadSignal::NoSpread;
    }
    if row_present_counts
        .iter()
        .all(|present_count| *present_count < MIN_REPLICATES_FOR_SPREAD)
    {
        return SpreadSignal::SingleRun;
    }
    if within_score_spread + SCORE_SPREAD_EPSILON >= score_spread {
        SpreadSignal::WithinMatchesBetween
    } else {
        SpreadSignal::BetweenExceedsWithin
    }
}

pub(crate) fn meaningful_score_spread_threshold(state_var: &StateVariable) -> Option<f64> {
    let mut thresholds: Vec<f64> = state_var
        .thresholds
        .values()
        .copied()
        .filter(|threshold| threshold.is_finite())
        .collect();
    thresholds.sort_by(f64::total_cmp);
    thresholds
        .windows(2)
        .map(|window| window[1] - window[0])
        .filter(|gap| *gap > SCORE_SPREAD_EPSILON)
        .reduce(f64::min)
}

pub(crate) fn majority_status(statuses: &[StateStatus]) -> (StateStatus, bool) {
    let mut ranked: Vec<(StateStatus, usize)> = STATE_STATUS_ORDER
        .iter()
        .map(|status| {
            (
                *status,
                statuses
                    .iter()
                    .filter(|candidate| *candidate == status)
                    .count(),
            )
        })
        .filter(|(_, count)| *count > 0)
        .collect();
    ranked.sort_by_key(|(_, count)| Reverse(*count));

    let Some((status, count)) = ranked.first() else {
        return (StateStatus::Unknown, true);
    };
    let tied = ranked
        .get(1)
        .is_some_and(|(_, next_count)| next_count == count);
    if tied {
        (StateStatus::Unknown, true)
    } else {
        (*status, false)
    }
}

// ------------------------------------------------------------------ Layer 1

pub(crate) fn standard_deviation(values: &[f64]) -> f64 {
    if values.len() < MIN_REPLICATES_FOR_SPREAD {
        return 0.0;
    }
    let avg = mean(values);
    let variance = values
        .iter()
        .map(|value| {
            let delta = value - avg;
            delta * delta
        })
        .sum::<f64>()
        / values.len() as f64;
    variance.sqrt()
}
