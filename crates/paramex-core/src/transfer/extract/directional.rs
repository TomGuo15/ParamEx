//! Forward/backward Transfer sweep policy for metric rows.

use crate::transfer::metrics::vth::FitGate;
use crate::transfer::types::{ExpertRanges, ExtractionContext, SweepExtractionResult};

use super::primary::PrimaryMetrics;
use super::sweep::{extract_single_sweep, NAN_SWEEP_METRICS, SWEEP_EXTRACT_MIN_POINTS};
use super::{SplitSweep, SweepWindows, WindowSource};

/// V_TH fit gate for the per-direction re-extraction with an auto-selected
/// window.
const VT_SWEEP_GATE_AUTO: FitGate = FitGate {
    min_points: SWEEP_EXTRACT_MIN_POINTS,
    min_r2: 0.995,
};
/// V_TH fit gate for the per-direction re-extraction with a user-pinned
/// window: the R² floor is dropped so the pinned window is honoured.
const VT_SWEEP_GATE_PINNED: FitGate = FitGate {
    min_points: SWEEP_EXTRACT_MIN_POINTS,
    min_r2: 0.0,
};

/// Direction-specific projections. With no usable backward sweep, `forward`
/// mirrors the primary result and `backward` is an all-`NaN` sentinel.
pub(super) struct DirectionalMetrics {
    pub(super) forward: SweepExtractionResult,
    pub(super) backward: SweepExtractionResult,
    pub(super) vt_window_bwd: Option<(f64, f64)>,
    pub(super) ss_window_bwd: Option<(f64, f64)>,
}

fn sweep_gate(source: WindowSource) -> FitGate {
    match source {
        WindowSource::Auto => VT_SWEEP_GATE_AUTO,
        WindowSource::Pinned => VT_SWEEP_GATE_PINNED,
    }
}

/// Re-extract each branch with its own windows when a usable backward sweep
/// exists; otherwise project the primary result as the forward row.
pub(super) fn extract_directional_metrics(
    split: &SplitSweep,
    context: ExtractionContext,
    expert_ranges: &ExpertRanges,
    primary_windows: &SweepWindows,
    primary: &PrimaryMetrics,
) -> DirectionalMetrics {
    if split.has_backward {
        let backward_windows = SweepWindows::resolve(
            &split.backward,
            expert_ranges.vt_range_bwd,
            expert_ranges.ss_range_bwd,
        );
        let fwd = extract_single_sweep(
            &split.forward,
            context,
            primary_windows.vt,
            primary_windows.ss,
            sweep_gate(primary_windows.vt_source),
        );
        let bwd = extract_single_sweep(
            &split.backward,
            context,
            backward_windows.vt,
            backward_windows.ss,
            sweep_gate(backward_windows.vt_source),
        );
        return DirectionalMetrics {
            forward: fwd,
            backward: bwd,
            vt_window_bwd: backward_windows.vt,
            ss_window_bwd: backward_windows.ss,
        };
    }

    DirectionalMetrics {
        forward: SweepExtractionResult {
            vt: primary.vt_result.vt,
            mobility: primary.vt_result.mobility,
            ss_mv_dec: primary.ss_result.ss_mv_dec,
            ion: primary.ion,
            ioff: primary.ioff,
            on_off_ratio: primary.on_off_ratio,
        },
        backward: NAN_SWEEP_METRICS,
        vt_window_bwd: None,
        ss_window_bwd: None,
    }
}
