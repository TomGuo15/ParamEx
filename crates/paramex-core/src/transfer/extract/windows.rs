//! Transfer V_TH and SS auto-window selection policy.

use crate::transfer::metrics::ss::{select_ss_window, SsWindowSelector};
use crate::transfer::metrics::vth::{select_elr_vt_window, VtWindowSelector, DEFAULT_VT_R2_LADDER};

/// Production ELR V_TH selector settings; `extract_metrics` never overrides them.
const VT_SELECTOR: VtWindowSelector = VtWindowSelector {
    window_size: 30,
    step: 1,
    min_points: 10,
    min_r2: 0.99,
    r2_ladder: &DEFAULT_VT_R2_LADDER,
};

/// Production SS selector settings; `extract_metrics` never overrides them.
const SS_SELECTOR: SsWindowSelector = SsWindowSelector {
    max_points: 30,
    min_decades: 1.0,
    min_points: 5,
    min_r2: 0.9,
    off_guard_decades: 0.3,
};

pub(super) fn auto_vt_window(vg: &[f64], id_abs: &[f64]) -> Option<(f64, f64)> {
    select_elr_vt_window(vg, id_abs, &VT_SELECTOR)
}

pub(super) fn auto_ss_window(vg: &[f64], id_abs: &[f64]) -> Option<(f64, f64)> {
    select_ss_window(vg, id_abs, &SS_SELECTOR)
}
