//! White content cards: the shared frame, the flow card, and the slot card
//! that fills a shell-allocated rectangle.

use eframe::egui::{self, CornerRadius, Margin, Stroke, StrokeKind};

use crate::theme::{radius, tokens, CARD_BEZEL_ALPHA};

/// Inner padding inside a card, both axes. `card_impl` reserves `2x` this when
/// sizing content, so the two stay coupled (change here, not the magic number).
/// Public so layout guards can derive a card's true inner edges from shell rects.
pub const CARD_INNER_MARGIN: i8 = 14;

/// A white content card frame. The shell owns outer spacing between cards.
pub fn card_frame() -> egui::Frame {
    let t = tokens();
    egui::Frame::new()
        .fill(t.surface)
        .stroke(Stroke::new(1.0_f32, t.border))
        .corner_radius(CornerRadius::same(radius::CARD))
        .inner_margin(Margin::same(CARD_INNER_MARGIN))
        .outer_margin(Margin::same(0))
        .shadow(crate::theme::soft_shadow())
}

fn paint_card_bezel(ui: &egui::Ui, rect: egui::Rect) {
    // A barely-there inner highlight gives the white card a machined edge while
    // keeping the app's flat, scientific visual language.
    ui.painter().rect_stroke(
        rect.shrink(0.5),
        CornerRadius::same(radius::CARD),
        Stroke::new(1.0_f32, crate::theme::utility_white_alpha(CARD_BEZEL_ALPHA)),
        StrokeKind::Inside,
    );
}

fn card_impl<R>(ui: &mut egui::Ui, fill_height: bool, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let inset = 2.0 * CARD_INNER_MARGIN as f32;
    let width = (ui.available_width() - inset).max(0.0);
    let height = (ui.available_height() - inset).max(0.0);
    let inner = card_frame().show(ui, |ui| {
        // The card is a self-contained design unit: restore the
        // context-default item spacing so the column shells' zeroed
        // inter-card spacing can't leak into card content
        // (table row math reads `item_spacing`).
        let default_spacing = ui.ctx().global_style().spacing.item_spacing;
        ui.spacing_mut().item_spacing = default_spacing;
        ui.set_min_width(width);
        if fill_height {
            ui.set_width(width);
            ui.set_height(height);
            ui.set_clip_rect(ui.max_rect());
        }
        add(ui)
    });
    paint_card_bezel(ui, inner.response.rect);
    inner.inner
}

/// A white content card: rounded, hairline border, soft shadow, and padded.
pub fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    card_impl(ui, false, add)
}

/// A card that fills the shell slot allocated by the parent layout.
pub fn card_slot<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let slot_size = ui.available_size();
    let (slot_rect, _response) = ui.allocate_exact_size(slot_size, egui::Sense::hover());
    let mut slot_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(slot_rect)
            .layout(*ui.layout()),
    );
    // Clip painting to the slot (an overflowing card must not paint over its
    // neighbors), but expanded so the card's soft drop shadow and 1px stroke
    // aren't sheared at the seam (clearance derived from the shadow geometry).
    slot_ui.set_clip_rect(slot_rect.expand(crate::theme::soft_shadow_clearance()));
    card_impl(&mut slot_ui, true, add)
}
