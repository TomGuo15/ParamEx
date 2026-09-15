//! Error summary and dismissible error-row recipes.

use eframe::egui;

use super::super::{
    close_button, file_row_gutter, right_aligned, selection_row_frame, truncated_row_title_label,
};
use super::{status_badge_line, BadgeTone, StatusLineText};

/// Width reserved on the right of an error row for the dismiss cross.
const ERROR_DISMISS_COLUMN_WIDTH: f32 = 30.0;
pub const FILE_ERROR_SUMMARY_MAX_CHARS: usize = 44;

pub fn file_error_summary(message: &str) -> String {
    let message = message.trim();
    if message.is_empty() {
        return "Import failed".to_string();
    }
    if message.starts_with("No usable transfer curve found") {
        return "No usable transfer curve".to_string();
    }
    summarize_first_sentence(message)
}

pub fn load_error_summary(message: &str) -> String {
    let message = message.trim();
    if message.is_empty() {
        return "Load failed".to_string();
    }
    if message.starts_with("No valid TLM workbooks") {
        return "No valid TLM workbooks".to_string();
    }
    if message.starts_with("Could not load the selected folder") {
        return "Folder did not match TLM layout".to_string();
    }
    summarize_first_sentence(message)
}

/// First sentence of `message` (up to the first `.`), compacted to the summary
/// length. Shared tail of [`file_error_summary`] / [`load_error_summary`].
fn summarize_first_sentence(message: &str) -> String {
    let first_sentence = message.split('.').next().unwrap_or(message).trim();
    let summary = if first_sentence.is_empty() {
        message
    } else {
        first_sentence
    };
    compact_summary(summary, FILE_ERROR_SUMMARY_MAX_CHARS)
}

fn compact_summary(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let head: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{head}...")
    } else {
        head
    }
}

/// File-list ERROR row. It uses the same outer row family as OK file rows:
/// normal list-row frame, reserved checkbox gutter, filename typography, status
/// badge line, and a right-pinned dismiss action.
pub fn file_error_row(ui: &mut egui::Ui, title: impl Into<String>, message: &str) -> bool {
    let mut dismissed = false;
    let frame = selection_row_frame(ui, false, false);
    frame.show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            file_row_gutter(ui);
            ui.vertical(|ui| {
                ui.set_max_width((ui.available_width() - ERROR_DISMISS_COLUMN_WIDTH).max(0.0));
                truncated_row_title_label(ui, title);
                let summary = file_error_summary(message);
                let status = status_badge_line(
                    ui,
                    "error",
                    BadgeTone::Error,
                    StatusLineText::Inline(summary.as_str()),
                    |_| {},
                );
                status.response.on_hover_text(message);
            });
            right_aligned(ui, |ui| {
                if close_button(ui, "Dismiss")
                    .on_hover_text("Dismiss")
                    .clicked()
                {
                    dismissed = true;
                }
            });
        });
    });
    dismissed
}

/// Compact dismissible error notice for form cards. Kept as the DATA load
/// error recipe so form-card errors do not reimplement file-list row chrome.
pub fn compact_error_notice(ui: &mut egui::Ui, summary: impl Into<String>, message: &str) -> bool {
    let mut dismissed = false;
    ui.horizontal(|ui| {
        let summary = summary.into();
        let status = status_badge_line(
            ui,
            "error",
            BadgeTone::Error,
            StatusLineText::Inline(summary.as_str()),
            |_| {},
        );
        status.response.on_hover_text(message);
        right_aligned(ui, |ui| {
            if close_button(ui, "Dismiss")
                .on_hover_text("Dismiss")
                .clicked()
            {
                dismissed = true;
            }
        });
    });
    dismissed
}
