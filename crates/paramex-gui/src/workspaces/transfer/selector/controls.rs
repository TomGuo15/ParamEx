//! The per-graph control column for the selector: the Forward/Backward direction
//! selector, the double-thumb edit strip, and the two numeric edge fields. Pushes
//! deferred `Cmd`s collected by `mod.rs::show`; reads the just-deferred command via
//! `drag::pending_window` so the strip never flashes back for one frame after release.

use eframe::egui;

use super::drag::{pending_window, Cmd};
use super::strip;
use super::window::backward_display;
use crate::state::EditBuffers;
use crate::theme::{SUISEI_DARK, SUISEI_MAIN};
use crate::ui_kit;
use crate::workspaces::transfer::state::{DragState, GraphMode, PlotKind, SelectorUi};

/// Inputs for the controls below one selector graph.
pub(super) struct GraphControlInputs<'a> {
    pub id: &'a str,
    pub kind: PlotKind,
    pub drag_at_start: Option<DragState>,
    pub fwd_committed: Option<(f64, f64)>,
    pub bwd_committed: Option<(f64, f64)>,
    pub has_backward: bool,
    pub axis: (f64, f64),
    pub show_values: bool,
}

/// Per-graph controls: a Forward/Backward direction selector. There is no per-graph
/// "Auto" button; reverting to auto is the header's "Reset to Auto".
/// Both directions show the edit strip + numeric fields; a single-sweep file shows a
/// lone Forward marker. `Auto` survives only as the internal unpinned default.
pub(super) fn graph_controls(
    ui: &mut egui::Ui,
    inputs: GraphControlInputs<'_>,
    sel: &mut SelectorUi,
    edits: &mut EditBuffers,
    cmds: &mut Vec<Cmd>,
) {
    let GraphControlInputs {
        id,
        kind,
        drag_at_start,
        fwd_committed,
        bwd_committed,
        has_backward,
        axis,
        show_values,
    } = inputs;
    // Read mode by value (GraphMode: Copy) — no held &mut field borrow.
    let old_mode = sel.mode(kind);
    let mut mode = old_mode;
    // Direction selector: Forward / Backward only. Auto is an internal unpinned
    // state and is shown as Forward because edits apply to the forward window.
    // Single-sweep files have only the forward window → a lone marker.
    if has_backward {
        let active = if matches!(mode, GraphMode::Bwd) { 1 } else { 0 };
        // color the toggle with the LINE colors (forward=blue, backward=green) so it
        // doubles as the legend, so no separate legend row is needed.
        if let Some(clicked) = ui_kit::segmented_two_colored(
            ui,
            ["Forward", "Backward"],
            active,
            [SUISEI_MAIN, SUISEI_DARK],
        ) {
            mode = if clicked == 1 {
                GraphMode::Bwd
            } else {
                GraphMode::Fwd
            };
        }
    } else if ui_kit::colored_button(ui, "Forward", SUISEI_MAIN).clicked() {
        mode = GraphMode::Fwd;
    }
    if mode != old_mode {
        sel.set_mode(kind, mode);
        // A pending numeric edge edit was typed against the OLD direction. Clicking
        // the toggle steals the field's focus and flips the commit target (`which`)
        // the same frame, so a buffered value would otherwise commit to the OTHER
        // sweep direction (wrong window). Discard the uncommitted edit instead.
        edits.forget(&format!("num:{id}:{kind:?}:lo"));
        edits.forget(&format!("num:{id}:{kind:?}:hi"));
    }

    let Some(which) = kind.edit_which(mode) else {
        return;
    };
    // `edit_which` coerces Auto→Fwd, so bwd is reached only in explicit Bwd mode.
    let is_bwd = matches!(mode, GraphMode::Bwd);
    // Numeric seed: backward falls back to the forward window when its own bwd window is None.
    let display = match mode {
        GraphMode::Auto | GraphMode::Fwd => fwd_committed.unwrap_or(axis),
        GraphMode::Bwd => backward_display(fwd_committed, bwd_committed).unwrap_or(axis),
    };
    let active_live = DragState::window_for_kind(drag_at_start, kind);
    let (mut lo, mut hi) = active_live.unwrap_or(display);
    // On the frame a band drag RELEASES, the new window is in `cmds` (deferred) but
    // not yet in `display`/the session — reflect it so the strip doesn't flash back to
    // the pre-drag window for one frame.
    if let Some((clo, chi)) = pending_window(cmds, which) {
        lo = clo;
        hi = chi;
    }

    // Partial extraction can return no window; keep the strip on the full-axis
    // fallback so the user can manually recover instead of losing the control.
    let accent = if is_bwd { SUISEI_DARK } else { SUISEI_MAIN };
    let outcome = strip::double_thumb_strip(
        ui,
        strip::StripInputs {
            id_salt: &format!("strip:{id}:{kind:?}"),
            v_min: axis.0,
            v_max: axis.1,
            lo: &mut lo,
            hi: &mut hi,
            accent,
        },
    );
    if outcome.dragging {
        sel.set_strip_drag(kind, lo, hi);
    }
    if outcome.released {
        sel.clear_drag();
        cmds.push(Cmd::Window {
            id: id.to_string(),
            which,
            win: Some((lo, hi)),
        });
    }
    // Numeric fields ALWAYS render (including Auto), commit on lost_focus.
    ui_kit::terminal_numeric_row(ui, |ui| {
        ui.spacing_mut().item_spacing.x = ui_kit::INPUT_LABEL_GAP;
        // field_label_rich: field_label is not markup-aware (would paint "<sub>").
        ui_kit::field_label_rich(ui, "V<sub>G</sub> min");
        edit_edge(
            ui,
            edits,
            &format!("num:{id}:{kind:?}:lo"),
            show_values.then_some(lo),
            |v| {
                cmds.push(Cmd::Window {
                    id: id.to_string(),
                    which,
                    win: Some((v, hi)),
                });
            },
        );
        ui_kit::field_label_rich(ui, "V<sub>G</sub> max");
        edit_edge(
            ui,
            edits,
            &format!("num:{id}:{kind:?}:hi"),
            show_values.then_some(hi),
            |v| {
                cmds.push(Cmd::Window {
                    id: id.to_string(),
                    which,
                    win: Some((lo, v)),
                });
            },
        );
    });
}

/// One numeric edge field: the same focus-tracked commit-on-`lost_focus` shape
/// as `geometry.rs::edit_dim`, with shared three-decimal formatting. Parse
/// failures revert silently because the selector contract is clamp-only.
fn edit_edge(
    ui: &mut egui::Ui,
    edits: &mut EditBuffers,
    key: &str,
    current: Option<f64>,
    mut on_commit: impl FnMut(f64),
) {
    let current = current.map(crate::format_ui::fmt_num3).unwrap_or_default();
    // The shared helper returns only changed text, so focus stealing cannot pin
    // an unchanged full-axis window.
    if let Some(text) = ui_kit::singleline_edit_commit(
        ui,
        edits,
        key,
        &current,
        ui_kit::COMPACT_NUMERIC_INPUT_WIDTH,
    ) {
        if let Ok(value) = text.trim().parse::<f64>() {
            on_commit(value);
        }
    }
}
