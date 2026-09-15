//! Shared app shell rectangles and fixed-rect rendering.

use eframe::egui::{self, pos2, Rect};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellRects {
    pub top: Rect,
    pub body: Rect,
    pub left: Rect,
    pub center: Rect,
    pub right: Rect,
}

impl ShellRects {
    pub fn from_content(content: Rect) -> Self {
        let top = Rect::from_min_max(
            content.min,
            pos2(content.right(), content.top() + super::TOP_BAR_HEIGHT),
        );
        let body = Rect::from_min_max(
            pos2(
                content.left() + super::PAGE_PAD_X,
                top.bottom() + super::PAGE_PAD_Y,
            ),
            pos2(
                content.right() - super::PAGE_PAD_X,
                content.bottom() - super::PAGE_PAD_Y,
            ),
        );

        // Proportional-with-caps columns: reproduce the base widths at the
        // reference window, grow the side columns toward their caps as the window
        // widens, and let the center column absorb whatever surplus remains.
        let cols_w = (body.width() - 2.0 * super::BODY_GAP).max(0.0);
        let left_w = (cols_w * super::LEFT_FRAC).clamp(super::LEFT_WIDTH, super::LEFT_MAX_WIDTH);
        let right_w =
            (cols_w * super::RIGHT_FRAC).clamp(super::RIGHT_WIDTH, super::RIGHT_MAX_WIDTH);

        let left = Rect::from_min_max(body.min, pos2(body.left() + left_w, body.bottom()));
        let right = Rect::from_min_max(pos2(body.right() - right_w, body.top()), body.max);
        let center = Rect::from_min_max(
            pos2(left.right() + super::BODY_GAP, body.top()),
            pos2(right.left() - super::BODY_GAP, body.bottom()),
        );

        Self {
            top,
            body,
            left,
            center,
            right,
        }
    }
}

pub fn show_in_rect<R>(
    ui: &mut egui::Ui,
    id_salt: &'static str,
    rect: egui::Rect,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    ui.scope_builder(
        egui::UiBuilder::new()
            .id_salt(id_salt)
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
            // Expand past the rect so card drop shadows are not clipped at
            // column seams (clearance derived from the shadow geometry).
            ui.set_clip_rect(rect.expand(crate::theme::soft_shadow_clearance()));
            ui.set_min_size(rect.size());
            ui.set_width(rect.width());
            ui.set_height(rect.height());
            add(ui)
        },
    )
    .inner
}
