//! Metric label/value voices for the summary grids and the inline readouts
//! beside custom controls.

use eframe::egui::{self, Color32, Response, RichText};

use crate::richtext;
use crate::theme::{tokens, type_scale};

pub fn metric_label_color() -> Color32 {
    tokens().ink_soft
}

pub fn metric_value_color() -> Color32 {
    tokens().ink
}

/// Markup-aware metric names in label/value grids. Metric labels are muted so
/// the measured value carries the visual weight in both Transfer and TLM cards.
pub fn metric_label(ui: &mut egui::Ui, markup: &str) -> Response {
    richtext::rich_colored(ui, markup, metric_label_color())
}

/// Markup-aware metric values in label/value grids.
pub fn metric_value(ui: &mut egui::Ui, markup: &str) -> Response {
    richtext::rich_colored(ui, markup, metric_value_color())
}

/// One cell in a metric label/value table. Column zero is the muted metric
/// label; every other column is a value. Keeps table callers at the semantic
/// level instead of repeating label/value branching in each panel.
pub fn metric_table_cell(ui: &mut egui::Ui, col: usize, markup: &str) {
    if col == 0 {
        metric_label(ui, markup);
    } else {
        metric_value(ui, markup);
    }
}

/// Compact inline numeric readout, used beside custom controls where a full
/// label/value grid would be too heavy.
pub fn readout_value_color() -> Color32 {
    metric_value_color()
}

pub fn readout_unit_color() -> Color32 {
    metric_label_color()
}

fn readout_value_text(text: impl Into<String>) -> RichText {
    RichText::new(text).color(readout_value_color())
}

/// Inline value recipe for strip readouts (the TLM V_G picker).
pub fn readout_value_label(ui: &mut egui::Ui, text: impl Into<String>) -> Response {
    ui.add(egui::Label::new(readout_value_text(text)).selectable(false))
}

fn readout_unit_text(text: impl Into<String>) -> RichText {
    RichText::new(text)
        .color(readout_unit_color())
        .size(type_scale::MUTED)
}

/// Inline unit recipe paired with [`readout_value_label`].
pub fn readout_unit_label(ui: &mut egui::Ui, text: impl Into<String>) -> Response {
    ui.add(egui::Label::new(readout_unit_text(text)).selectable(false))
}
