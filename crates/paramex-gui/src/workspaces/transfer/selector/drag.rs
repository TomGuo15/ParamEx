//! On-chart drag hit-testing for the two-graph selector: the deferred `Cmd`
//! enum, its `pending_window` reader, and the `grab_band` band hit-test. These
//! communicate with the orchestrator (`mod.rs::show`) and the controls column
//! only via `SelectorUi`, `EditBuffers`, the `cmds` Vec, and the pub
//! `bands`/`strip` siblings.

use eframe::egui;
use paramex_core::transfer::ExpertWindow;

use super::{bands, strip};
use crate::workspaces::transfer::state::{DragEdge, GraphMode, PlotKind, SelectorUi};

/// Deferred commit collected during the RENDER phase and applied AFTER (the
/// snapshot→collect→apply pattern that keeps `&session` and `&mut session` apart).
pub(super) enum Cmd {
    Window {
        id: String,
        which: ExpertWindow,
        win: Option<(f64, f64)>,
    },
    AutoFitAll {
        id: String,
    },
}

/// The pending (deferred) window commit for `which` collected THIS frame, if any. The
/// strip reads this so a just-released band drag shows its final position immediately,
/// instead of flashing back to the pre-drag window for one frame before the deferred command applies.
pub(super) fn pending_window(cmds: &[Cmd], which: ExpertWindow) -> Option<(f64, f64)> {
    cmds.iter().rev().find_map(|c| match c {
        Cmd::Window {
            which: w,
            win: Some(win),
            ..
        } if *w == which => Some(*win),
        _ => None,
    })
}

/// Inputs identifying the graph and committed window being hit-tested.
pub(super) struct GrabInputs<'a> {
    pub kind: PlotKind,
    pub mode: GraphMode,
    pub committed_window: Option<(f64, f64)>,
    pub sorted_xs: &'a [f64],
    pub id: &'a str,
}

/// On-chart band hit-test: drives the selector drag state from a graph's `PlotResponse`.
///
/// - `drag_started` → classify the grab + seed the selector drag.
/// - `dragged` → update live `lo`/`hi` (edge snaps to nearest data-x,
///   Whole slides by the screen delta converted to data units via the transform).
/// - `drag_stopped` → clear the matching drag + push exactly one `Cmd::Window`.
///
/// `sorted_xs` is a snapshot of `fitter.x()` (vg-sorted ascending). Only
/// `drag_stopped()` ever pushes a commit, preserving one commit per drag.
pub(super) fn grab_band(
    resp: egui_plot::PlotResponse<()>,
    inputs: GrabInputs<'_>,
    sel: &mut SelectorUi,
    cmds: &mut Vec<Cmd>,
) {
    let GrabInputs {
        kind,
        mode,
        committed_window,
        sorted_xs,
        id,
    } = inputs;
    use bands::Grab;

    // Only process grabs when there is an active-direction committed window to grab.
    let Some((win_lo, win_hi)) = committed_window else {
        return;
    };
    // Auto is grabbable: dragging its auto-selected window pins it Forward (the mode
    // flip happens in show()'s commit loop, keyed on `which`).
    let Some(which) = kind.edit_which(mode) else {
        return;
    };

    // Per-pixel data scale + a generous 12 px edge-grab tolerance (the old 6 px
    // made the band edges very hard to grab).
    let p0 = resp
        .transform
        .value_from_position(egui::Pos2::new(0.0, 0.0));
    let p1 = resp
        .transform
        .value_from_position(egui::Pos2::new(1.0, 0.0));
    let data_per_px = p1.x - p0.x;
    let edge_data = data_per_px.abs() * 12.0;

    if resp.response.drag_started() {
        if let Some(pos) = resp.response.interact_pointer_pos() {
            let px = resp.transform.value_from_position(pos).x;
            if let Some(grab) = bands::classify_grab(px, win_lo, win_hi, edge_data) {
                let edge = match grab {
                    Grab::EdgeLo => DragEdge::Lo,
                    Grab::EdgeHi => DragEdge::Hi,
                    Grab::Whole => DragEdge::Whole,
                };
                sel.start_drag(kind, edge, win_lo, win_hi);
            }
        }
    }

    if resp.response.dragged() {
        // Only update drag state if it belongs to THIS graph.
        sel.update_drag_for(kind, |drag| {
            let grab = match drag.edge() {
                DragEdge::Lo => Grab::EdgeLo,
                DragEdge::Hi => Grab::EdgeHi,
                DragEdge::Whole => Grab::Whole,
            };
            match grab {
                Grab::EdgeLo => {
                    if let Some(pos) = resp.response.interact_pointer_pos() {
                        let px = resp.transform.value_from_position(pos).x;
                        let snapped = bands::snap_to_nearest_x(sorted_xs, px);
                        let (_, old_hi) = drag.window();
                        let (lo, hi) = strip::clamp_pair(snapped, old_hi);
                        drag.set_window(lo, hi);
                    }
                }
                Grab::EdgeHi => {
                    if let Some(pos) = resp.response.interact_pointer_pos() {
                        let px = resp.transform.value_from_position(pos).x;
                        let snapped = bands::snap_to_nearest_x(sorted_xs, px);
                        let (old_lo, _) = drag.window();
                        let (lo, hi) = strip::clamp_pair(old_lo, snapped);
                        drag.set_window(lo, hi);
                    }
                }
                Grab::Whole => {
                    // Slide both edges by the screen drag delta converted to data units.
                    let delta_px = resp.response.drag_delta().x;
                    let data_delta = (delta_px as f64) * data_per_px;
                    let (old_lo, old_hi) = drag.window();
                    let (lo, hi) = strip::clamp_pair(old_lo + data_delta, old_hi + data_delta);
                    drag.set_window(lo, hi);
                }
            }
        });
    }

    if resp.response.drag_stopped() {
        if let Some(drag) = sel.finish_drag_for(kind) {
            let (lo, hi) = drag.window();
            let (lo, hi) = strip::clamp_pair(lo, hi);
            cmds.push(Cmd::Window {
                id: id.to_string(),
                which,
                win: Some((lo, hi)),
            });
        }
    }

    // Affordance: the cursor telegraphs what a press would grab (or is grabbing) —
    // edges resize, the interior slides. Purely visual; cmds stay drag-driven.
    if let Some(drag) = sel.drag_for(kind) {
        let grab = match drag.edge() {
            DragEdge::Lo => Grab::EdgeLo,
            DragEdge::Hi => Grab::EdgeHi,
            DragEdge::Whole => Grab::Whole,
        };
        resp.response
            .ctx
            .set_cursor_icon(bands::cursor_for_grab(grab, true));
    } else if let Some(pos) = resp.response.hover_pos() {
        let px = resp.transform.value_from_position(pos).x;
        if let Some(grab) = bands::classify_grab(px, win_lo, win_hi, edge_data) {
            resp.response
                .ctx
                .set_cursor_icon(bands::cursor_for_grab(grab, false));
        }
    }
}
