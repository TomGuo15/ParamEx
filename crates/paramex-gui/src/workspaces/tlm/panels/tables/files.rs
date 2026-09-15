//! Right-column TLM FILES status card.

use eframe::egui;
use egui_notify::Toasts;

use crate::format_ui::removed_items;
use crate::table_kit;
use crate::ui_kit;
use crate::workspaces::tlm::panels::columns::STATUS_COLS;
use crate::workspaces::tlm::TlmWorkspace;

use super::grid::{grid_table, GridSpec};

enum Cmd {
    Remove(String),
}

/// One file + status row per workbook, the always-visible replacement for the
/// removed Files tab. The header pill carries the failure count; a failed row's
/// parse message shows on status-cell hover. Each row carries a ✕ to drop that
/// workbook from the dataset.
pub fn show_files(ui: &mut egui::Ui, workspace: &mut TlmWorkspace, toasts: &mut Toasts) {
    let mut cmds = Vec::new();
    ui_kit::card_slot(ui, |ui| {
        let tlm = &workspace.state;
        let card = tlm.files_card();
        let pill = card.map(|card| {
            if card.error_count > 0 {
                format!("{} failed", card.error_count)
            } else {
                format!("{} ok", card.status_count)
            }
        });
        ui_kit::section_header(ui, "FILES", pill.as_deref());

        // Fixed schema columns fill the card without reacting to filename length.
        // Horizontal-only outer scroll; the table owns vertical scroll with a
        // sticky header.
        let empty_rows: &[Vec<String>] = &[];
        let rows = if card.is_some() {
            tlm.rows().status()
        } else {
            empty_rows
        };
        let generation = tlm.rows_generation();
        let remove_enabled = Some(workspace.io.is_idle());
        let clicked = table_kit::horizontal_table_scroll(ui, "tlm_files_scroll", |ui, card_w| {
            grid_table(
                ui,
                GridSpec {
                    id: "tlm_status_table",
                    cols: &STATUS_COLS,
                    rows,
                    card_w,
                    generation,
                    remove_enabled,
                },
                &mut workspace.grid_cache,
            )
        });
        // Row 0 of a status row is the `status.file` path the reducer matches on.
        if let Some(file) = clicked
            .and_then(|idx| rows.get(idx))
            .and_then(|r| r.first())
            .cloned()
        {
            cmds.push(Cmd::Remove(file));
        }
    });
    apply_commands(workspace, toasts, cmds);
}

fn apply_commands(workspace: &mut TlmWorkspace, toasts: &mut Toasts, cmds: Vec<Cmd>) {
    for cmd in cmds {
        match cmd {
            Cmd::Remove(file) => {
                let removed = workspace.state.remove_file(&file);
                if removed > 0 {
                    toasts.info(removed_items(removed, "file"));
                }
            }
        }
    }
}
