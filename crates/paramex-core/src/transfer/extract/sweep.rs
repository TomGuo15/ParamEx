//! Per-sweep Transfer extraction primitives.
//!
//! This is downstream of `transfer::metrics::sweep`: that module decides how a
//! round-trip curve is split, while this module extracts metrics from one branch
//! after the split has already happened.

use crate::transfer::metrics::on_off::on_off_ratio;
use crate::transfer::metrics::ss::extract_ss;
use crate::transfer::metrics::vth::{extract_vt_mu, FitGate};
use crate::transfer::types::{ExtractionContext, SweepData, SweepExtractionResult};

/// Minimum samples for the per-branch V_TH and SS fits.
pub(super) const SWEEP_EXTRACT_MIN_POINTS: usize = 5;

/// Sentinel projection used when no usable branch exists.
pub(super) const NAN_SWEEP_METRICS: SweepExtractionResult = SweepExtractionResult {
    vt: f64::NAN,
    mobility: f64::NAN,
    ss_mv_dec: f64::NAN,
    ion: f64::NAN,
    ioff: f64::NAN,
    on_off_ratio: f64::NAN,
};

/// Run V_TH / mobility / SS / Ion-Ioff extraction on one sweep with the given
/// windows. `vt_gate` is forwarded to [`extract_vt_mu`]; the caller relaxes it
/// for a user-pinned window.
pub(super) fn extract_single_sweep(
    sweep: &SweepData,
    context: ExtractionContext,
    vt_range: Option<(f64, f64)>,
    ss_range: Option<(f64, f64)>,
    vt_gate: FitGate,
) -> SweepExtractionResult {
    let vt_res = extract_vt_mu(&sweep.vg, &sweep.id_abs, context, vt_range, vt_gate);
    let ss_res = extract_ss(&sweep.vg, &sweep.id_abs, ss_range, SWEEP_EXTRACT_MIN_POINTS);
    let (ion, ioff, on_off_ratio) = on_off_ratio(&sweep.id_abs);
    SweepExtractionResult {
        vt: vt_res.vt,
        mobility: vt_res.mobility,
        ss_mv_dec: ss_res.ss_mv_dec,
        ion,
        ioff,
        on_off_ratio,
    }
}
