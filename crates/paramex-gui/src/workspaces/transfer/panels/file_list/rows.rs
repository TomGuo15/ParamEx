//! Transfer file-list row rendering; row actions become deferred `Cmd`s.

use eframe::egui;
use paramex_core::transfer::{FileListRow, Session};

use crate::format_ui::point_count_label;
use crate::ui_kit::{self, output_action_icon_button, BadgeTone, OutputActionIcon, StatusLineText};
use crate::workspaces::transfer::state::{FileRow, FileRows, PendingOutput};

use super::model::Cmd;

pub(super) fn render_rows(
    ui: &mut egui::Ui,
    session: &Session,
    file_rows: &FileRows,
    pending_outputs: &[PendingOutput],
    actions_enabled: bool,
    cmds: &mut Vec<Cmd>,
) {
    for row in file_rows.rows() {
        match row {
            FileRow::File { id } => {
                let Some(file_row) = session.file_list_row(id) else {
                    continue;
                };
                render_file_row(ui, &file_row, cmds);
                if let Some(output_name) = &file_row.output_name {
                    render_attached_output_row(
                        ui,
                        &file_row.file_id,
                        output_name,
                        actions_enabled,
                        cmds,
                    );
                }
            }
            FileRow::Error { id, name, message } => {
                render_error_row(ui, id, name, message, cmds);
            }
        }
    }
    for pending in pending_outputs {
        render_pending_output_row(
            ui,
            pending,
            actions_enabled,
            session.has_selected_file(),
            cmds,
        );
    }
}

fn render_file_row(ui: &mut egui::Ui, row: &FileListRow, cmds: &mut Vec<Cmd>) {
    // Row click target: render the row inside a frame, then `interact`
    // over the frame rect excluding the checkbox column so the rest of the row
    // selects the file while the checkbox keeps its own click handling.
    let row_id = ui.id().with(("file_row", row.file_id.as_str()));
    let hovered = ui
        .ctx()
        .read_response(row_id)
        .map(|r| r.hovered())
        .unwrap_or(false);
    let frame = ui_kit::selection_row_frame(ui, row.is_selected, hovered);
    let mut cb_rect = egui::Rect::NOTHING;
    let inner = frame.show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            let mut checked = row.is_checked;
            let cb = ui
                .push_id(("file_cb", row.file_id.as_str()), |ui| {
                    ui.checkbox(&mut checked, "")
                })
                .inner;
            let enabled = cb.enabled();
            let bulk_label = format!("Mark {} for bulk actions", row.name);
            cb.widget_info(|| {
                egui::WidgetInfo::selected(
                    egui::WidgetType::Checkbox,
                    enabled,
                    checked,
                    bulk_label.clone(),
                )
            });
            let cb = cb.on_hover_text(bulk_label);
            cb_rect = cb.rect;
            if cb.changed() {
                cmds.push(Cmd::SetChecked(row.file_id.clone(), checked));
            }
            render_compact_file_row_content(ui, row);
        });
    });

    let row_rect = inner.response.rect;
    if row.is_selected {
        ui_kit::selection_bar(ui, row_rect);
    }
    let select_rect = egui::Rect::from_min_max(
        egui::pos2(cb_rect.right().min(row_rect.right()), row_rect.top()),
        row_rect.max,
    );
    let response =
        ui_kit::selectable_row_response(ui, select_rect, row_id, &row.name, row.is_selected);
    if response.clicked() {
        cmds.push(Cmd::Select(row.file_id.clone()));
    }
}

fn render_compact_file_row_content(ui: &mut egui::Ui, row: &FileListRow) {
    let points = point_count_label(row.point_count);
    ui_kit::list_row_title_status(
        ui,
        &row.name,
        "ok",
        BadgeTone::Ok,
        StatusLineText::Inline(points.as_str()),
        |ui| {
            if row.manual_ranges {
                ui_kit::semantic_badge(ui, "manual", BadgeTone::Warning);
            }
        },
    );
}

fn render_attached_output_row(
    ui: &mut egui::Ui,
    file_id: &str,
    output_name: &str,
    actions_enabled: bool,
    cmds: &mut Vec<Cmd>,
) {
    let frame = ui_kit::selection_row_frame(ui, false, false);
    frame.show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            ui_kit::file_row_gutter(ui);
            ui.vertical(|ui| {
                ui_kit::list_row_title_status(
                    ui,
                    output_name,
                    "output",
                    BadgeTone::Ok,
                    StatusLineText::Inline("attached"),
                    |_| {},
                );
            });
            ui_kit::right_aligned(ui, |ui| {
                ui.add_enabled_ui(actions_enabled, |ui| {
                    if ui_kit::close_button(ui, "Remove attached output")
                        .on_hover_text("Remove this attached output")
                        .clicked()
                    {
                        cmds.push(Cmd::RemoveAttachedOutput(file_id.to_string()));
                    }
                    if output_action_icon_button(ui, "Detach output", OutputActionIcon::Detach)
                        .on_hover_text("Move this output to pending")
                        .clicked()
                    {
                        cmds.push(Cmd::DetachOutput(file_id.to_string()));
                    }
                });
            });
        });
    });
}

fn render_error_row(ui: &mut egui::Ui, id: &str, name: &str, message: &str, cmds: &mut Vec<Cmd>) {
    if ui_kit::file_error_row(ui, name, message) {
        cmds.push(Cmd::DismissError(id.to_string()));
    }
}

fn render_pending_output_row(
    ui: &mut egui::Ui,
    pending: &PendingOutput,
    actions_enabled: bool,
    can_attach: bool,
    cmds: &mut Vec<Cmd>,
) {
    let frame = ui_kit::selection_row_frame(ui, false, false);
    frame.show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            ui_kit::file_row_gutter(ui);
            ui.vertical(|ui| {
                ui_kit::list_row_title_status(
                    ui,
                    pending.name(),
                    "pending",
                    BadgeTone::Warning,
                    StatusLineText::Inline(pending.reason().label()),
                    |_| {},
                );
            });
            ui_kit::right_aligned(ui, |ui| {
                if ui
                    .add_enabled_ui(actions_enabled, |ui| {
                        ui_kit::close_button(ui, "Remove pending output")
                            .on_hover_text("Remove this pending output row")
                            .clicked()
                    })
                    .inner
                {
                    cmds.push(Cmd::RemovePendingOutput(pending.id().to_string()));
                }
                if ui
                    .add_enabled_ui(actions_enabled && can_attach, |ui| {
                        output_action_icon_button(
                            ui,
                            "Attach to Selected",
                            OutputActionIcon::Attach,
                        )
                        .on_hover_text("Attach to selected transfer file")
                        .clicked()
                    })
                    .inner
                {
                    cmds.push(Cmd::AttachPendingOutput(pending.id().to_string()));
                }
            });
        });
    });
}
