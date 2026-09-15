//! TLM gate-voltage selection policy.

use crate::shared::numpy_compat::nanmedian;
use crate::tlm::types::{available_vg_values, TlmCurve};

/// The `V_G` with the strongest *median* channel current across curves. The
/// median, not the max, drives the default so one leaky device cannot pick the
/// operating point. Ties resolve to the smaller `|V_G|`.
pub(in crate::tlm) fn default_selected_vg(curves: &[TlmCurve]) -> f64 {
    let vg_values = available_vg_values(curves);
    if vg_values.is_empty() {
        return f64::NAN;
    }
    let mut best: Option<(f64, f64)> = None; // (median_current, vg)
    for &vg in &vg_values {
        let currents: Vec<f64> = curves
            .iter()
            .map(|c| c.current_at(vg).0)
            .filter(|c| c.is_finite() && *c > 0.0)
            .collect();
        if currents.is_empty() {
            continue;
        }
        let med = nanmedian(&currents);
        // max by (median, -|vg|): higher median wins; on tie, smaller |vg| wins.
        let better = match best {
            None => true,
            Some((bm, bvg)) => med > bm || (med == bm && -vg.abs() > -bvg.abs()),
        };
        if better {
            best = Some((med, vg));
        }
    }
    match best {
        Some((_, vg)) => vg,
        None => vg_values[0],
    }
}

/// Snap a requested `V_G` to the nearest measured value; `None` selects the
/// first measured value.
pub(in crate::tlm) fn selected_vg_for_dataset(vg_values: &[f64], selected_vg: Option<f64>) -> f64 {
    match selected_vg {
        None => vg_values.first().copied().unwrap_or(f64::NAN),
        Some(_) if vg_values.is_empty() => f64::NAN,
        Some(target) => *vg_values
            .iter()
            .min_by(|a, b| (**a - target).abs().total_cmp(&(**b - target).abs()))
            .unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tlm::types::{TlmSample, VdSource};

    fn curve(vg: &[f64], current: &[f64]) -> TlmCurve {
        let samples = vg
            .iter()
            .zip(current)
            .map(|(&vg, &current)| TlmSample::try_new(vg, current, current).unwrap())
            .collect();
        TlmCurve::try_new(
            "device.xlsx".to_owned(),
            "process".to_owned(),
            50.0,
            samples,
            -0.5,
            VdSource::Setup,
        )
        .unwrap()
    }

    #[test]
    fn available_gate_voltages_are_sorted_and_unique() {
        let first = curve(&[-40.0, -39.0], &[1.0, 1.0]);
        let second = curve(&[-39.0, -38.0], &[1.0, 1.0]);
        assert_eq!(
            available_vg_values(&[first, second]),
            vec![-40.0, -39.0, -38.0]
        );
    }

    #[test]
    fn selected_gate_voltage_snaps_to_the_nearest_measurement() {
        let vg = vec![-40.0, -38.0];
        assert_eq!(selected_vg_for_dataset(&vg, Some(-39.4)), -40.0);
        assert_eq!(selected_vg_for_dataset(&vg, Some(-38.6)), -38.0);
        assert_eq!(selected_vg_for_dataset(&vg, None), -40.0);
    }

    #[test]
    fn default_gate_voltage_uses_the_strongest_median_current() {
        let first = curve(&[-40.0, -38.0], &[5e-6, 1e-6]);
        let second = curve(&[-40.0, -38.0], &[7e-6, 2e-6]);
        assert_eq!(default_selected_vg(&[first, second]), -40.0);
    }
}
