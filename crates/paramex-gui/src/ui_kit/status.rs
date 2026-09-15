//! Status chips and the badge + detail row grammar shared by file rows, group
//! rows, and load-error notices.

mod errors;

use eframe::egui::{
    self, Color32, CornerRadius, FontFamily, FontId, Margin, Response, RichText, Stroke,
};

use crate::richtext;
use crate::theme::{radius, tokens, type_scale};

use super::{muted_label, muted_wrapped_label, truncated_row_title_label};

pub use errors::{
    compact_error_notice, file_error_row, file_error_summary, load_error_summary,
    FILE_ERROR_SUMMARY_MAX_CHARS,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BadgeTone {
    Ok,
    Warning,
    Error,
}

/// Compact status chip used in dense lists/tables. The semantic fill comes from
/// the ReGLOSS token group; dark Studio Stellar text keeps pastel chips readable.
pub fn semantic_badge_colors(tone: BadgeTone) -> (Color32, Color32, Color32) {
    let t = tokens();
    let accent = match tone {
        BadgeTone::Ok => t.green,
        BadgeTone::Warning => t.yellow,
        BadgeTone::Error => t.red,
    };
    (accent, accent, t.ink)
}

pub fn semantic_badge(ui: &mut egui::Ui, text: &str, tone: BadgeTone) -> Response {
    let (fill, stroke, text_color) = semantic_badge_colors(tone);
    egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, stroke))
        .inner_margin(Margin::symmetric(5, 1))
        .corner_radius(CornerRadius::same(radius::CHIP))
        .show(ui, |ui| {
            ui.label(
                RichText::new(text.to_ascii_uppercase())
                    .color(text_color)
                    .size(type_scale::BADGE)
                    .strong(),
            );
        })
        .response
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusLineText<'a> {
    Inline(&'a str),
    RichInline(&'a str),
    Wrapped(&'a str),
}

/// Shared badge + detail row used by file-status rows and folder-load errors.
/// Keep the badge, optional extra badges, and detail text as one recipe so OK
/// and ERROR states cannot drift into different row grammars.
pub fn status_badge_line(
    ui: &mut egui::Ui,
    badge: &str,
    tone: BadgeTone,
    detail: StatusLineText<'_>,
    add_extra_badges: impl FnOnce(&mut egui::Ui),
) -> egui::InnerResponse<()> {
    ui.horizontal_top(|ui| {
        semantic_badge(ui, badge, tone);
        add_extra_badges(ui);
        match detail {
            StatusLineText::Inline(text) => {
                muted_label(ui, text);
            }
            StatusLineText::RichInline(text) => {
                let font = FontId::new(type_scale::MUTED, FontFamily::Proportional);
                ui.label(richtext::layout_sub_sup(text, font, tokens().ink_soft));
            }
            StatusLineText::Wrapped(text) => {
                muted_wrapped_label(ui, text);
            }
        }
    })
}

pub fn list_row_title_status(
    ui: &mut egui::Ui,
    title: impl Into<String>,
    badge: &str,
    tone: BadgeTone,
    detail: StatusLineText<'_>,
    add_extra_badges: impl FnOnce(&mut egui::Ui),
) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 1.0;
        truncated_row_title_label(ui, title);
        status_badge_line(ui, badge, tone, detail, add_extra_badges);
    });
}
