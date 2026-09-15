//! GROUPS card: a selectable list of groups (the Transfer file-list role).
//! Rows use the shared `selection_row` idiom; clicking re-selects the group
//! (re-select only — no recompute, the analysis already covers every group).

use eframe::egui;
use paramex_core::tlm::GroupAnalysis;

use crate::format_ui::point_count_label;
use crate::ui_kit::{self, BadgeTone, StatusLineText};
use crate::workspaces::tlm::state::TlmState;

enum Cmd {
    Select(String),
}

pub fn show(ui: &mut egui::Ui, tlm: &mut TlmState) {
    let mut cmds = Vec::new();
    ui_kit::card_slot(ui, |ui| {
        ui_kit::section_header(ui, "GROUPS", None);
        let rows_h = ui.available_height().max(0.0);
        if let Some(groups) = tlm.group_list() {
            if let Some(name) = render_rows(ui, groups.groups, groups.selected, rows_h) {
                cmds.push(Cmd::Select(name));
            }
        } else if let Some(name) = render_rows(ui, &[], None, rows_h) {
            cmds.push(Cmd::Select(name));
        }
    });
    for cmd in cmds {
        match cmd {
            Cmd::Select(name) => {
                tlm.select_group(&name);
            }
        }
    }
}

fn render_rows(
    ui: &mut egui::Ui,
    groups: &[GroupAnalysis],
    selected_group: Option<&str>,
    rows_h: f32,
) -> Option<String> {
    let mut select: Option<String> = None;
    ui_kit::scroll_body(ui, "tlm_group_rows", rows_h, |ui| {
        for g in groups {
            let is_selected = selected_group == Some(g.group.as_str());
            let row_id = ui.id().with(("tlm_group_row", g.group.as_str()));
            let hovered = ui
                .ctx()
                .read_response(row_id)
                .map(|r| r.hovered())
                .unwrap_or(false);
            let frame = ui_kit::selection_row_frame(ui, is_selected, hovered);
            let inner = frame.show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    let points = point_count_label(g.points.len());
                    let (badge, tone) = if g.warnings.is_empty() {
                        ("ok", BadgeTone::Ok)
                    } else {
                        ("warn", BadgeTone::Warning)
                    };
                    ui_kit::list_row_title_status(
                        ui,
                        &g.group,
                        badge,
                        tone,
                        StatusLineText::Inline(points.as_str()),
                        |_| {},
                    );
                });
            });
            if is_selected {
                ui_kit::selection_bar(ui, inner.response.rect);
            }
            let row = ui_kit::selectable_row_response(
                ui,
                inner.response.rect,
                row_id,
                &g.group,
                is_selected,
            );
            if row.clicked() {
                select = Some(g.group.clone());
            }
        }
    });
    select
}
