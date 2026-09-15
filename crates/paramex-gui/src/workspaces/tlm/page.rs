//! TLM workspace page composition.

use eframe::egui;
use egui_notify::Toasts;

use super::panels;
use crate::layout::{self, ShellRects};
use crate::state::EditBuffers;
use crate::workspaces::tlm::layout as tlm_layout;
use crate::workspaces::tlm::TlmWorkspace;

pub fn show(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    shell: &ShellRects,
    workspace: &mut TlmWorkspace,
    edits: &mut EditBuffers,
    toasts: &mut Toasts,
) {
    layout::show_in_rect(ui, "tlm_left_rect", shell.left, |ui| {
        show_left_column(ui, ctx, workspace, edits, toasts);
    });
    layout::show_in_rect(ui, "tlm_center_rect", shell.center, |ui| {
        show_center_column(ui, ctx, workspace);
    });
    layout::show_in_rect(ui, "tlm_right_rect", shell.right, |ui| {
        show_right_column(ui, workspace, toasts);
    });
}

fn show_left_column(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    workspace: &mut TlmWorkspace,
    edits: &mut EditBuffers,
    toasts: &mut Toasts,
) {
    layout::prepare_column(ui);
    show_inputs(ui, ctx, workspace, edits, toasts);
    ui.add_space(layout::CARD_GAP);
    let groups_h = ui.available_height().max(tlm_layout::TLM_GROUPS_MIN_HEIGHT);
    ui.allocate_ui(egui::vec2(ui.available_width(), groups_h), |ui| {
        panels::groups::show(ui, &mut workspace.state);
    });
}

fn show_inputs(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    workspace: &mut TlmWorkspace,
    edits: &mut EditBuffers,
    toasts: &mut Toasts,
) {
    let slot_h =
        layout::fixed_bottom_stack(ui.available_height(), layout::SELECTED_METRICS_HEIGHT).top;
    let (data_h, analysis_h) = input_heights(slot_h);
    ui.allocate_ui(egui::vec2(ui.available_width(), data_h), |ui| {
        panels::data::show(ui, ctx, workspace, edits, toasts);
    });
    ui.add_space(layout::CARD_GAP);
    ui.allocate_ui(egui::vec2(ui.available_width(), analysis_h), |ui| {
        panels::analysis::show(ui, &mut workspace.state, edits, toasts);
    });
}

fn input_heights(slot_h: f32) -> (f32, f32) {
    let analysis_h = tlm_layout::TLM_ANALYSIS_HEIGHT;
    let data_h =
        (slot_h - layout::CARD_GAP - analysis_h).clamp(0.0, tlm_layout::TLM_DATA_CARD_HEIGHT);
    (data_h, analysis_h)
}

fn show_center_column(ui: &mut egui::Ui, ctx: &egui::Context, workspace: &mut TlmWorkspace) {
    // Identical to the Transfer center: flex plot on top, fixed shared band below.
    let stack = layout::fixed_bottom_stack(ui.available_height(), layout::SELECTED_METRICS_HEIGHT);
    layout::show_card_stack(ui, stack, |ui, slot| match slot {
        layout::StackSlot::Top => {
            panels::plot::show(ui, &workspace.state);
        }
        layout::StackSlot::Bottom => {
            panels::tables::show_results(
                ui,
                ctx,
                &mut workspace.state,
                &mut workspace.grid_cache,
                &mut workspace.io,
            );
        }
    });
}

fn show_right_column(ui: &mut egui::Ui, workspace: &mut TlmWorkspace, toasts: &mut Toasts) {
    let stack = layout::fixed_bottom_stack(ui.available_height(), layout::SELECTED_METRICS_HEIGHT);
    layout::show_card_stack(ui, stack, |ui, slot| match slot {
        layout::StackSlot::Top => {
            panels::tables::show_files(ui, workspace, toasts);
        }
        layout::StackSlot::Bottom => {
            panels::metrics::show(ui, &workspace.state);
        }
    });
}
