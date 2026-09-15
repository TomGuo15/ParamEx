//! DATA card (left top): folder summary, persistent load-error row, the
//! EditBuffers fallback-V_D field, and the Load Folder / Clear All buttons.
//! Loading concerns ONLY — group/V_G selection live in `groups`/`analysis`.

use eframe::egui;
use egui_notify::Toasts;

use crate::format_ui::fmt_num3;
use crate::state::EditBuffers;
use crate::ui_kit::{self, Variant};
use crate::workspaces::tlm::{state::TlmState, TlmWorkspace};

mod model;

use model::{apply_commands, Cmd};

pub use model::{commit_fallback_vd, folder_summary};

const SUMMARY_BLOCK_HEIGHT: f32 = 36.0;

pub fn show(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    workspace: &mut TlmWorkspace,
    edits: &mut EditBuffers,
    toasts: &mut Toasts,
) {
    let mut cmds: Vec<Cmd> = Vec::new();
    ui_kit::card_slot(ui, |ui| {
        ui_kit::section_header(ui, "DATA", None);
        render_body(ui, &workspace.state, workspace.is_idle(), edits, &mut cmds);
    });
    apply_commands(ctx, workspace, toasts, cmds);
}

/// TLM Data card body: folder/counts, the persistent load-error row, the
/// EditBuffers fallback field, and the gated Load/Clear All buttons.
fn render_body(
    ui: &mut egui::Ui,
    tlm: &TlmState,
    idle: bool,
    edits: &mut EditBuffers,
    cmds: &mut Vec<Cmd>,
) {
    let data = tlm.data_card();
    if let Some(err) = data.load_error {
        render_summary_block(ui, |ui| {
            let summary = ui_kit::load_error_summary(err);
            if ui_kit::compact_error_notice(ui, summary, err) {
                cmds.push(Cmd::DismissLoadError);
            }
        });
    } else if let Some(folder) = data.folder {
        render_summary_block(ui, |ui| {
            let (base, counts) = folder_summary(folder.root, folder.workbooks, folder.groups);
            ui_kit::row_title_label(ui, base);
            ui_kit::field_label(ui, &counts);
        });
    } else {
        render_summary_block(ui, |ui| {
            ui_kit::muted_row_title_label(ui, "No folder loaded");
            ui_kit::field_label(ui, "0 workbooks \u{00B7} 0 groups");
        });
    }

    ui.add_space(6.0);
    let key = "tlm:fallback_vd";
    let committed = fmt_num3(data.fallback_vd);
    if let Some(text) =
        ui_kit::inline_settings_row_commit(ui, edits, key, "Fallback V<sub>D</sub> (V)", &committed)
    {
        match commit_fallback_vd(&text) {
            Ok(value) => cmds.push(Cmd::Fallback(value)),
            Err(_) => cmds.push(Cmd::FallbackInvalid),
        }
    }
    ui.add_space(8.0);

    let mut load = false;
    let mut clear = false;
    ui.columns(2, |cols| {
        load = cols[0]
            .add_enabled_ui(idle, |ui| {
                ui_kit::button_full(ui, "Load Folder", Variant::Primary)
            })
            .inner
            .clicked();
        clear = cols[1]
            .add_enabled_ui(idle && data.has_dataset, |ui| {
                ui_kit::button_full(ui, "Clear All", Variant::Danger)
            })
            .inner
            .clicked();
    });
    if load {
        cmds.push(Cmd::Load);
    }
    if clear {
        cmds.push(Cmd::Clear);
    }
}

fn render_summary_block(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), SUMMARY_BLOCK_HEIGHT),
        egui::Sense::hover(),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    add_contents(&mut child);
}
