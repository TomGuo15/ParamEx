//! Primary-sweep Transfer metric policy.

use crate::transfer::metrics::hysteresis::extract_delta_vth_hysteresis_curve_shift;
use crate::transfer::metrics::on_off::on_off_ratio;
use crate::transfer::metrics::ss::{extract_ss, SsFitResult};
use crate::transfer::metrics::vth::{extract_vt_mu, FitGate, VtFitResult};
use crate::transfer::types::{ExtractionContext, ExtractionStatus, SweepData};

use super::{SplitSweep, SweepWindows, WindowSource};

const HYST_TRIM_FRACTION: f64 = 0.2;
const HYST_MIN_POINTS: usize = 12;
const SS_EXTRACT_MIN_POINTS: usize = 5;

/// V_TH fit gate for an auto-selected primary window: more points and a high
/// R² so a weak automatic pick is reported as a failed fit.
const VT_PRIMARY_GATE_AUTO: FitGate = FitGate {
    min_points: 10,
    min_r2: 0.99,
};
/// V_TH fit gate for a user-pinned primary window: relaxed so the chosen
/// window is honoured as-is.
const VT_PRIMARY_GATE_PINNED: FitGate = FitGate {
    min_points: 5,
    min_r2: 0.0,
};

pub(super) struct PrimaryMetrics {
    pub(super) vt_result: VtFitResult,
    pub(super) ss_result: SsFitResult,
    pub(super) ion: f64,
    pub(super) ioff: f64,
    pub(super) on_off_ratio: f64,
    pub(super) hysteresis: f64,
    /// `ok` means both primary V_TH and SS fits are finite. Derived quantities
    /// such as hysteresis may still be unavailable and remain `NaN`.
    pub(super) status: ExtractionStatus,
}

/// Run every primary-sweep metric. `full_id_abs` is the whole curve's current
/// so on/off spans both branches; hysteresis reuses the existing split rather
/// than re-splitting the same curve.
pub(super) fn extract_primary_metrics(
    primary: &SweepData,
    full_id_abs: &[f64],
    split: &SplitSweep,
    context: ExtractionContext,
    windows: &SweepWindows,
) -> PrimaryMetrics {
    let vt_gate = match windows.vt_source {
        WindowSource::Auto => VT_PRIMARY_GATE_AUTO,
        WindowSource::Pinned => VT_PRIMARY_GATE_PINNED,
    };
    let vt_result = extract_vt_mu(&primary.vg, &primary.id_abs, context, windows.vt, vt_gate);
    let ss_result = extract_ss(
        &primary.vg,
        &primary.id_abs,
        windows.ss,
        SS_EXTRACT_MIN_POINTS,
    );
    let (ion, ioff, on_off_ratio) = on_off_ratio(full_id_abs);
    let hysteresis = extract_delta_vth_hysteresis_curve_shift(
        &split.forward,
        &split.backward,
        HYST_TRIM_FRACTION,
        HYST_MIN_POINTS,
    );
    let status = if vt_result.vt.is_finite() && ss_result.ss_mv_dec.is_finite() {
        ExtractionStatus::Ok
    } else {
        ExtractionStatus::Partial
    };
    PrimaryMetrics {
        vt_result,
        ss_result,
        ion,
        ioff,
        on_off_ratio,
        hysteresis,
        status,
    }
}
