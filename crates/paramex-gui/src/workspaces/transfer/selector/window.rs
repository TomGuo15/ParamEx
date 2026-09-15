//! Pure window-selection policy for the Transfer fit-window selector.
//!
//! Rendering, controls, and drag hit-testing all need the same answers: which
//! expert-range field a graph edits, which committed/live window is visible, and
//! which branch supplies the active fit. Keeping that policy here keeps the view
//! modules as consumers instead of each restating part of the same state machine.

use paramex_core::transfer::ExpertWindow;

use crate::workspaces::transfer::state::{CurveView, GraphMode, PlotKind, SweepBranch};

impl PlotKind {
    /// The `expert_ranges` field a strip/numeric edit on this graph targets in the
    /// given mode; `None` in Auto (Auto edits nothing — reverting to auto is the global
    /// "Reset to Auto").
    pub fn window_which(self, mode: GraphMode) -> Option<ExpertWindow> {
        match (self, mode) {
            (PlotKind::Vt, GraphMode::Fwd) => Some(ExpertWindow::FwdVt),
            (PlotKind::Vt, GraphMode::Bwd) => Some(ExpertWindow::BwdVt),
            (PlotKind::Ss, GraphMode::Fwd) => Some(ExpertWindow::FwdSs),
            (PlotKind::Ss, GraphMode::Bwd) => Some(ExpertWindow::BwdSs),
            (_, GraphMode::Auto) => None,
        }
    }

    /// The `expert_ranges` field an edit on this graph targets, applying the
    /// "Auto edits as Forward" coercion: Auto pins the auto-selected window
    /// Forward, so it edits the same field Fwd does (`window_which(Fwd)`).
    pub fn edit_which(self, mode: GraphMode) -> Option<ExpertWindow> {
        let edit_mode = if matches!(mode, GraphMode::Auto) {
            GraphMode::Fwd
        } else {
            mode
        };
        self.window_which(edit_mode)
    }
}

/// Backward display value: the bwd window, falling back to the forward window when
/// the bwd window is `None`. Visibility is decided separately.
pub fn backward_display(fwd: Option<(f64, f64)>, bwd: Option<(f64, f64)>) -> Option<(f64, f64)> {
    bwd.or(fwd)
}

/// Derive a graph's mode from its two pins on a file-switch: Auto when both
/// `None`; Fwd if the forward pin is `Some` (precedence); else Bwd.
pub fn derive_mode(fwd: Option<(f64, f64)>, bwd: Option<(f64, f64)>) -> GraphMode {
    match (fwd, bwd) {
        (None, None) => GraphMode::Auto,
        (Some(_), _) => GraphMode::Fwd,
        (None, Some(_)) => GraphMode::Bwd,
    }
}

#[allow(clippy::type_complexity)] // reason: three tightly-coupled Option return values
pub fn draw_windows(
    mode: GraphMode,
    fwd_committed: Option<(f64, f64)>,
    bwd_committed: Option<(f64, f64)>,
    live: Option<(f64, f64)>,
    axis: (f64, f64),
) -> (
    Option<(f64, f64)>,
    Option<(f64, f64)>,
    Option<((f64, f64), SweepBranch)>,
) {
    match mode {
        // Auto draws like Fwd — its auto-selected forward window is live-editable, so
        // a drag in Auto previews on the forward band (and pins Forward on release).
        GraphMode::Fwd | GraphMode::Auto => {
            let fit = live.or(fwd_committed);
            let draw = fit.or(Some(axis));
            (draw, bwd_committed, fit.map(|w| (w, SweepBranch::Forward)))
        }
        GraphMode::Bwd => {
            let fit = live.or(bwd_committed);
            let draw = fit
                .or_else(|| backward_display(fwd_committed, bwd_committed))
                .or(Some(axis));
            (fwd_committed, draw, fit.map(|w| (w, SweepBranch::Backward)))
        }
    }
}

/// The per-side band-grab inputs both columns compute identically (modulo the
/// `vt_`/`ss_` prefix): the vg-sorted x snapshot for edge-snapping and the
/// active-direction committed window.
pub fn grab_inputs(
    view: &CurveView,
    mode: GraphMode,
    kind: PlotKind,
    fwd_window: Option<(f64, f64)>,
    bwd_window: Option<(f64, f64)>,
) -> (&[f64], Option<(f64, f64)>) {
    let is_bwd = matches!(mode, GraphMode::Bwd);
    let branch = if is_bwd {
        SweepBranch::Backward
    } else {
        SweepBranch::Forward
    };
    let sorted_xs = view.fitter(branch, kind).x();
    let committed_window = if is_bwd {
        backward_display(fwd_window, bwd_window).or(Some(view.axes().vg()))
    } else {
        fwd_window.or(Some(view.axes().vg()))
    };
    (sorted_xs, committed_window)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspaces::transfer::state::PlotCache;

    #[test]
    fn grab_inputs_keep_full_axis_window_when_extraction_has_no_window() {
        let vg = vec![-1.0, 0.0, 1.0, 2.0];
        let id_abs = vec![1e-12, 2e-12, 3e-12, 4e-12];
        let mut cache = PlotCache::default();
        let view = cache.view("partial", &vg, &id_abs);

        let (_xs, window) = grab_inputs(view, GraphMode::Fwd, PlotKind::Vt, None, None);

        assert_eq!(window, Some(view.axes().vg()));
    }

    #[test]
    fn draw_windows_uses_axis_fallback_without_claiming_a_fit_window() {
        let axis = (-1.0, 2.0);

        let (fwd_draw, bwd_draw, active_fit) =
            draw_windows(GraphMode::Auto, None, None, None, axis);

        assert_eq!(fwd_draw, Some(axis));
        assert_eq!(bwd_draw, None);
        assert_eq!(active_fit, None);
    }

    #[test]
    fn backward_draw_falls_back_to_forward_then_axis() {
        let axis = (-1.0, 2.0);
        let fwd = Some((-0.5, 1.0));

        let (_fwd_draw, bwd_draw, active_fit) = draw_windows(GraphMode::Bwd, fwd, None, None, axis);
        assert_eq!(bwd_draw, fwd);
        assert_eq!(active_fit, None);

        let (_fwd_draw, bwd_draw, active_fit) =
            draw_windows(GraphMode::Bwd, None, None, None, axis);
        assert_eq!(bwd_draw, Some(axis));
        assert_eq!(active_fit, None);
    }
}
