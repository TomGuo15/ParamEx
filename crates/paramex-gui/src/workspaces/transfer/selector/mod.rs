//! The central two-graph window selector. Thin view over core seams;
//! commits windows through the deferred-recompute pattern (geometry.rs template).
//!
//! This module owns the public `show` entry point; separable concerns live in
//! private siblings: `window` (mode/window policy), `render` (plot + controls
//! pipeline), `drag` (on-chart hit-testing), and `controls` (per-graph inputs).
mod controls;
mod drag;
mod render;
mod window;

pub mod bands;
pub mod graph;
pub mod strip;

pub use window::{backward_display, derive_mode};

use eframe::egui;
use paramex_core::transfer::{ExpertRanges, ExpertWindow, Session};

use crate::state::EditBuffers;
use crate::ui_kit::{self, Variant};
use crate::workspaces::transfer::state::{GraphMode, PlotCache, PlotKind, SelectorUi};

use drag::Cmd;
use render::{forget_selector_buffers, render_selector_column};

fn show_selector_pair(ui: &mut egui::Ui, mut add: impl FnMut(&mut egui::Ui, PlotKind)) {
    if super::plot_pair_should_stack(ui.available_size()) {
        let height = ui.available_height();
        super::show_stacked_plot_pair(ui, "selector_plot_pair", height, |ui, index| {
            add(
                ui,
                if index == 0 {
                    PlotKind::Vt
                } else {
                    PlotKind::Ss
                },
            );
        });
    } else {
        ui.columns(2, |cols| {
            add(&mut cols[0], PlotKind::Vt);
            add(&mut cols[1], PlotKind::Ss);
        });
    }
}

/// `Reset to Auto` is authoritative for the selected file. Clicking the header
/// button steals focus from any open V_G min/max field, whose `lost_focus` then
/// commits a `Window` pin LATER in `cmds` than the `AutoFitAll` pushed for the
/// reset — and that field re-seeds from the NOT-yet-cleared window (the clear is
/// deferred to the APPLY phase), so it would re-pin exactly what the reset clears
/// and the reset would be silently undone. Make the reset win: drop any same-file
/// `Window` collected in the same frame. (Unlike the direction toggle — which
/// changes mode immediately, so forgetting the buffer re-seeds to the new
/// direction — here the buffer re-seeds to the stale window, so the stray commit
/// must be dropped, not just the buffer forgotten.)
fn drop_windows_superseded_by_reset(cmds: &mut Vec<Cmd>) {
    let reset_ids: Vec<String> = cmds
        .iter()
        .filter_map(|c| match c {
            Cmd::AutoFitAll { id } => Some(id.clone()),
            _ => None,
        })
        .collect();
    if reset_ids.is_empty() {
        return;
    }
    cmds.retain(|c| !matches!(c, Cmd::Window { id, .. } if reset_ids.contains(id)));
}

/// Render the window selector for the selected file.
pub fn show(
    ui: &mut egui::Ui,
    session: &mut Session,
    sel: &mut SelectorUi,
    plot: &mut PlotCache,
    edits: &mut EditBuffers,
) {
    ui_kit::card_slot(ui, |ui| {
        let selected = session.selected_fit_window_file();
        let manual_range = selected
            .map(|selected| has_manual_expert_range(selected.expert_ranges))
            .unwrap_or(false);
        let reset_clicked = ui_kit::header_action_row(ui, "FIT", |ui| {
            ui.add_enabled_ui(manual_range, |ui| {
                ui_kit::header_action(ui, "Reset to Auto", Variant::Secondary).clicked()
            })
            .inner
        });
        // Drop cache entries for files removed since last frame.
        plot.prune_to(|id| session.has_file(id));
        // The selected-file view borrows only immutable selector inputs. Its last
        // use is the plot.view() call below, so the &session borrow ends before
        // the deferred APPLY phase mutates window state.
        if let Some(selected) = selected {
            let id = selected.file_id.to_string();
            let er = selected.expert_ranges;

            // Re-derive per-graph mode and forget stale buffers on a file switch.
            if sel.sync_file(
                &id,
                derive_mode(er.vt_range, er.vt_range_bwd),
                derive_mode(er.ss_range, er.ss_range_bwd),
            ) {
                forget_selector_buffers(edits, &id);
            }

            // has_bwd and windows come from the committed result, matching the
            // displayed split whenever a result exists.
            let has_bwd = selected.has_backward_sweep;
            let (vt_w, ss_w, vt_wb, ss_wb) = (
                selected.vt_window,
                selected.ss_window,
                selected.vt_window_bwd,
                selected.ss_window_bwd,
            );

            // Per-file derived data (split sweeps, scatters, axes, fitters), cached once
            // per curve and borrowed immutably for the whole render phase.
            let view = plot.view(&id, selected.vg, selected.id_abs);
            let axes = view.axes();

            let vt_live = sel.live_window(PlotKind::Vt);
            let ss_live = sel.live_window(PlotKind::Ss);
            // Snapshot the drag state BEFORE the columns run `grab_band` (which mutates it):
            // the bands are drawn from this, and the strips read it too, so they stay in sync.
            let drag_at_start = sel.drag();

            // COLLECT: gather all cmds during the render phase (deferred pattern).
            let mut cmds: Vec<Cmd> = Vec::new();

            if reset_clicked {
                cmds.push(Cmd::AutoFitAll { id: id.clone() });
            }

            // Each responsive graph group is the plot, its on-chart band grab, then controls.
            // Normal windows stay side by side; tall/narrow bodies stack VTH above SS.
            show_selector_pair(ui, |ui, kind| {
                let (plot_id, y_bounds, fwd_window, bwd_window, live) = match kind {
                    PlotKind::Vt => ("selector_vt", axes.vt_y(), vt_w, vt_wb, vt_live),
                    PlotKind::Ss => ("selector_ss", axes.ss_y(), ss_w, ss_wb, ss_live),
                };
                render_selector_column(
                    ui,
                    view,
                    sel,
                    edits,
                    &mut cmds,
                    &render::GraphInputs {
                        file_id: &id,
                        kind,
                        plot_id,
                        axes,
                        y_bounds,
                        fwd_window,
                        bwd_window,
                        live,
                        has_bwd,
                        show_values: true,
                        drag_at_start,
                    },
                );
            });

            // A focus-stolen field can append a Window pin that would undo a same-frame
            // Reset to Auto; let the reset win before applying.
            drop_windows_superseded_by_reset(&mut cmds);

            // APPLY: now the snapshot borrow is released and we can mutate session.
            for c in cmds {
                match c {
                    Cmd::Window { id, which, win } => {
                        session.set_expert_window(&id, which, win);
                        // A manual window pins that graph's mode to the edited direction
                        // (so an edit made while in Auto flips the radio to Fwd/Bwd).
                        match which {
                            ExpertWindow::FwdVt => sel.set_mode(PlotKind::Vt, GraphMode::Fwd),
                            ExpertWindow::BwdVt => sel.set_mode(PlotKind::Vt, GraphMode::Bwd),
                            ExpertWindow::FwdSs => sel.set_mode(PlotKind::Ss, GraphMode::Fwd),
                            ExpertWindow::BwdSs => sel.set_mode(PlotKind::Ss, GraphMode::Bwd),
                        }
                    }
                    Cmd::AutoFitAll { id } => {
                        session.clear_expert_windows(&id);
                        sel.reset_modes_to_auto();
                    }
                }
                // Windows are curve-independent, so the cache stays valid.
            }
        } else {
            let view = plot.empty_view();
            let axes = view.axes();
            let mut cmds = Vec::new();
            ui.add_enabled_ui(false, |ui| {
                show_selector_pair(ui, |ui, kind| {
                    let (plot_id, y_bounds) = match kind {
                        PlotKind::Vt => ("selector_vt", axes.vt_y()),
                        PlotKind::Ss => ("selector_ss", axes.ss_y()),
                    };
                    render_selector_column(
                        ui,
                        view,
                        sel,
                        edits,
                        &mut cmds,
                        &render::GraphInputs {
                            file_id: "",
                            kind,
                            plot_id,
                            axes,
                            y_bounds,
                            fwd_window: None,
                            bwd_window: None,
                            live: None,
                            has_bwd: true,
                            show_values: false,
                            drag_at_start: None,
                        },
                    );
                });
            });
        }
    });
}

fn has_manual_expert_range(er: ExpertRanges) -> bool {
    er.vt_range.is_some()
        || er.ss_range.is_some()
        || er.vt_range_bwd.is_some()
        || er.ss_range_bwd.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_supersedes_a_same_file_window_commit() {
        // The focus-steal path collects [AutoFitAll, Window] for the SAME file in one
        // frame; the reset must win (the Window would re-pin what the reset clears).
        let id = "file-1".to_string();
        let mut cmds = vec![
            Cmd::AutoFitAll { id: id.clone() },
            Cmd::Window {
                id,
                which: ExpertWindow::FwdVt,
                win: Some((0.5, 1.5)),
            },
        ];
        drop_windows_superseded_by_reset(&mut cmds);
        assert_eq!(cmds.len(), 1);
        assert!(matches!(cmds[0], Cmd::AutoFitAll { .. }));
    }

    #[test]
    fn a_window_commit_without_a_reset_is_kept() {
        let mut cmds = vec![Cmd::Window {
            id: "file-1".to_string(),
            which: ExpertWindow::FwdVt,
            win: Some((0.5, 1.5)),
        }];
        drop_windows_superseded_by_reset(&mut cmds);
        assert_eq!(
            cmds.len(),
            1,
            "a normal edit with no reset must still commit"
        );
    }

    #[test]
    fn reset_does_not_drop_a_different_files_window() {
        let mut cmds = vec![
            Cmd::AutoFitAll {
                id: "file-1".to_string(),
            },
            Cmd::Window {
                id: "file-2".to_string(),
                which: ExpertWindow::FwdVt,
                win: Some((0.5, 1.5)),
            },
        ];
        drop_windows_superseded_by_reset(&mut cmds);
        assert_eq!(
            cmds.len(),
            2,
            "a reset on one file must not drop another file's window"
        );
    }
}
