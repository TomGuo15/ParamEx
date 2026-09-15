//! Right-column GEOMETRY card: a per-file W/L table and a global W·L apply.
//! Committed state is `Session`; per-file edit text lives in `EditBuffers`
//! (commit on focus-loss).

use eframe::egui;
use egui_notify::Toasts;
use paramex_core::transfer::Session;

mod model;
mod rows;

use crate::state::EditBuffers;
use crate::ui_kit::{self, Variant};
use crate::workspaces::transfer::state::GeometryUi;

use model::{apply_commands, Cmd};

pub use model::commit_row_geometry;

pub use show as show_setup;

pub fn show(
    ui: &mut egui::Ui,
    session: &mut Session,
    geometry: &mut GeometryUi,
    edits: &mut EditBuffers,
    toasts: &mut Toasts,
) {
    let mut cmds: Vec<Cmd> = Vec::new();
    ui_kit::card_slot(ui, |ui| {
        ui_kit::section_header(ui, "GEOMETRY", None);
        render_global_wl_controls(ui, session, geometry, &mut cmds);
        ui.add_space(8.0);
        rows::render_geometry_rows_section(ui, session, edits, &mut cmds);
    });
    apply_commands(session, edits, toasts, cmds);
}

fn render_global_wl_controls(
    ui: &mut egui::Ui,
    session: &Session,
    geometry: &mut GeometryUi,
    cmds: &mut Vec<Cmd>,
) {
    let (global_w, global_l) = geometry.global_wl_mut();
    let row_w = ui.available_width();
    ui_kit::inline_paired_settings_row_sized(
        ui,
        row_w,
        "W (\u{00B5}m)",
        global_w,
        "L (\u{00B5}m)",
        global_l,
    );
    ui.add_space(8.0);
    let apply_clicked = ui
        .add_enabled_ui(session.has_files(), |ui| {
            ui_kit::button_full(ui, "Apply W/L to All Files", Variant::Secondary)
        })
        .inner
        .clicked();
    if apply_clicked {
        // Non-numeric text and non-positive numbers report different messages,
        // like the per-row editor below; these are free-text fields.
        match geometry.parse_global_wl() {
            Some((width, length)) => cmds.push(Cmd::ApplyGlobalWl { width, length }),
            None => cmds.push(Cmd::RejectNonNumeric),
        }
    }
}
