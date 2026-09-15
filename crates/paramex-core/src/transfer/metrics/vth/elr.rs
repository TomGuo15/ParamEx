//! ELR fit and mobility extraction for V_TH.

use crate::shared::numerics::FLOAT_EPSILON;
use crate::transfer::fit::{Transform, WindowedFitter};
use crate::transfer::types::ExtractionContext;

/// Acceptance gate for a V_TH fit: too few samples or a low R² reject the fit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::transfer) struct FitGate {
    /// Minimum samples inside the fit window.
    pub(in crate::transfer) min_points: usize,
    /// Minimum R² when the R² is finite; `0.0` accepts any finite fit.
    pub(in crate::transfer) min_r2: f64,
}

/// Linear fit of sqrt(Id) vs Vg used by the ELR V_TH method;
/// `vt = -intercept/slope`. Float fields are `NaN` when the fit is rejected.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::transfer) struct VtFitResult {
    pub(in crate::transfer) vt: f64,
    pub(in crate::transfer) mobility: f64,
    pub(in crate::transfer) slope: f64,
    pub(in crate::transfer) intercept: f64,
    pub(in crate::transfer) r2: f64,
    pub(in crate::transfer) points: usize,
}

/// Extract V_TH by ELR (extrapolation in the linear region): a linear fit of
/// sqrt(|Id|) vs Vg whose x-intercept is `vt`. Mobility is left NaN here;
/// [`extract_vt_mu`] fills it in.
///
/// The fit is rejected (NaN `vt`/`mobility`, fit diagnostics carried through)
/// when too few points, a non-finite or ~zero slope, or (when `r2` is finite)
/// `r2 < gate.min_r2`.
pub(in crate::transfer::metrics) fn extract_vth_elr(
    vg: &[f64],
    id_abs: &[f64],
    fit_range: Option<(f64, f64)>,
    gate: FitGate,
) -> VtFitResult {
    let fit = WindowedFitter::from_slices(vg, id_abs, Transform::Sqrt).fit(fit_range);
    // A rejected fit keeps the fit diagnostics (slope/intercept/r2/points) so
    // a weak fit stays inspectable; only vt/mobility become NaN.
    let reject = VtFitResult {
        vt: f64::NAN,
        mobility: f64::NAN,
        slope: fit.slope,
        intercept: fit.intercept,
        r2: fit.r2,
        points: fit.points,
    };
    if fit.points < gate.min_points || !fit.slope.is_finite() || fit.slope.abs() <= FLOAT_EPSILON {
        return reject;
    }
    if fit.r2.is_finite() && fit.r2 < gate.min_r2 {
        return reject;
    }
    let vt = -fit.intercept / fit.slope;
    VtFitResult {
        vt,
        mobility: f64::NAN,
        slope: fit.slope,
        intercept: fit.intercept,
        r2: fit.r2,
        points: fit.points,
    }
}

/// Extract V_TH and saturation mobility from the sqrt(|Id|) vs Vg fit.
/// Mobility is `2*slope^2/(cox*aspect)` when both geometry constants are
/// `> 0`, else NaN; a rejected fit (NaN `vt`) is returned unchanged.
pub(in crate::transfer) fn extract_vt_mu(
    vg: &[f64],
    id_abs: &[f64],
    context: ExtractionContext,
    fit_range: Option<(f64, f64)>,
    gate: FitGate,
) -> VtFitResult {
    let fit = extract_vth_elr(vg, id_abs, fit_range, gate);
    if !fit.vt.is_finite() {
        return fit;
    }
    let mobility = if context.cox_f_per_cm2 > 0.0 && context.aspect_ratio > 0.0 {
        2.0 * fit.slope * fit.slope / (context.cox_f_per_cm2 * context.aspect_ratio)
    } else {
        f64::NAN
    };
    VtFitResult { mobility, ..fit }
}
