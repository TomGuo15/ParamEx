//! Bottom-left selected-file metric tiles.

use eframe::egui;
use paramex_core::transfer::{ExtractionStatus, SelectedFileMetricsProjection, Session};

mod rows;

use crate::table_kit;
use crate::ui_kit::{self, BadgeTone};

use rows::{device_tiles, empty_device_tiles, empty_sweep_metric_rows, sweep_metric_rows};

/// Fractional row height: the device block plus the header and six sweep rows
/// must fill the fixed bottom band exactly, with no vertical scrollbar.
const SELECTED_METRIC_ROW_H: f32 = 20.3;
const DEVICE_METRIC_ROW_H: f32 = 20.0;
const DEVICE_METRIC_PAIR_GAP: f32 = 8.0;
const DEVICE_METRIC_VALUE_GAP: f32 = 4.0;
const DEVICE_METRIC_LEFT_LABEL_W: f32 = 42.0;
const DEVICE_METRIC_RIGHT_LABEL_W: f32 = 62.0;

/// Render the selected-file metrics (read-only over `Session`).
pub fn show(ui: &mut egui::Ui, session: &Session) {
    ui_kit::card_slot(ui, |ui| {
        let selected = session.selected_file_metrics_projection();
        let diagnostic = selected.and_then(|selected| {
            (selected.result.status != ExtractionStatus::Ok).then_some((
                "warn",
                BadgeTone::Warning,
                selected.result.message.as_str(),
            ))
        });
        ui_kit::header_action_row(ui, "SELECTED", |ui| {
            if let Some((badge, tone, detail)) = diagnostic {
                ui_kit::semantic_badge(ui, badge, tone).on_hover_text(detail);
            }
            if let Some(selected) = selected {
                ui_kit::truncated_row_title_label(ui, selected.filename)
                    .on_hover_text(selected.filename);
            }
        });
        let card_w = ui.available_width();
        render_metrics_body(ui, selected, card_w);
    });
}

fn render_metrics_body(
    ui: &mut egui::Ui,
    selected: Option<SelectedFileMetricsProjection<'_>>,
    card_w: f32,
) {
    let (device_rows, has_backward, rows) = match selected {
        None => (empty_device_tiles(), true, empty_sweep_metric_rows()),
        Some(selected) => {
            let (has_backward, rows) = sweep_metric_rows(selected.result);
            (device_tiles(selected.result), has_backward, rows)
        }
    };

    // Tight vertical budget: everything (Device block + 7-row sweep table) must
    // sit inside the fixed bottom band with NO vertical scrollbar; the
    // vertical-fit guard lives in tests/gui/transfer/selected_metrics.rs.
    ui.spacing_mut().item_spacing.y = 2.0;
    ui_kit::field_label(ui, "Device");
    render_device_tile_rows(ui, &device_rows, card_w);

    ui.add_space(2.0);
    ui_kit::field_label(ui, "Sweep metrics");
    // Full-card-width striped table (like the geometry W/L table) instead of a
    // shrink-wrapped Grid — the stripes and columns span the whole card.
    let headers = if has_backward {
        ["Metric", "Forward", "Backward"]
    } else {
        ["Metric", "Value", ""]
    };
    let row_cells: Vec<Vec<String>> = rows
        .into_iter()
        .map(|(label, forward, backward)| vec![label.to_string(), forward, backward])
        .collect();
    table_kit::striped_fill_table(
        ui,
        table_kit::FillTableSpec {
            id: "sweep_tiles",
            headers: &headers,
            base_ws: &[94.0, 66.0, 66.0],
            rows: &row_cells,
            card_w,
            row_h: SELECTED_METRIC_ROW_H,
        },
        ui_kit::metric_table_cell,
    );
}

fn render_device_tile_rows(ui: &mut egui::Ui, device_rows: &[(&'static str, String)], card_w: f32) {
    let row_w = card_w.max(0.0);
    let cell_w = ((row_w - DEVICE_METRIC_PAIR_GAP).max(0.0) / 2.0).floor();
    for pair in device_rows.chunks(2) {
        let (row_rect, _) =
            ui.allocate_exact_size(egui::vec2(row_w, DEVICE_METRIC_ROW_H), egui::Sense::hover());
        let left_rect =
            egui::Rect::from_min_size(row_rect.min, egui::vec2(cell_w, DEVICE_METRIC_ROW_H));
        let right_rect = egui::Rect::from_min_size(
            egui::pos2(left_rect.right() + DEVICE_METRIC_PAIR_GAP, row_rect.top()),
            egui::vec2(cell_w, DEVICE_METRIC_ROW_H),
        );
        if let Some((label, value)) = pair.first() {
            render_device_tile_cell(ui, left_rect, label, value, DEVICE_METRIC_LEFT_LABEL_W);
        }
        if let Some((label, value)) = pair.get(1) {
            render_device_tile_cell(ui, right_rect, label, value, DEVICE_METRIC_RIGHT_LABEL_W);
        }
    }
}

fn render_device_tile_cell(
    ui: &mut egui::Ui,
    cell_rect: egui::Rect,
    label: &str,
    value: &str,
    label_w: f32,
) {
    let row_h = cell_rect.height();
    let label_w = label_w.min(cell_rect.width());
    let value_w = (cell_rect.width() - label_w - DEVICE_METRIC_VALUE_GAP).max(0.0);
    let label_rect = egui::Rect::from_min_size(cell_rect.min, egui::vec2(label_w, row_h));
    let value_rect = egui::Rect::from_min_size(
        egui::pos2(
            label_rect.right() + DEVICE_METRIC_VALUE_GAP,
            cell_rect.top(),
        ),
        egui::vec2(value_w, row_h),
    );
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(label_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.set_width(label_w);
            ui_kit::metric_label(ui, label);
        },
    );
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(value_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.set_width(value_w);
            ui_kit::metric_value(ui, value);
        },
    );
}
