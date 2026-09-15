//! Per-file geometry row rendering.

use eframe::egui;
use egui_extras::Column;
use paramex_core::transfer::{GeometrySource, Session};

use crate::state::EditBuffers;
use crate::table_kit;
use crate::ui_kit;
use crate::workspaces::numeric_edit::numeric_edit_commit;

use super::model::Cmd;

const GEOMETRY_COL_GALLEYS: [f32; 3] = [32.0, 38.0, 38.0];
const GEOMETRY_COL_FLOORS: [f32; 3] = [96.0, 64.0, 64.0];

pub(super) fn render_geometry_rows_section(
    ui: &mut egui::Ui,
    session: &Session,
    edits: &mut EditBuffers,
    cmds: &mut Vec<Cmd>,
) {
    let table_h = ui.available_height().max(0.0);
    render_file_table(ui, session, edits, cmds, table_h);
}

/// The per-file W/L grid. Snapshots display rows first and collects edits as
/// deferred commands; the card applies them through `Session` after render.
fn render_file_table(
    ui: &mut egui::Ui,
    session: &Session,
    edits: &mut EditBuffers,
    cmds: &mut Vec<Cmd>,
    table_h: f32,
) {
    let rows = session.file_geometry_rows();

    let widths = table_kit::fit_fill_widths(
        &GEOMETRY_COL_GALLEYS,
        &GEOMETRY_COL_FLOORS,
        ui.available_width(),
        ui.spacing().item_spacing.x,
    );
    table_kit::quiet_table_builder(ui, table_h - table_kit::ROW_H)
        .column(Column::exact(widths[0]).at_least(widths[0]).clip(true))
        .column(Column::exact(widths[1]).at_least(widths[1]))
        .column(Column::exact(widths[2]).at_least(widths[2]))
        .header(table_kit::ROW_H, |mut header| {
            for label in ["File", "W (\u{00B5}m)", "L (\u{00B5}m)"] {
                header.col(|ui| {
                    table_kit::aligned_cell(ui, false, |ui| {
                        table_kit::muted_header_label(ui, label)
                    });
                    table_kit::header_rule(ui);
                });
            }
        })
        .body(|body| {
            body.rows(table_kit::ROW_H, rows.len(), |mut row| {
                let geometry = &rows[row.index()];
                row.col(|ui| {
                    table_kit::aligned_cell(ui, false, |ui| {
                        if geometry.source == GeometrySource::Manual {
                            ui_kit::semantic_badge(ui, "manual", ui_kit::BadgeTone::Warning);
                        }
                        let resp = table_kit::body_label(ui, &geometry.name);
                        table_kit::hover_if_clipped(ui, resp, &geometry.name);
                    });
                });
                row.col(|ui| {
                    table_kit::aligned_cell(ui, false, |ui| {
                        edit_dim(
                            ui,
                            edits,
                            &format!("geom:{}:w", geometry.file_id),
                            geometry.width_um,
                            |value| Cmd::RowGeometry {
                                file_id: geometry.file_id.clone(),
                                width: Some(value),
                                length: None,
                            },
                            cmds,
                        );
                    });
                });
                row.col(|ui| {
                    table_kit::aligned_cell(ui, false, |ui| {
                        edit_dim(
                            ui,
                            edits,
                            &format!("geom:{}:l", geometry.file_id),
                            geometry.length_um,
                            |value| Cmd::RowGeometry {
                                file_id: geometry.file_id.clone(),
                                width: None,
                                length: Some(value),
                            },
                            cmds,
                        );
                    });
                });
            });
        });
}

/// One focus-tracked W or L input. A parsed commit becomes the row command;
/// non-numeric text is reported once through `Cmd::RejectNonNumeric`.
fn edit_dim(
    ui: &mut egui::Ui,
    edits: &mut EditBuffers,
    key: &str,
    current: f64,
    to_cmd: impl FnOnce(f64) -> Cmd,
    cmds: &mut Vec<Cmd>,
) {
    let current_str = format!("{current}");
    let width = ui.available_width();
    match numeric_edit_commit(ui, edits, key, &current_str, width) {
        Some(Ok(value)) => cmds.push(to_cmd(value)),
        Some(Err(_)) => cmds.push(Cmd::RejectNonNumeric),
        None => {}
    }
}
