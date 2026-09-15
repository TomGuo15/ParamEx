use eframe::egui;
use egui_extras::{Column, TableBuilder};
use paramex_core::transfer::{OutputFitKind, OutputFitStatus, OutputReportRow, Session};

use super::span;
use crate::format_ui::{fmt_eng, fmt_r2, fmt_vg};
use crate::table_kit::{
    aligned_cell, body_label, fill_card_width, galley_width, header_rule, hover_if_clipped,
    hover_if_clipped_at, muted_header_font, muted_header_label, pad_and_clamp, ROW_H,
};

// Static floors keep the empty/loaded grid fixed; identity columns get the
// width budget so paired transfer/output filenames remain distinguishable.
const OUTPUT_COLUMNS: [OutputColumn; 10] = [
    OutputColumn::new("Device", false, 112.0),
    OutputColumn::new("Output file", false, 155.0),
    OutputColumn::new("Fit", false, 46.0),
    OutputColumn::new("V<sub>G</sub> (V)", true, 46.0),
    OutputColumn::new("I<sub>D,sat</sub> (A)", true, 58.0),
    OutputColumn::new("g<sub>ds</sub> (S)", true, 46.0),
    OutputColumn::new("r<sub>o</sub> (Ω)", true, 46.0),
    OutputColumn::new("V<sub>D0</sub> / V<sub>A</sub> (V)", true, 72.0),
    OutputColumn::new("λ (V<sup>-1</sup>)", true, 50.0),
    OutputColumn::new("R<sup>2</sup>", true, 28.0),
];

struct OutputColumn {
    label: &'static str,
    right: bool,
    min_width: f32,
}

impl OutputColumn {
    const fn new(label: &'static str, right: bool, min_width: f32) -> Self {
        Self {
            label,
            right,
            min_width,
        }
    }
}

pub(super) struct OutputDisplayRow {
    cells: Vec<String>,
    group_span: isize,
}

#[derive(Default)]
pub(crate) struct OutputResultsTableCache {
    generation: Option<u64>,
    pixels_per_point: f32,
    rows: Vec<OutputDisplayRow>,
    widths: Vec<f32>,
}

impl OutputResultsTableCache {
    pub(super) fn ensure(&mut self, ui: &egui::Ui, session: &Session) {
        let ppp = ui.ctx().pixels_per_point();
        if self.generation == Some(session.generation()) && self.pixels_per_point == ppp {
            return;
        }
        self.pixels_per_point = ppp;
        let rows = display_rows(session);
        self.widths = measure_col_widths(ui, &rows);
        self.rows = rows;
        self.generation = Some(session.generation());
    }

    pub(super) fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub(super) fn rows(&self) -> &[OutputDisplayRow] {
        &self.rows
    }

    pub(super) fn widths(&self) -> &[f32] {
        &self.widths
    }
}

fn display_rows(session: &Session) -> Vec<OutputDisplayRow> {
    let report_rows = session.output_report_rows();
    let mut rows = Vec::with_capacity(report_rows.len());
    let mut start = 0;
    while start < report_rows.len() {
        debug_assert_eq!(report_rows[start].fit, OutputFitKind::Family);
        let end = report_rows[start + 1..]
            .iter()
            .position(|row| row.fit == OutputFitKind::Family)
            .map_or(report_rows.len(), |offset| start + 1 + offset);
        let span = (end - start) as isize;
        for (offset, row) in report_rows[start..end].iter().enumerate() {
            rows.push(report_display_row(
                row,
                if offset == 0 { span } else { -span },
            ));
        }
        start = end;
    }
    rows
}

fn report_display_row(row: &OutputReportRow, group_span: isize) -> OutputDisplayRow {
    let fit = match row.status {
        OutputFitStatus::Ok => row.fit.label(),
        OutputFitStatus::Partial => "Partial",
        OutputFitStatus::Unavailable => "Failed",
    };
    OutputDisplayRow {
        cells: vec![
            row.device.clone(),
            row.output_file.clone(),
            fit.to_owned(),
            fmt_vg(row.vg),
            fmt_eng(row.idsat),
            fmt_eng(row.gds),
            fmt_eng(row.ro),
            fmt_vg(row.early_voltage),
            fmt_eng(row.lambda),
            fmt_r2(row.r2),
        ],
        group_span,
    }
}

fn measure_col_widths(ui: &egui::Ui, _rows: &[OutputDisplayRow]) -> Vec<f32> {
    let header_font = muted_header_font();
    OUTPUT_COLUMNS
        .iter()
        .map(|col| pad_and_clamp(galley_width(ui, col.label, &header_font), col.min_width))
        .collect()
}

pub(super) fn render_output_results_table_body(
    ui: &mut egui::Ui,
    rows: &[OutputDisplayRow],
    measured: &[f32],
    card_w: f32,
) {
    let fitted = fit_widths(ui, measured, card_w);
    ui.set_min_width(fitted.painted_w);
    let body_h = (ui.available_height() - ROW_H - ui.spacing().item_spacing.y).max(0.0);
    let paint_clip = span::table_body_clip(ui, body_h);
    let mut builder = TableBuilder::new(ui)
        .striped(false)
        .vscroll(true)
        .min_scrolled_height(0.0)
        .max_scroll_height(body_h)
        .auto_shrink([false, false])
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center));
    for width in &fitted.widths {
        builder = builder.column(Column::initial(*width).at_least(*width).clip(true));
    }
    let table = builder
        .header(ROW_H, |mut header| {
            for col in &OUTPUT_COLUMNS {
                header.col(|ui| {
                    header_rule(ui);
                    let resp = aligned_cell(ui, col.right, |ui| muted_header_label(ui, col.label));
                    hover_if_clipped_at(ui, resp, col.label, &muted_header_font());
                });
            }
        })
        .body(|mut body| {
            for (row_idx, row_cells) in rows.iter().enumerate() {
                body.row(ROW_H, |mut row| {
                    for (idx, col) in OUTPUT_COLUMNS.iter().enumerate() {
                        row.col(|ui| {
                            if idx == 0 && row_idx > 0 && row_cells.group_span > 0 {
                                span::group_separator(ui, fitted.painted_w);
                            }
                            if idx < 2 {
                                merged_output_group_cell(
                                    ui, rows, row_idx, idx, col.right, paint_clip,
                                );
                                return;
                            }
                            let text = row_cells.cells.get(idx).map_or("", String::as_str);
                            aligned_cell(ui, col.right, |ui| {
                                let resp = body_label(ui, text);
                                hover_if_clipped(ui, resp, text);
                            });
                        });
                    }
                });
            }
        });
    if rows.is_empty() {
        super::results_empty_notice(ui, table.inner_rect, "No output fit rows.");
    }
}

fn merged_output_group_cell(
    ui: &mut egui::Ui,
    rows: &[OutputDisplayRow],
    row_idx: usize,
    col_idx: usize,
    right: bool,
    paint_clip: egui::Rect,
) {
    let Some(leader_idx) = (0..=row_idx).rev().find(|&j| rows[j].group_span > 0) else {
        return;
    };
    let n = rows[leader_idx].group_span as usize;
    if row_idx + 1 != leader_idx + n {
        return;
    }
    let text = &rows[leader_idx].cells[col_idx];
    if text.is_empty() {
        return;
    }

    span::paint_merged_group_cell(ui, text, n, right, ROW_H, paint_clip);
}

struct FittedWidths {
    widths: Vec<f32>,
    painted_w: f32,
}

fn fit_widths(ui: &egui::Ui, measured: &[f32], card_w: f32) -> FittedWidths {
    let mut widths = measured.to_vec();
    let gaps = OUTPUT_COLUMNS.len().saturating_sub(1) as f32 * ui.spacing().item_spacing.x;
    fill_card_width(&mut widths, (card_w - gaps).max(0.0));
    let min_w = OUTPUT_COLUMNS.iter().map(|col| col.min_width).sum::<f32>();
    let table_w = widths.iter().sum::<f32>().max(min_w);
    FittedWidths {
        widths,
        painted_w: table_w + gaps,
    }
}
