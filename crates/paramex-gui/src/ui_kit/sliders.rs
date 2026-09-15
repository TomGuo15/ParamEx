//! Geometry, rail painting, and native keyboard/assistive input for the
//! custom-painted horizontal sliders.

use eframe::egui::{self, Color32, CornerRadius, Stroke};

use crate::theme::tokens;

/// Shared geometry for custom-painted horizontal controls.
pub const CONTROL_SLIDER_HEIGHT: f32 = 24.0;
pub const CONTROL_SLIDER_INSET: f32 = 13.0;
pub const CONTROL_THUMB_RADIUS: f32 = 7.0;
pub const CONTROL_THUMB_RING_WIDTH: f32 = 2.0;
const CONTROL_RAIL_HALF_HEIGHT: f32 = 2.5;

pub fn control_thumb_style(accent: Color32) -> (Color32, Stroke) {
    (
        tokens().surface,
        Stroke::new(CONTROL_THUMB_RING_WIDTH, accent),
    )
}

/// Native keyboard and assistive input for a custom-painted horizontal slider.
/// Returns `(index_delta, requested_value, focused)`; the caller owns snapping
/// and painting because measured-value rails need domain-specific positions.
pub fn discrete_slider_input(
    ui: &mut egui::Ui,
    response: &egui::Response,
) -> (isize, Option<f64>, bool) {
    let focused = response.has_focus();
    let mut decrement = 0usize;
    let mut increment = 0usize;
    if focused {
        ui.memory_mut(|memory| {
            memory.set_focus_lock_filter(
                response.id,
                egui::EventFilter {
                    horizontal_arrows: true,
                    ..Default::default()
                },
            );
        });
        ui.input(|input| {
            decrement += input.num_presses(egui::Key::ArrowLeft);
            increment += input.num_presses(egui::Key::ArrowRight);
        });
    }
    ui.input(|input| {
        use egui::accesskit::Action;
        decrement += input.num_accesskit_action_requests(response.id, Action::Decrement);
        increment += input.num_accesskit_action_requests(response.id, Action::Increment);
    });
    let mut requested_value = None;
    ui.input(|input| {
        use egui::accesskit::{Action, ActionData};
        for request in input.accesskit_action_requests(response.id, Action::SetValue) {
            if let Some(ActionData::NumericValue(value)) = request.data {
                requested_value = Some(value);
            }
        }
    });
    (
        increment as isize - decrement as isize,
        requested_value,
        focused,
    )
}

const CONTROL_RAIL_RADIUS: u8 = 2;
pub const CONTROL_RAIL_COLOR: Color32 = crate::theme::UTILITY_GRAY;

fn control_rail_rect(x0: f32, x1: f32, mid_y: f32) -> egui::Rect {
    egui::Rect::from_min_max(
        egui::Pos2::new(x0, mid_y - CONTROL_RAIL_HALF_HEIGHT),
        egui::Pos2::new(x1, mid_y + CONTROL_RAIL_HALF_HEIGHT),
    )
}

pub fn paint_control_rail_segment(
    painter: &egui::Painter,
    x0: f32,
    x1: f32,
    mid_y: f32,
    color: Color32,
) {
    painter.rect_filled(
        control_rail_rect(x0, x1, mid_y),
        CornerRadius::same(CONTROL_RAIL_RADIUS),
        color,
    );
}

pub fn paint_control_rail(painter: &egui::Painter, x0: f32, x1: f32, mid_y: f32) {
    paint_control_rail_segment(painter, x0, x1, mid_y, CONTROL_RAIL_COLOR);
}
