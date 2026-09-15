//! Numeric text inputs and the focus-tracked commit ritual around them: the
//! compact right-aligned field, the label-left cell recipes, and the paired
//! W/L-style row.

use eframe::egui::{self, Response};

use super::field_label_rich;

pub const COMPACT_NUMERIC_INPUT_WIDTH: f32 = 64.0;
pub const INPUT_LABEL_GAP: f32 = 6.0;
const INPUT_PAIR_GAP: f32 = 8.0;
const PAIRED_CELL_MIN_WIDTH: f32 = 80.0;
const TERMINAL_CONTROL_BOTTOM_INSET: f32 = 4.0;
// The 17-point label glyph box otherwise centers half a logical point above
// egui's native-height numeric input (a full physical pixel at 200% scale).
const FIELD_LABEL_CENTER_OFFSET: f32 = 0.5;

fn numeric_text_edit(text: &mut String) -> egui::TextEdit<'_> {
    egui::TextEdit::singleline(text)
        .horizontal_align(egui::Align::RIGHT)
        .vertical_align(egui::Align::Center)
}

/// Render a terminal numeric row on one shared bottom rail. Plot panels use
/// this after their strips so sibling views cannot round the final row onto
/// different physical pixels at fractional display scales.
pub fn terminal_numeric_row<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let row_height = ui.spacing().interact_size.y;
    let max = ui.max_rect();
    let bottom = max.bottom() - TERMINAL_CONTROL_BOTTOM_INSET;
    let row = egui::Rect::from_min_max(
        egui::pos2(max.left(), bottom - row_height),
        egui::pos2(max.right(), bottom),
    );
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(row)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.set_width(row.width());
            ui.set_height(row_height);
            add(ui)
        },
    )
    .inner
}

/// Shared single-line text input entry point. Width still belongs to each
/// layout context, but the widget creation is centralized so text fields cannot
/// drift into one-off styling.
pub fn singleline_edit(ui: &mut egui::Ui, text: &mut String, width: f32) -> Response {
    ui.add(numeric_text_edit(text).desired_width(width))
}

/// One label-left numeric cell spanning the available width, committed through
/// the shared focus-tracked buffer.
pub fn inline_settings_row_commit(
    ui: &mut egui::Ui,
    edits: &mut crate::state::EditBuffers,
    key: &str,
    label_markup: &str,
    current: &str,
) -> Option<String> {
    let width = ui.available_width();
    settings_cell_commit(ui, edits, key, label_markup, current, width)
}

/// Two label-left numeric cells side by side on one native-height row, split
/// evenly across `row_w`. Returns `(left_changed, right_changed)`.
pub fn inline_paired_settings_row_sized(
    ui: &mut egui::Ui,
    row_w: f32,
    left_label: &str,
    left_text: &mut String,
    right_label: &str,
    right_text: &mut String,
) -> (bool, bool) {
    let row_h = ui.spacing().interact_size.y;
    let cell_w = paired_cell_width(row_w);
    let (row_rect, _) = ui.allocate_exact_size(egui::vec2(row_w, row_h), egui::Sense::hover());
    let left_rect = egui::Rect::from_min_size(row_rect.min, egui::vec2(cell_w, row_h));
    let right_rect = egui::Rect::from_min_size(
        egui::pos2(left_rect.right() + INPUT_PAIR_GAP, row_rect.top()),
        egui::vec2(cell_w, row_h),
    );
    let label_w = (cell_w - INPUT_LABEL_GAP - COMPACT_NUMERIC_INPUT_WIDTH).max(0.0);
    let left = inline_settings_cell(ui, left_rect, left_label, left_text, label_w).changed();
    let right = inline_settings_cell(ui, right_rect, right_label, right_text, label_w).changed();
    (left, right)
}

fn inline_settings_cell(
    ui: &mut egui::Ui,
    cell_rect: egui::Rect,
    label_markup: &str,
    text: &mut String,
    label_width: f32,
) -> Response {
    let row_h = cell_rect.height();
    let label_rect = egui::Rect::from_min_size(cell_rect.min, egui::vec2(label_width, row_h))
        .translate(egui::vec2(0.0, FIELD_LABEL_CENTER_OFFSET));
    let input_rect = egui::Rect::from_min_size(
        egui::pos2(label_rect.right() + INPUT_LABEL_GAP, cell_rect.top()),
        egui::vec2(COMPACT_NUMERIC_INPUT_WIDTH, row_h),
    );

    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(label_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.set_width(label_width);
            field_label_rich(ui, label_markup);
        },
    );
    ui.put(input_rect, numeric_text_edit(text))
}

fn paired_cell_width(row_w: f32) -> f32 {
    ((row_w - INPUT_PAIR_GAP) * 0.5).max(PAIRED_CELL_MIN_WIDTH)
}

/// A label-left numeric cell of `width`: the compact input pinned to the right
/// edge, the markup label filling the rest.
fn settings_cell(ui: &mut egui::Ui, label_markup: &str, text: &mut String, width: f32) -> Response {
    let row_h = ui.spacing().interact_size.y;
    let (row_rect, _) = ui.allocate_exact_size(egui::vec2(width, row_h), egui::Sense::hover());
    let input_rect = egui::Rect::from_min_size(
        egui::pos2(
            row_rect.right() - COMPACT_NUMERIC_INPUT_WIDTH,
            row_rect.top(),
        ),
        egui::vec2(COMPACT_NUMERIC_INPUT_WIDTH, row_h),
    );
    let label_rect = egui::Rect::from_min_max(
        row_rect.min,
        egui::pos2(
            (input_rect.left() - INPUT_LABEL_GAP).max(row_rect.left()),
            row_rect.bottom(),
        ),
    )
    .translate(egui::vec2(0.0, FIELD_LABEL_CENTER_OFFSET));
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(label_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.set_width(label_rect.width());
            field_label_rich(ui, label_markup)
        },
    );
    ui.put(input_rect, numeric_text_edit(text))
}

/// [`inline_settings_row_commit`] at an explicit `width`.
pub fn settings_cell_commit(
    ui: &mut egui::Ui,
    edits: &mut crate::state::EditBuffers,
    key: &str,
    label_markup: &str,
    current: &str,
    width: f32,
) -> Option<String> {
    let resp = settings_cell(ui, label_markup, edits.buffer(key, current), width);
    commit_response(ui, edits, key, current, &resp)
}

/// A focus-tracked single-line edit. Returns the committed text on blur or
/// Enter; otherwise keeps the user's active edit or re-syncs to the committed
/// value while unfocused via [`crate::state::EditBuffers`].
pub fn singleline_edit_commit(
    ui: &mut egui::Ui,
    edits: &mut crate::state::EditBuffers,
    key: &str,
    current: &str,
    width: f32,
) -> Option<String> {
    let resp = singleline_edit(ui, edits.buffer(key, current), width);
    commit_response(ui, edits, key, current, &resp)
}

fn commit_response(
    ui: &egui::Ui,
    edits: &mut crate::state::EditBuffers,
    key: &str,
    current: &str,
    resp: &Response,
) -> Option<String> {
    let enter_pressed = resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    commit_only_if_changed(
        edits,
        key,
        current,
        resp.lost_focus() || enter_pressed,
        resp.has_focus() && !enter_pressed,
    )
}

/// Commit on blur/Enter, but ONLY when the text actually changed from `current`. A
/// focus-steal — a button / toggle / strip / workspace switch grabbing the field's focus —
/// fires `lost_focus` with the UNCHANGED seed value; re-committing that would spuriously pin
/// a fit window, flip a geometry row's source to "manual", or truncate a stored value to its
/// 3-decimal display. Gating here covers every focus-tracked field in one place. (Caveat: a
/// field whose committed value was changed underneath it by a sibling action the same frame —
/// e.g. a global W/L apply — holds a now-stale buffer that does differ from the new `current`;
/// that path must DROP the buffer so `current` re-seeds, via
/// [`crate::state::EditBuffers::forget_prefix`], not rely on this.)
fn commit_only_if_changed(
    edits: &mut crate::state::EditBuffers,
    key: &str,
    current: &str,
    lost_focus: bool,
    has_focus: bool,
) -> Option<String> {
    let committed = edits.take_on_commit(key, lost_focus, has_focus)?;
    (committed.trim() != current.trim()).then_some(committed)
}
