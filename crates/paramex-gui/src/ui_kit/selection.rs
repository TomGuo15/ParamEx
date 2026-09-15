//! Selectable list-row chrome: fill/stroke by state, the left accent bar, and
//! the native radio-style selection semantics for hand-painted rows.

use eframe::egui::{self, Color32, Margin, Stroke, StrokeKind};

use crate::theme::{radius, tokens};

/// Pure: list-row fill for (selected, hovered). Selection wins over hover.
pub fn selection_row_fill(selected: bool, hovered: bool) -> Color32 {
    let t = tokens();
    if selected {
        t.accent_soft
    } else if hovered {
        t.surface_muted
    } else {
        Color32::TRANSPARENT
    }
}

/// The frame for a selectable list row. Selection is carried by the fill
/// (`selection_row_fill`) + the 3px accent bar (`selection_bar`) + a primary
/// hairline outline, so the active row reads as a lifted instrument selection
/// without changing row height.
pub fn selection_row_stroke(selected: bool) -> Stroke {
    let t = tokens();
    Stroke::new(1.0_f32, if selected { t.primary } else { t.border })
}

pub fn selection_row_frame(ui: &egui::Ui, selected: bool, hovered: bool) -> egui::Frame {
    egui::Frame::group(ui.style())
        .fill(selection_row_fill(selected, hovered))
        .stroke(selection_row_stroke(selected))
        .inner_margin(Margin::symmetric(6, 3))
}

/// Paint the 3px `primary` selection bar along the row's left edge, inset past
/// the frame's 1px stroke and widget-radius rounded corners so it stays inside
/// the visible row outline (square ends are fine once inset).
pub fn selection_bar(ui: &egui::Ui, row_rect: egui::Rect) {
    // `Frame::group` rounds the row with the theme's widget corner radius.
    let r = f32::from(radius::WIDGET);
    let x = row_rect.min.x + 1.0; // skip the 1px frame stroke
    let bar = egui::Rect::from_min_max(
        egui::pos2(x, row_rect.min.y + r),
        egui::pos2(x + 3.0, (row_rect.bottom() - r).max(row_rect.min.y + r)),
    );
    ui.painter()
        .rect_filled(bar, egui::CornerRadius::ZERO, tokens().primary);
}

/// Add native mutually-exclusive selection semantics to a hand-painted row.
/// The caller keeps control of the hit rectangle so nested actions (checkboxes
/// and remove buttons) remain independent.
pub fn selectable_row_response(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    id: egui::Id,
    label: &str,
    selected: bool,
) -> egui::Response {
    let response = ui.interact(rect, id, egui::Sense::click());
    let enabled = response.enabled();
    let accessible_label = format!("Select {label}");
    response.widget_info(move || {
        egui::WidgetInfo::selected(
            egui::WidgetType::RadioButton,
            enabled,
            selected,
            accessible_label.clone(),
        )
    });
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.shrink(2.0),
            egui::CornerRadius::same(radius::SEGMENT),
            Stroke::new(2.0_f32, tokens().primary),
            StrokeKind::Inside,
        );
    }
    response
}
