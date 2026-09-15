//! Transfer workspace page composition.

use eframe::egui;
use egui_notify::Toasts;

use super::{panels, selector};
use crate::layout::{self, ShellRects};
use crate::state::EditBuffers;
use crate::workspaces::transfer::layout as transfer_layout;
use crate::workspaces::transfer::state::{CoxUi, TransferResultsView};
use crate::workspaces::transfer::TransferWorkspace;

pub fn show(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    shell: &ShellRects,
    workspace: &mut TransferWorkspace,
    edits: &mut EditBuffers,
    toasts: &mut Toasts,
) {
    // Each column renders into a FIXED rect with its own `id_salt` (see show_in_rect),
    // so render ORDER is independent of both visual position and widget Ids. The
    // RIGHT column (Gate Oxide / Device Geometry inputs) renders BEFORE the CENTER
    // column (results table + Export CSV) on purpose: the Cox / per-file W/L fields
    // commit on lost_focus, and clicking "Export CSV" steals their focus in the SAME
    // frame. Reading the results + export AFTER those inputs commit makes the
    // exported CSV reflect the value the user just typed, not the stale pre-edit
    // Cox/geometry. Do not reorder center before right. (Intra-column tab order is
    // unchanged; only the rare cross-column tab transition differs.)
    layout::show_in_rect(ui, "left_column_rect", shell.left, |ui| {
        show_left_column(ui, ctx, workspace, toasts);
    });
    layout::show_in_rect(ui, "right_column_rect", shell.right, |ui| {
        show_right_column(ui, workspace, edits, toasts);
    });
    layout::show_in_rect(ui, "center_column_rect", shell.center, |ui| {
        show_center_column(ui, ctx, workspace, edits);
    });
}

fn show_left_column(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    workspace: &mut TransferWorkspace,
    toasts: &mut Toasts,
) {
    let stack = layout::fixed_bottom_stack(ui.available_height(), layout::SELECTED_METRICS_HEIGHT);
    layout::show_card_stack(ui, stack, |ui, slot| match slot {
        layout::StackSlot::Top => {
            panels::file_list::show(ui, ctx, workspace, toasts);
        }
        layout::StackSlot::Bottom => {
            panels::selected_metrics::show(ui, &workspace.session);
        }
    });
}

fn show_center_column(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    workspace: &mut TransferWorkspace,
    edits: &mut EditBuffers,
) {
    // Same height math as the left column (flex top + fixed bottom), so the
    // high-density data cards keep one seam at every window size.
    let stack = layout::fixed_bottom_stack(ui.available_height(), layout::SELECTED_METRICS_HEIGHT);
    layout::show_card_stack(ui, stack, |ui, slot| match slot {
        layout::StackSlot::Top => match workspace.ui.results_view() {
            TransferResultsView::Transfer => {
                let selector = workspace.ui.selector_mut();
                selector::show(
                    ui,
                    &mut workspace.session,
                    selector,
                    &mut workspace.plot,
                    edits,
                );
            }
            TransferResultsView::Output => {
                panels::output_plot::show(
                    ui,
                    &mut workspace.session,
                    edits,
                    &mut workspace.output_plot,
                );
            }
        },
        layout::StackSlot::Bottom => {
            panels::results_table::show(ui, ctx, workspace);
        }
    });
}

fn show_right_column(
    ui: &mut egui::Ui,
    workspace: &mut TransferWorkspace,
    edits: &mut EditBuffers,
    toasts: &mut Toasts,
) {
    let stack =
        layout::content_bottom_stack(ui.available_height(), cox_setup_height(workspace.ui.cox()));
    layout::show_card_stack(ui, stack, |ui, slot| match slot {
        layout::StackSlot::Top => {
            let geometry = workspace.ui.geometry_mut();
            panels::geometry::show(ui, &mut workspace.session, geometry, edits, toasts);
        }
        layout::StackSlot::Bottom => {
            let cox = workspace.ui.cox_mut();
            panels::cox::show(ui, &mut workspace.session, cox, edits, toasts);
        }
    });
}

fn cox_setup_height(cox: &CoxUi) -> f32 {
    if cox.estimate_value().is_some() {
        transfer_layout::COX_ESTIMATED_SETUP_HEIGHT
    } else {
        transfer_layout::COX_STACK_SETUP_HEIGHT
    }
}
