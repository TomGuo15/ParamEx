//! TLM grid-table rendering and measurement cache.
//!
//! `tables.rs` owns the Results/Files card flow. This module owns the repeated
//! analytical table renderer used by all four TLM row sets.

use std::collections::HashMap;

use eframe::egui;
use egui_extras::{Column, TableBuilder};

use crate::table_kit::{self, ROW_H};
use crate::ui_kit::{self, BadgeTone};
use crate::workspaces::tlm::panels::columns::{CellKind, TlmCol};

/// One TLM table's cached column measurements. Widths depend on both the
/// projected rows and the live card width.
#[derive(Clone)]
struct GridMeasure {
    generation: u64,
    card_w: f32,
    widths: Vec<f32>,
}

impl GridMeasure {
    fn valid_for(&self, generation: u64, card_w: f32) -> bool {
        self.generation == generation && self.card_w == card_w
    }
}

/// Workspace-owned cache for measured grid widths, keyed by table id and
/// invalidated by the rows generation plus the card width.
#[derive(Default)]
pub(crate) struct TlmGridCache {
    entries: HashMap<String, GridMeasure>,
}

impl TlmGridCache {
    pub(crate) fn get(&self, id: &str, generation: u64, card_w: f32) -> Option<&[f32]> {
        self.entries
            .get(id)
            .filter(|measure| measure.valid_for(generation, card_w))
            .map(|measure| measure.widths.as_slice())
    }

    pub(crate) fn insert(&mut self, id: &str, generation: u64, card_w: f32, widths: Vec<f32>) {
        self.entries.insert(
            id.to_owned(),
            GridMeasure {
                generation,
                card_w,
                widths,
            },
        );
    }
}

/// One TLM cell's content (no alignment: the caller wraps it in the column's
/// layout): plain rich text, a semantic ok/error badge, or yellow warnings.
fn cell_content(ui: &mut egui::Ui, col: &TlmCol, row: &[String], ncols: usize, text: &str) {
    match col.kind {
        CellKind::Status => {
            // Engine contract: tlm::Status::as_str() is exactly "ok" | "error"
            // (core/src/tlm/types.rs). Anything not-"ok" renders red.
            let tone = if text == "ok" {
                BadgeTone::Ok
            } else {
                BadgeTone::Error
            };
            let resp = ui_kit::semantic_badge(ui, text, tone);
            // Trailing cells beyond the column count are a hover payload. The FILES
            // table carries the parse message there: column-less, but one hover away.
            if let Some(extra) = row.get(ncols) {
                if !extra.is_empty() {
                    resp.on_hover_text(extra);
                }
            }
        }
        CellKind::Warnings => {
            if !text.is_empty() {
                let resp = ui_kit::semantic_badge(ui, "warn", BadgeTone::Warning);
                let enabled = resp.enabled();
                let accessible = format!("Warning: {text}");
                resp.widget_info(move || {
                    egui::WidgetInfo::labeled(egui::WidgetType::Label, enabled, accessible.clone())
                });
                resp.on_hover_text(text);
            }
        }
        CellKind::Text => {
            let resp = table_kit::body_label(ui, text);
            table_kit::hover_if_clipped(ui, resp, text);
        }
    }
}

/// Width of the optional trailing ✕ (remove) column.
const ACTION_COL_W: f32 = 28.0;

/// Inputs that identify one TLM grid and its current rows.
pub(super) struct GridSpec<'a> {
    pub id: &'a str,
    pub cols: &'a [TlmCol],
    pub rows: &'a [Vec<String>],
    pub card_w: f32,
    pub generation: u64,
    pub remove_enabled: Option<bool>,
}

/// Quiet analytical table over pre-formatted string rows (transfer results-table
/// style): content-measured columns, boxless striped rows, muted ruled header,
/// right-aligned numeric columns, status/warning colors, sticky header.
/// `generation` identifies the rows' contents and keys the [`TlmGridCache`] entry.
pub(super) fn grid_table(
    ui: &mut egui::Ui,
    spec: GridSpec<'_>,
    cache: &mut TlmGridCache,
) -> Option<usize> {
    let GridSpec {
        id,
        cols,
        rows,
        card_w,
        generation,
        remove_enabled,
    } = spec;
    let removable = remove_enabled.is_some();
    let spacing = ui.spacing().item_spacing.x;
    // Reserve the trailing ✕ column out of the fit width so the text columns still
    // span the card exactly (no h-scrollbar when they fit).
    let fit_w = if removable {
        (card_w - ACTION_COL_W - spacing).max(0.0)
    } else {
        card_w
    };
    let gaps = (cols.len().saturating_sub(1)) as f32 * spacing;
    let widths = if removable {
        // FILES is a fixed summary card. Reserve its schema columns from the
        // declared widths so filenames cannot move the status/action columns.
        let mut widths: Vec<f32> = cols.iter().map(|col| col.min_w).collect();
        table_kit::fill_card_width(&mut widths, (fit_w - gaps).max(0.0));
        widths
    } else if let Some(cached) = cache.get(id, generation, card_w) {
        cached.to_vec()
    } else {
        let headers: Vec<&str> = cols.iter().map(|c| c.label).collect();
        let min_ws: Vec<f32> = cols.iter().map(|c| c.min_w).collect();
        // egui_extras TableBuilder inserts item_spacing.x between every pair of adjacent
        // columns, so the real table width is sum(widths) + (ncols-1)*spacing. Gap-aware
        // fill matches the Transfer results table: fill against card_w minus gaps so a
        // fitted table spans the card exactly with no horizontal scrollbar; genuinely
        // wider content keeps horizontal scroll.
        let mut widths = table_kit::measure_grid_col_widths(ui, &headers, &min_ws, rows);
        // Prose columns (warnings, file paths) yield width before the table overflows:
        // they clip-with-hover by design, so shrinking them to leftover space keeps the
        // whole table inside the card whenever the fixed columns fit.
        let yields: Vec<bool> = cols.iter().map(|c| c.yields).collect();
        table_kit::fit_yielding_widths(&mut widths, &min_ws, &yields, fit_w, spacing);
        cache.insert(id, generation, card_w, widths.clone());
        widths
    };
    let action_extra = if removable {
        ACTION_COL_W + spacing
    } else {
        0.0
    };
    ui.set_min_width(widths.iter().sum::<f32>() + gaps + action_extra);
    // The table owns vertical scroll (sticky header); the outer ScrollArea is
    // horizontal-only, so header and body scroll sideways together.
    let body_h = (ui.available_height() - ROW_H - ui.spacing().item_spacing.y).max(0.0);
    let mut remove: Option<usize> = None;
    ui.push_id(id, |ui| {
        let mut builder = TableBuilder::new(ui)
            .striped(true)
            .vscroll(true)
            .min_scrolled_height(0.0)
            .max_scroll_height(body_h)
            .auto_shrink([false, false])
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center));
        // clip(true): a cell whose content is wider than its column, like long error
        // messages or fit warnings, must truncate rather than bleed across neighbors.
        // The full text is on hover.
        for w in &widths {
            builder = builder.column(if removable {
                Column::exact(*w)
            } else {
                Column::initial(*w).at_least(*w).clip(true)
            });
        }
        if removable {
            builder = builder.column(Column::exact(ACTION_COL_W));
        }
        builder
            .header(ROW_H, |mut header| {
                for col in cols {
                    header.col(|ui| {
                        table_kit::header_rule(ui);
                        let resp = table_kit::aligned_cell(ui, col.right, |ui| {
                            table_kit::muted_header_label(ui, col.label)
                        });
                        table_kit::hover_if_clipped_at(
                            ui,
                            resp,
                            col.label,
                            &table_kit::muted_header_font(),
                        );
                    });
                }
                if removable {
                    header.col(|ui| {
                        table_kit::header_rule(ui);
                    });
                }
            })
            .body(|mut body| {
                for (ri, row) in rows.iter().enumerate() {
                    body.row(ROW_H, |mut tr| {
                        for (i, col) in cols.iter().enumerate() {
                            tr.col(|ui| {
                                let text = row.get(i).map(String::as_str).unwrap_or("");
                                table_kit::aligned_cell(ui, col.right, |ui| {
                                    cell_content(ui, col, row, cols.len(), text);
                                });
                            });
                        }
                        if let Some(enabled) = remove_enabled {
                            tr.col(|ui| {
                                if ui
                                    .add_enabled_ui(enabled, |ui| {
                                        ui_kit::close_button(ui, "Remove file")
                                    })
                                    .inner
                                    .clicked()
                                {
                                    remove = Some(ri);
                                }
                            });
                        }
                    });
                }
            });
    });
    remove
}
