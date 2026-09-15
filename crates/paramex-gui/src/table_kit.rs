//! Generic quiet-table machinery shared by the transfer results table and the
//! TLM tables: content-measured column widths, padding bounds, and the shared
//! header voice (muted labels over a single hairline rule). Rows are boxless —
//! striping carries row structure, alignment carries column structure.

mod fill;
mod sizing;
mod text;

use eframe::egui;
use egui_extras::TableBuilder;

use crate::theme::{token_alpha, tokens};

pub use fill::{striped_fill_table, FillTableSpec};
pub use sizing::{
    fill_card_width, fit_fill_widths, fit_yielding_widths, measure_grid_col_galleys,
    measure_grid_col_widths, pad_and_clamp,
};
pub use text::{
    body_label, galley_width, hover_if_clipped, hover_if_clipped_at, muted_header_font,
    muted_header_label, table_measure_text_color,
};

/// Horizontal breathing room added to a column's measured text width (~6px/side).
pub const CELL_PAD_X: f32 = 12.0;
/// Per-side text inset inside a cell (the measured width carries CELL_PAD_X total),
/// used by the aligned quiet-table cells so flush-left/right text keeps breathing room.
pub const CELL_INSET: f32 = CELL_PAD_X / 2.0;
/// Upper bound so one long status/message cell can't make a column absurdly wide.
pub const COL_MAX_WIDTH: f32 = 260.0;
/// The app-standard single-line table row height, shared by every quiet table
/// (transfer results, TLM grids, the TLM metrics card) so sibling tables read
/// as one family. Call sites with a documented reason may override (the
/// SELECTED card's 20px rows that fit six metrics in 286px).
pub const ROW_H: f32 = 22.0;

pub fn quiet_table_builder(ui: &mut egui::Ui, max_scroll_height: f32) -> TableBuilder<'_> {
    TableBuilder::new(ui)
        .striped(true)
        .vscroll(true)
        .min_scrolled_height(0.0)
        .max_scroll_height(max_scroll_height.max(0.0))
        .auto_shrink([false, false])
        .scroll_bar_visibility(
            egui::containers::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
        )
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
}

/// 1px hairline under a header cell, extended half the column gap per side so the
/// per-cell segments join into one continuous rule. Painted at bottom+0.5 — inside
/// the header band, clear of the first body stripe (stripes expand 0.5×item_spacing
/// past their row, so the first one starts at bottom+1.5). The paint escapes a
/// `clip(true)` cell's tight clip rect (3px margin < half_gap) — without that the
/// rule shows a 2px hole at every column boundary on clipped tables (TLM).
pub fn header_rule(ui: &egui::Ui) {
    let r = ui.max_rect();
    let half_gap = ui.spacing().item_spacing.x / 2.0;
    let painter = ui
        .painter()
        .with_clip_rect(ui.clip_rect().expand2(egui::vec2(half_gap, 0.0)));
    painter.hline(
        egui::Rangef::new(r.left() - half_gap, r.right() + half_gap),
        r.bottom() + 0.5,
        header_rule_stroke(),
    );
}

pub fn header_rule_stroke() -> egui::Stroke {
    egui::Stroke::new(1.0_f32, tokens().border)
}

pub const GROUP_SEPARATOR_ALPHA: u8 = 140;

pub fn group_separator_stroke() -> egui::Stroke {
    egui::Stroke::new(1.0_f32, token_alpha(tokens().border, GROUP_SEPARATOR_ALPHA))
}

/// Lay one cell's content with the column's alignment — the single place the quiet
/// tables' alignment language lives: numeric columns right-align flush to the cell's
/// right edge minus [`CELL_INSET`]; text columns left-align with the same inset.
pub fn aligned_cell<R>(
    ui: &mut egui::Ui,
    right: bool,
    content: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    if right {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(CELL_INSET);
            content(ui)
        })
        .inner
    } else {
        ui.add_space(CELL_INSET);
        content(ui)
    }
}

/// Horizontal-only scroll wrapper for analytical tables. The card width is
/// captured before opening the scroll area because scroll contents get an
/// unbounded horizontal extent; callers use that width for fill-vs-overflow
/// column fitting while the table itself owns vertical scrolling.
pub fn horizontal_table_scroll<R>(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    content: impl FnOnce(&mut egui::Ui, f32) -> R,
) -> R {
    let card_w = ui.available_width();
    let body_h = ui.available_height().max(0.0);
    egui::ScrollArea::horizontal()
        .id_salt(id_salt)
        .auto_shrink([false, false])
        .max_height(body_h)
        .show(ui, |ui| content(ui, card_w))
        .inner
}
