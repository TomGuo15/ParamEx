//! Per-graph selector render pipeline.
//!
//! This module owns the repeated VT/SS column sequence: series construction,
//! plot rendering, on-chart drag capture, and numeric controls.

use eframe::egui;

use super::controls::graph_controls;
use super::drag::{grab_band, Cmd};
use super::window::{draw_windows, grab_inputs};
use super::{bands, graph};
use crate::state::EditBuffers;
use crate::theme::{SUISEI_DARK, SUISEI_MAIN};
use crate::workspaces::transfer::state::{
    AxisRanges, CurveView, DragState, GraphMode, PlotKind, SelectorUi, SweepBranch,
};

// Direction toggle, range strip, numeric row, and their gaps. No plot footer
// follows these controls, so this is the only space withheld from the plot.
const SELECTOR_CONTROLS_RESERVE: f32 = 105.0;

/// Per-graph inputs shared by render, grab, and controls.
pub(super) struct GraphInputs<'a> {
    pub file_id: &'a str,
    pub kind: PlotKind,
    pub plot_id: &'a str,
    pub axes: AxisRanges,
    pub y_bounds: [f64; 2],
    pub fwd_window: Option<(f64, f64)>,
    pub bwd_window: Option<(f64, f64)>,
    pub live: Option<(f64, f64)>,
    pub has_bwd: bool,
    pub show_values: bool,
    pub drag_at_start: Option<DragState>,
}

/// Render one graph: bands (fwd solid, bwd dashed) + scatter + the active-direction
/// dashed fit-line (live drag -> strict `preview_gate`; committed -> weak `committed_line_gate`).
fn render_one(
    ui: &mut egui::Ui,
    view: &CurveView,
    inputs: &GraphInputs<'_>,
    mode: GraphMode,
    series: &[graph::SeriesDraw<'_>],
) -> egui_plot::PlotResponse<()> {
    let kind = inputs.kind;
    let axes = inputs.axes;
    let y_bounds = inputs.y_bounds;
    let fwd_w = inputs.fwd_window;
    let bwd_w = inputs.bwd_window;
    let live = inputs.live;
    let has_bwd = inputs.has_bwd;
    let show_values = inputs.show_values;
    let vg_axis = axes.vg();
    let (fwd_draw, bwd_draw, active_fit) = if show_values {
        draw_windows(mode, fwd_w, bwd_w, live, vg_axis)
    } else {
        (None, None, None)
    };
    let is_live = live.is_some();
    let fit = active_fit.and_then(|(w, branch)| {
        let r = view.fitter(branch, kind).fit(Some(w));
        let ok = if is_live {
            graph::preview_gate(&r)
        } else {
            graph::committed_line_gate(&r)
        };
        if ok {
            let color = match branch {
                SweepBranch::Forward => SUISEI_MAIN,
                SweepBranch::Backward => SUISEI_DARK,
            };
            crate::plot_kit::fit_line_endpoints(r.slope, r.intercept, vg_axis.0, vg_axis.1)
                .map(|points| (points, color))
        } else {
            None
        }
    });
    let mut draws = Vec::new();
    if let Some(w) = fwd_draw {
        draws.push(graph::BandDraw {
            window: w,
            fill: bands::forward_fill(),
            stroke: SUISEI_MAIN,
        });
    }
    if has_bwd {
        if let Some(w) = bwd_draw {
            draws.push(graph::BandDraw {
                window: w,
                fill: bands::backward_fill(),
                stroke: SUISEI_DARK,
            });
        }
    }
    // Return the PlotResponse so callers can hit-test the on-chart bands.
    // Axis titles name the physical quantity and unit; log10 is only the SS plot's
    // internal coordinate transform.
    let (title, y_label) = match kind {
        PlotKind::Vt => (
            "V<sub>TH</sub> fit range",
            "\u{221A}|I<sub>D</sub>| (A<sup>1/2</sup>)",
        ),
        PlotKind::Ss => ("SS fit range", "|I<sub>D</sub>| (A)"),
    };
    // Fill the card vertically: reserve exactly the per-graph controls rendered
    // below this plot. The legend row is gone and axis titles live
    // inside the plot, so there is no footer reserve after the terminal fields.
    let plot_h = selector_plot_height(ui.available_height());
    graph::render_graph(
        ui,
        graph::GraphConfig {
            id_source: inputs.plot_id,
            title,
            x_label: "Gate voltage V<sub>G</sub> (V)",
            y_label,
            x_bounds: vg_axis,
            y_bounds,
            y_log: matches!(kind, PlotKind::Ss),
            height: plot_h,
            show_scale_values: show_values,
        },
        series,
        fit,
        &draws,
    )
}

fn selector_plot_height(available: f32) -> f32 {
    (available - SELECTOR_CONTROLS_RESERVE).max(0.0)
}

/// Drop this file's selector edit buffers so the numeric fields re-sync to the
/// committed windows on file-switch. Uses `PlotKind` Debug so the keys match
/// `graph_controls` exactly.
pub(super) fn forget_selector_buffers(edits: &mut EditBuffers, id: &str) {
    for kind in [PlotKind::Vt, PlotKind::Ss] {
        edits.forget(&format!("num:{id}:{kind:?}:lo"));
        edits.forget(&format!("num:{id}:{kind:?}:hi"));
    }
}

/// Render one selector column from its cached view. The drag snapshot is computed
/// before painting so the second column observes the same interaction state even
/// when the first column updates the shared drag state.
pub(super) fn render_selector_column(
    ui: &mut egui::Ui,
    view: &CurveView,
    sel: &mut SelectorUi,
    edits: &mut EditBuffers,
    cmds: &mut Vec<Cmd>,
    inputs: &GraphInputs<'_>,
) {
    // Read the mode by value before the control renderer mutates selector state.
    let mode = sel.mode(inputs.kind);
    let mut series = Vec::new();
    if inputs.show_values {
        series.push(graph::SeriesDraw {
            name: "Forward sweep",
            points: view.scatter(SweepBranch::Forward, inputs.kind),
            color: SUISEI_MAIN,
        });
    }
    // The bwd scatter is always cached (result-independent); drawing stays gated
    // on the committed result's has_backward_sweep, exactly as before.
    if inputs.show_values && inputs.has_bwd {
        series.push(graph::SeriesDraw {
            name: "Backward sweep",
            points: view.scatter(SweepBranch::Backward, inputs.kind),
            color: SUISEI_DARK,
        });
    }
    let resp = render_one(ui, view, inputs, mode, &series);
    if inputs.show_values {
        let (sorted_xs, committed_window) = grab_inputs(
            view,
            mode,
            inputs.kind,
            inputs.fwd_window,
            inputs.bwd_window,
        );
        grab_band(
            resp,
            super::drag::GrabInputs {
                kind: inputs.kind,
                mode,
                committed_window,
                sorted_xs,
                id: inputs.file_id,
            },
            sel,
            cmds,
        );
    }
    graph_controls(
        ui,
        super::controls::GraphControlInputs {
            id: inputs.file_id,
            kind: inputs.kind,
            drag_at_start: inputs.drag_at_start,
            fwd_committed: inputs.fwd_window,
            bwd_committed: inputs.bwd_window,
            has_backward: inputs.has_bwd,
            axis: inputs.axes.vg(),
            show_values: inputs.show_values,
        },
        sel,
        edits,
        cmds,
    );
}
