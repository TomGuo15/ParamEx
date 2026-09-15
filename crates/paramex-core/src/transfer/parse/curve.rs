//! Parsed transfer-curve construction and integrity checks.

use std::path::PathBuf;

use crate::transfer::types::ParsedCurve;

use super::MIN_TRANSFER_POINTS;

/// Build a [`ParsedCurve`] from already-coerced `vg`/`current` columns.
/// `current` is taken in absolute value;
/// samples are kept where `vg` and `|current|` are finite and `|current| > 0`;
/// [`validate_curve_integrity`] applies the minimum-point and shape checks,
/// returning `None` when the normalized curve is unusable.
///
/// Callers coerce raw cells before this seam (the labeled and fallback paths
/// both do), keeping coercion a single tested primitive.
pub(super) fn build_curve(
    name: &str,
    source_path: Option<PathBuf>,
    vg: &[f64],
    current: &[f64],
) -> Option<ParsedCurve> {
    let mut out_vg: Vec<f64> = Vec::new();
    let mut out_id: Vec<f64> = Vec::new();
    for (&x, &c) in vg.iter().zip(current.iter()) {
        let y = c.abs();
        if x.is_finite() && y.is_finite() && y > 0.0 {
            out_vg.push(x);
            out_id.push(y);
        }
    }
    let curve = ParsedCurve {
        name: name.to_string(),
        vg: out_vg,
        id_abs: out_id,
        source_path,
    };
    if validate_curve_integrity(&curve) {
        Some(curve)
    } else {
        None
    }
}

/// Whether a parsed curve is usable for transfer-metric extraction.
/// Equal sizes; >= [`MIN_TRANSFER_POINTS`]; all finite;
/// no non-positive `id_abs`; `ptp(vg) > 0`; >= `MIN_TRANSFER_POINTS / 2` unique
/// `vg`; >= 3 unique `id_abs`. Uniqueness is exact equality (no tolerance);
/// the curve is all-finite here, so a plain sort+dedup suffices.
pub(in crate::transfer) fn validate_curve_integrity(curve: &ParsedCurve) -> bool {
    if curve.vg.len() != curve.id_abs.len() {
        return false;
    }
    if curve.vg.len() < MIN_TRANSFER_POINTS {
        return false;
    }
    if !curve.vg.iter().all(|v| v.is_finite()) || !curve.id_abs.iter().all(|v| v.is_finite()) {
        return false;
    }
    if curve.id_abs.iter().any(|&v| v <= 0.0) {
        return false;
    }
    if crate::shared::numpy_compat::ptp(&curve.vg) <= 0.0 {
        return false;
    }
    if unique_count(&curve.vg) < MIN_TRANSFER_POINTS / 2 {
        return false;
    }
    unique_count(&curve.id_abs) >= 3
}

/// Count of distinct finite values by exact equality.
fn unique_count(vals: &[f64]) -> usize {
    let mut sorted: Vec<f64> = vals.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    sorted.dedup();
    sorted.len()
}
