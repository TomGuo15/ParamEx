//! Results-table merged group spans and separators.

use eframe::egui;

use crate::table_kit::{body_label, group_separator_stroke, hover_if_clipped, CELL_INSET, ROW_H};

/// A display row that may lead or continue a merged group. `group_span` is the
/// group length on the leader row and its negation on every follower row.
pub(super) trait GroupRow {
    fn group_span(&self) -> i64;
    fn cell(&self, col_idx: usize) -> &str;
}

pub(super) fn table_body_clip(ui: &egui::Ui, body_h: f32) -> egui::Rect {
    let top = ui.cursor().top() + ROW_H + ui.spacing().item_spacing.y;
    egui::Rect::from_min_max(
        egui::pos2(ui.clip_rect().left(), top),
        egui::pos2(ui.clip_rect().right(), top + body_h),
    )
    .intersect(ui.clip_rect())
}

/// One merged label for a group column, centered across the whole span. Painted
/// from the group's LAST row, once every row of the span has been laid out.
pub(super) fn merged_group_cell<R: GroupRow>(
    ui: &mut egui::Ui,
    rows: &[R],
    row_idx: usize,
    col_idx: usize,
    right: bool,
    row_h: f32,
    paint_clip: egui::Rect,
) {
    let Some(leader_idx) = (0..=row_idx).rev().find(|&j| rows[j].group_span() > 0) else {
        return;
    };
    let n = rows[leader_idx].group_span() as usize;
    if row_idx + 1 != leader_idx + n {
        return;
    }
    let text = rows[leader_idx].cell(col_idx);
    if text.is_empty() {
        return;
    }

    paint_merged_group_cell(ui, text, n, right, row_h, paint_clip);
}

pub(super) fn paint_merged_group_cell(
    ui: &mut egui::Ui,
    text: &str,
    n: usize,
    right: bool,
    row_h: f32,
    paint_clip: egui::Rect,
) {
    let gap = ui.spacing().item_spacing.y;
    let cell = ui.max_rect();
    let nf = n as f32;
    let block = egui::Rect::from_min_size(
        egui::pos2(cell.left(), cell.top() - (nf - 1.0) * (row_h + gap)),
        egui::vec2(cell.width(), nf * row_h + (nf - 1.0) * gap),
    );
    let clip = block.intersect(paint_clip);
    if !clip.is_positive() {
        return;
    }

    let layout = if right {
        egui::Layout::right_to_left(egui::Align::Center)
    } else {
        egui::Layout::left_to_right(egui::Align::Center)
    };
    ui.scope_builder(
        egui::UiBuilder::new().max_rect(block).layout(layout),
        |ui| {
            ui.set_clip_rect(clip);
            ui.add_space(CELL_INSET);
            let resp = body_label(ui, text);
            hover_if_clipped(ui, resp, text);
        },
    );
}

/// 1px group-boundary hairline across the whole painted table width (columns +
/// inter-column gaps), sitting in the inter-row gap above the group's leader
/// row. Extends half a gap per side, matching the header rule and stripes'
/// gapless expansion.
pub(super) fn group_separator(ui: &egui::Ui, painted_w: f32) {
    let cell = ui.max_rect();
    let half_gap = ui.spacing().item_spacing.x / 2.0;
    let y = cell.top() - ui.spacing().item_spacing.y / 2.0;
    ui.painter().hline(
        egui::Rangef::new(cell.left() - half_gap, cell.left() + painted_w + half_gap),
        y,
        group_separator_stroke(),
    );
}
