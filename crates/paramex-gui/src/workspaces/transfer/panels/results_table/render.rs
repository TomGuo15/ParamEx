//! Results-table body rendering.
//!
//! The parent panel owns cache invalidation, export, and empty-state flow. This
//! module owns the table body pipeline and delegates measurement, group spans,
//! and Overall stacked cells to private sibling modules.

use eframe::egui;
use egui_extras::{Column, TableBuilder};

use super::{columns, measure, model::DisplayRow, overall, span};
use crate::table_kit::{
    aligned_cell, body_label, header_rule, hover_if_clipped, hover_if_clipped_at,
    muted_header_font, muted_header_label, ROW_H,
};
use crate::theme::{token_alpha, tokens};

pub(super) fn render_results_table_body(
    ui: &mut egui::Ui,
    rows: &[DisplayRow],
    measured: &[f32],
    card_w: f32,
) {
    let indexed_specs = columns::indexed_gui_column_specs();
    let min_widths: Vec<f32> = indexed_specs.iter().map(|spec| spec.min_width).collect();
    let fitted = measure::fit_table_widths(ui, measured, &min_widths, card_w);
    let sweep_cell_idx = indexed_specs
        .iter()
        .find(|spec| spec.column == paramex_core::transfer::ResultsTableColumn::Sweep)
        .map(|spec| spec.column.index());
    ui.set_min_width(fitted.painted_w);
    // The TABLE owns vertical scroll (sticky header — the geometry-table recipe);
    // the outer ScrollArea is horizontal-only, so header and body scroll sideways
    // together while the header stays pinned vertically.
    let body_h = (ui.available_height() - ROW_H - ui.spacing().item_spacing.y).max(0.0);
    let paint_clip = span::table_body_clip(ui, body_h);
    let mut builder = TableBuilder::new(ui)
        .striped(false)
        .vscroll(true)
        .min_scrolled_height(0.0)
        .max_scroll_height(body_h)
        .auto_shrink([false, false])
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center));
    for width in &fitted.column_widths {
        builder = builder.column(Column::initial(*width).at_least(*width).clip(true));
    }
    let table = builder
        .header(ROW_H, |mut header| {
            for spec in indexed_specs {
                header.col(|ui| {
                    header_rule(ui);
                    let header_text = columns::gui_header_label_html(spec);
                    let resp = aligned_cell(ui, columns::col_right_aligned(spec), |ui| {
                        muted_header_label(ui, header_text)
                    });
                    hover_if_clipped_at(ui, resp, header_text, &muted_header_font());
                });
            }
        })
        .body(|mut body| {
            // Per-row heights: Overall rows are taller because they stack mean over ±std.
            for (row_idx, display) in rows.iter().enumerate() {
                let row_h = if display.is_overall {
                    overall::OVERALL_ROW_H
                } else {
                    ROW_H
                };
                body.row(row_h, |mut row| {
                    for (i, spec) in indexed_specs.iter().enumerate() {
                        row.col(|ui| {
                            // A hairline above each new group (`group_span > 0` marks the
                            // leader row) — the quiet replacement for the cell boxes.
                            // Painted during THIS row's closure, after the row's own
                            // stripe, so nothing overdraws it.
                            if i == 0 && row_idx > 0 && display.group_span > 0 {
                                span::group_separator(ui, fitted.painted_w);
                            }
                            let right = columns::col_right_aligned(spec);
                            let col_idx = spec.column.index();
                            if !spec.column.is_sweep_aware() {
                                span::merged_group_cell(
                                    ui, rows, row_idx, col_idx, right, row_h, paint_clip,
                                );
                                return;
                            }
                            if is_backward_row(display, sweep_cell_idx) {
                                paint_backward_cell(ui);
                            }
                            let text = &display.cells[col_idx];
                            if display.is_overall {
                                // Per-sweep metric on the Overall row: stack the mean over a
                                // small muted "± std" (or "N=k") line so the column stays as
                                // narrow as the per-file value (user design).
                                overall::paint_stacked(ui, text, right);
                            } else {
                                let resp = aligned_cell(ui, right, |ui| body_label(ui, text));
                                hover_if_clipped(ui, resp, text);
                            }
                        });
                    }
                });
            }
        });
    if rows.is_empty() {
        super::results_empty_notice(ui, table.inner_rect, "No transfer fit rows.");
    }
}

fn is_backward_row(display: &DisplayRow, sweep_idx: Option<usize>) -> bool {
    sweep_idx
        .and_then(|idx| display.cells.get(idx))
        .is_some_and(|text| text == "B" || text.starts_with("B "))
}

fn paint_backward_cell(ui: &egui::Ui) {
    let cell = ui.max_rect();
    let half_gap = ui.spacing().item_spacing.x / 2.0;
    let rect = egui::Rect::from_min_max(
        egui::pos2(cell.left() - half_gap, cell.top()),
        egui::pos2(cell.right() + half_gap, cell.bottom()),
    );
    ui.painter()
        .with_clip_rect(ui.clip_rect().expand2(egui::vec2(half_gap, 0.0)))
        .rect_filled(rect, 0.0, token_alpha(tokens().bg, 120));
}
