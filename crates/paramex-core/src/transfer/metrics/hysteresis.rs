//! Hysteresis extraction from the log-current curve-shift rule.

use crate::shared::numpy_compat::unique_mean;
use crate::shared::numpy_compat::{argsort, interp, linspace, nanmedian, take_by};
use crate::transfer::types::SweepData;

/// Minimum number of log-current grid points the branch shift is sampled on;
/// a sparser `min_points` never shrinks the grid below this.
const HYSTERESIS_GRID_MIN_POINTS: usize = 25;

/// Hysteresis as the median Vg shift between branches at equal log-current over
/// the trimmed overlap. Extraction uses `trim_fraction = 0.2` and
/// `min_points = 12`.
pub(in crate::transfer) fn extract_delta_vth_hysteresis_curve_shift(
    forward: &SweepData,
    backward: &SweepData,
    trim_fraction: f64,
    min_points: usize,
) -> f64 {
    let (f_logi0, f_vg0) = masked_log(forward);
    let (b_logi0, b_vg0) = masked_log(backward);
    if f_logi0.len() < min_points || b_logi0.len() < min_points {
        return f64::NAN;
    }

    // Sort by log-current, then collapse duplicate log-currents (avg Vg).
    let fo = argsort(&f_logi0);
    let f_logi_s = take_by(&f_logi0, &fo);
    let f_vg_s = take_by(&f_vg0, &fo);
    let bo = argsort(&b_logi0);
    let b_logi_s = take_by(&b_logi0, &bo);
    let b_vg_s = take_by(&b_vg0, &bo);

    let (f_logi, f_vg) = unique_mean(&f_logi_s, &f_vg_s);
    let (b_logi, b_vg) = unique_mean(&b_logi_s, &b_vg_s);
    if f_logi.len() < min_points || b_logi.len() < min_points {
        return f64::NAN;
    }

    let f_min = f_logi.iter().copied().fold(f64::INFINITY, f64::min);
    let f_max = f_logi.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let b_min = b_logi.iter().copied().fold(f64::INFINITY, f64::min);
    let b_max = b_logi.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let lo = f_min.max(b_min);
    let hi = f_max.min(b_max);
    if !lo.is_finite() || !hi.is_finite() || hi <= lo {
        return f64::NAN;
    }

    let span = hi - lo;
    let mut lo_trim = lo + trim_fraction * span;
    let mut hi_trim = hi - trim_fraction * span;
    if hi_trim <= lo_trim {
        lo_trim = lo;
        hi_trim = hi;
    }

    let grid = linspace(lo_trim, hi_trim, min_points.max(HYSTERESIS_GRID_MIN_POINTS));
    let f_interp = interp(&grid, &f_logi, &f_vg);
    let b_interp = interp(&grid, &b_logi, &b_vg);
    let delta: Vec<f64> = b_interp
        .iter()
        .zip(f_interp.iter())
        .map(|(b, f)| b - f)
        .collect();
    if delta.len() < min_points || !delta.iter().any(|v| v.is_finite()) {
        return f64::NAN;
    }
    nanmedian(&delta)
}

/// Mask finite positive current and return `(log10|Id|, Vg)` pairs.
fn masked_log(sweep: &SweepData) -> (Vec<f64>, Vec<f64>) {
    let mut logi: Vec<f64> = Vec::new();
    let mut vg: Vec<f64> = Vec::new();
    for (&v, &i) in sweep.vg.iter().zip(sweep.id_abs.iter()) {
        if v.is_finite() && i.is_finite() && i > 0.0 {
            logi.push(i.abs().log10());
            vg.push(v);
        }
    }
    (logi, vg)
}
