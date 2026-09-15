//! The shared "single-line edit, parse `f64` on commit" input used by every
//! numeric field that commits on lost focus.

use eframe::egui;

use crate::state::EditBuffers;
use crate::ui_kit;

/// One focus-tracked numeric field. `current` is the committed value already
/// formatted the way the field displays it. Returns `None` while nothing
/// committed, `Some(Ok(value))` for a parsed commit, and `Some(Err(text))` with
/// the raw text when the committed text is not a number; the caller decides
/// whether a parse failure is reported or reverted silently.
pub fn numeric_edit_commit(
    ui: &mut egui::Ui,
    edits: &mut EditBuffers,
    key: &str,
    current: &str,
    width: f32,
) -> Option<Result<f64, String>> {
    let text = ui_kit::singleline_edit_commit(ui, edits, key, current, width)?;
    Some(text.trim().parse::<f64>().map_err(|_| text))
}
