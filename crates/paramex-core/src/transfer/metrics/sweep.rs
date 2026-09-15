//! Pure sweep operations: double-sweep split + predicates.

use crate::shared::numerics::FLOAT_EPSILON;
use crate::shared::numpy_compat::{isclose, nanargmax, nanargmin};
use crate::transfer::types::SweepData;

/// Minimum samples per branch for a usable backward sweep.
pub(in crate::transfer::metrics) const MIN_SWEEP_POINTS: usize = 12;

/// Whether the split contains a usable backward sweep: both branches have at
/// least `MIN_SWEEP_POINTS` samples.
pub(in crate::transfer) fn has_backward_sweep(forward: &SweepData, backward: &SweepData) -> bool {
    forward.vg.len() >= MIN_SWEEP_POINTS && backward.vg.len() >= MIN_SWEEP_POINTS
}

/// Split a round-trip transfer sweep into forward and backward branches.
///
/// Non-finite `(vg, id_abs)` rows are dropped first. With fewer than 4 surviving
/// points (or no voltage change at all), the whole sweep is the forward branch
/// and the backward branch is empty. Otherwise the turn-around voltage is the
/// max (rising start) or min (falling start); the **first** matching index ends
/// the forward branch and the **last** matching index starts the backward branch
/// (so the apex sample appears in both). The duplicated-apex match uses
/// `isclose(rtol=0, atol=FLOAT_EPSILON)`.
pub fn split_double_sweep(vg: &[f64], id_abs: &[f64]) -> (SweepData, SweepData) {
    let mut x: Vec<f64> = Vec::new();
    let mut y: Vec<f64> = Vec::new();
    for (&v, &i) in vg.iter().zip(id_abs.iter()) {
        if v.is_finite() && i.is_finite() {
            x.push(v);
            y.push(i);
        }
    }
    let n = x.len();
    let empty = || {
        (
            SweepData {
                vg: x.clone(),
                id_abs: y.clone(),
            },
            SweepData {
                vg: Vec::new(),
                id_abs: Vec::new(),
            },
        )
    };
    if n < 4 {
        return empty();
    }

    // dx = diff(x); first index where |dx| > EPS drives the initial direction.
    let mut first_dx: Option<f64> = None;
    for k in 0..n - 1 {
        let d = x[k + 1] - x[k];
        if d.abs() > FLOAT_EPSILON {
            first_dx = Some(d);
            break;
        }
    }
    let Some(dx0) = first_dx else {
        return empty();
    };
    let initial_direction = if dx0 > 0.0 { 1.0 } else { -1.0 };

    let turn_value = if initial_direction > 0.0 {
        x.iter().copied().fold(f64::NEG_INFINITY, f64::max)
    } else {
        x.iter().copied().fold(f64::INFINITY, f64::min)
    };

    let turn_idx: Vec<usize> = (0..n)
        .filter(|&i| isclose(x[i], turn_value, 0.0, FLOAT_EPSILON))
        .collect();

    if turn_idx.is_empty() {
        // Fallback: first-tie argmax/argmin over the (finite) x.
        let Some(idx_turn) = (if initial_direction > 0.0 {
            nanargmax(&x)
        } else {
            nanargmin(&x)
        }) else {
            return empty();
        };
        let forward = SweepData {
            vg: x[..idx_turn + 1].to_vec(),
            id_abs: y[..idx_turn + 1].to_vec(),
        };
        let backward = SweepData {
            vg: x[idx_turn..].to_vec(),
            id_abs: y[idx_turn..].to_vec(),
        };
        return (forward, backward);
    }

    let forward_end = turn_idx[0];
    let Some(&backward_start) = turn_idx.last() else {
        return empty();
    };
    let forward = SweepData {
        vg: x[..forward_end + 1].to_vec(),
        id_abs: y[..forward_end + 1].to_vec(),
    };
    let backward = SweepData {
        vg: x[backward_start..].to_vec(),
        id_abs: y[backward_start..].to_vec(),
    };
    (forward, backward)
}
