//! Quiet-table text measurement, labels, and clipped-hover helpers.

use eframe::egui;

use crate::richtext;
use crate::theme::{tokens, type_scale};

/// Laid-out pixel width of `markup` (honouring `<sub>`/`<sup>`) at `base` font.
pub fn galley_width(ui: &egui::Ui, markup: &str, base: &egui::FontId) -> f32 {
    let job = richtext::layout_sub_sup(markup, base.clone(), table_measure_text_color());
    ui.painter().layout_job(job).rect.width()
}

/// Text color used for offscreen measurement jobs. Width is color-independent,
/// but routing through tokens keeps table text helpers inside the palette system.
pub fn table_measure_text_color() -> egui::Color32 {
    tokens().ink
}

/// The quiet-table header font (the muted size — also what `hover_if_clipped`-style
/// checks must measure header labels with).
pub fn muted_header_font() -> egui::FontId {
    egui::FontId::new(type_scale::MUTED, egui::FontFamily::Proportional)
}

/// Muted rich header label — the one quiet-table header voice (same size and
/// color as `striped_fill_table`'s headers, markup-aware for unit-bearing labels).
pub fn muted_header_label(ui: &mut egui::Ui, markup: &str) -> egui::Response {
    ui.label(richtext::layout_sub_sup(
        markup,
        muted_header_font(),
        tokens().ink_soft,
    ))
}

/// Body-cell label for quiet analytical tables. It is markup-aware like the
/// measurement path, and keeps table body text on the shared egui Body voice.
pub fn body_label(ui: &mut egui::Ui, markup: &str) -> egui::Response {
    richtext::rich_label(ui, markup)
}

/// Attach the full plain-text value as hover help when `markup` is wider than
/// the rendered cell. The width check uses the same font and markup parser as
/// the label render path, avoiding false clipped reports for muted headers.
pub fn hover_if_clipped_at(ui: &egui::Ui, resp: egui::Response, markup: &str, font: &egui::FontId) {
    if markup.is_empty() {
        return;
    }
    if galley_width(ui, markup, font) > resp.rect.width() {
        resp.on_hover_text(richtext::strip_markup(markup));
    }
}

/// [`hover_if_clipped_at`] using the normal table body font.
pub fn hover_if_clipped(ui: &egui::Ui, resp: egui::Response, markup: &str) {
    let base = egui::TextStyle::Body.resolve(ui.style());
    hover_if_clipped_at(ui, resp, markup, &base);
}
