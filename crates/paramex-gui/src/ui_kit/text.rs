//! Text voices: muted metadata, row titles, field labels, and the file-row
//! checkbox gutter that keeps error rows aligned with selectable rows.

use eframe::egui::{self, FontFamily, FontId, Response, RichText};

use crate::richtext;
use crate::theme::{tokens, type_scale};

/// The app's small muted metadata/help text recipe.
fn muted_rich_text(text: impl Into<String>) -> RichText {
    RichText::new(text)
        .color(tokens().ink_soft)
        .size(type_scale::MUTED)
}

pub fn muted_label(ui: &mut egui::Ui, text: impl Into<String>) -> Response {
    ui.label(muted_rich_text(text))
}

pub fn muted_wrapped_label(ui: &mut egui::Ui, text: impl Into<String>) -> Response {
    ui.add(egui::Label::new(muted_rich_text(text)).wrap())
}

/// Primary row title text: selected-file names and TLM group names share the
/// same ink voice.
pub fn row_title_color() -> egui::Color32 {
    tokens().ink
}

fn row_title_text(text: impl Into<String>) -> RichText {
    RichText::new(text).color(row_title_color())
}

pub fn row_title_label(ui: &mut egui::Ui, text: impl Into<String>) -> Response {
    ui.add(egui::Label::new(row_title_text(text)).selectable(false))
}

pub fn muted_row_title_label(ui: &mut egui::Ui, text: impl Into<String>) -> Response {
    ui.add(egui::Label::new(RichText::new(text).color(tokens().ink_soft)).selectable(false))
}

pub fn truncated_row_title_label(ui: &mut egui::Ui, text: impl Into<String>) -> Response {
    ui.add(
        egui::Label::new(row_title_text(text))
            .selectable(false)
            .truncate(),
    )
}

/// Width reserved by the checkbox column in file-list rows. Error rows are not
/// selectable, but reserving the same gutter keeps their title/status content
/// aligned with OK file rows.
const FILE_ROW_GUTTER_WIDTH: f32 = 18.0;

pub fn file_row_gutter(ui: &mut egui::Ui) {
    ui.allocate_space(egui::vec2(FILE_ROW_GUTTER_WIDTH, 0.0));
}

/// A small grey field label rendered inline left of its input.
pub fn field_label(ui: &mut egui::Ui, text: &str) {
    ui.label(muted_rich_text(text));
}

/// Like [`field_label`] but `<sub>`/`<sup>`-aware -- for placeholder/help text that names
/// metrics (V<sub>TH</sub>, mu<sub>sat</sub>, I<sub>on</sub>/I<sub>off</sub>). Wraps to the
/// available width like a plain label.
pub fn field_label_rich(ui: &mut egui::Ui, markup: &str) {
    let font = FontId::new(type_scale::MUTED, FontFamily::Proportional);
    let mut job = richtext::layout_sub_sup(markup, font, tokens().ink_soft);
    job.wrap.max_width = ui.available_width();
    ui.label(job);
}
