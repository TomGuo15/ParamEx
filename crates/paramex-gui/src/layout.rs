//! Responsive shell geometry: proportional-with-caps columns pinned to reproduce
//! the reference 1280×800 design exactly at the reference window.
//!
//! The shell owns page padding, column widths, and gutters. Individual panels
//! own only their content; they must not add outer card margins that fight this grid.

mod shell;

use eframe::egui;

pub use self::shell::{show_in_rect, ShellRects};

pub const TOP_BAR_HEIGHT: f32 = 56.0;
pub const PAGE_PAD_X: f32 = 12.0;
pub const PAGE_PAD_Y: f32 = 12.0;
pub const BODY_GAP: f32 = 16.0;
pub const CARD_GAP: f32 = 16.0;
pub const LEFT_WIDTH: f32 = 280.0;
/// The right column matches the left; the center column absorbs the difference.
pub const RIGHT_WIDTH: f32 = LEFT_WIDTH;

/// Reference window width the fixed layout was designed for. At this width the
/// responsive column math reproduces the base `LEFT_WIDTH`/`RIGHT_WIDTH` exactly.
pub const REF_WINDOW_WIDTH: f32 = 1280.0;

/// Total width available to the three columns at the reference window width
/// (page padding + both body gutters removed): 1280 - 2·12 - 2·16 = 1224.
const REF_COLS_WIDTH: f32 = REF_WINDOW_WIDTH - 2.0 * PAGE_PAD_X - 2.0 * BODY_GAP;

/// Fraction of the column band each side column claims, pinned so the reference
/// width yields the base widths exactly (280/1224 each side).
const LEFT_FRAC: f32 = LEFT_WIDTH / REF_COLS_WIDTH;
const RIGHT_FRAC: f32 = RIGHT_WIDTH / REF_COLS_WIDTH;

/// Side-column width caps. Once a side column hits its cap, all further surplus
/// goes to the center column (so the graphs grow to fill on wide/maximized windows).
pub const LEFT_MAX_WIDTH: f32 = 360.0;
pub const RIGHT_MAX_WIDTH: f32 = LEFT_MAX_WIDTH;

pub const FILES_MIN_HEIGHT: f32 = 250.0;
pub const SELECTED_METRICS_HEIGHT: f32 = 278.0;
pub const CONTENT_CARD_MIN_HEIGHT: f32 = 120.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StackHeights {
    pub top: f32,
    pub bottom: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackSlot {
    Top,
    Bottom,
}

/// Prepare a shell column for explicit card allocation. The cards restore normal
/// spacing internally, but the column itself must not add egui's default
/// inter-widget spacing on top of `CARD_GAP`.
pub fn prepare_column(ui: &mut egui::Ui) {
    ui.spacing_mut().item_spacing.y = 0.0;
    ui.set_width(ui.available_width());
}

/// Render a standard two-card column stack: flex top slot, one page card gap,
/// then bottom slot. Height calculation stays separate (`fixed_bottom_stack` or
/// `content_bottom_stack`); this owns the repeated egui allocation mechanics.
pub fn show_card_stack(
    ui: &mut egui::Ui,
    stack: StackHeights,
    mut add: impl FnMut(&mut egui::Ui, StackSlot),
) {
    prepare_column(ui);
    ui.allocate_ui(egui::vec2(ui.available_width(), stack.top), |ui| {
        add(ui, StackSlot::Top);
    });
    ui.add_space(CARD_GAP);
    ui.allocate_ui(egui::vec2(ui.available_width(), stack.bottom), |ui| {
        add(ui, StackSlot::Bottom);
    });
}

/// Flex-top + fixed-bottom card stack used by the page-wide bottom band.
pub fn fixed_bottom_stack(available_height: f32, bottom_height: f32) -> StackHeights {
    StackHeights {
        top: (available_height - bottom_height - CARD_GAP).max(FILES_MIN_HEIGHT),
        bottom: bottom_height,
    }
}

/// Flex-top + content-sized-bottom stack used when the bottom card should show
/// all of its predictable content while the top card absorbs the remaining space.
pub fn content_bottom_stack(available_height: f32, desired_bottom_height: f32) -> StackHeights {
    let bottom_cap = (available_height - FILES_MIN_HEIGHT - CARD_GAP).max(CONTENT_CARD_MIN_HEIGHT);
    let bottom = desired_bottom_height.min(bottom_cap);
    StackHeights {
        top: (available_height - bottom - CARD_GAP).max(FILES_MIN_HEIGHT),
        bottom,
    }
}
