//! Custom double-thumb range strip (egui's `Slider` is single-value) + the pure
//! value↔fraction mapping helpers that carry the logic (unit-tested).

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Vec2};

use crate::ui_kit::{self, CONTROL_SLIDER_HEIGHT, CONTROL_SLIDER_INSET, CONTROL_THUMB_RADIUS};

// ── Shared strip visual identity ─────────────────────────────────────────────
// Shared geometry and painting live in `ui_kit`; only range interaction stays here.

/// Keep close thumb hit boxes one point apart while their painted rings overlap.
pub const THUMB_MIN_CENTER_GAP: f32 = CONTROL_THUMB_RADIUS * 2.0 + 1.0;

/// Outcome of a strip interaction this frame.
#[derive(Debug, Clone, Copy, Default)]
pub struct StripDrag {
    /// A thumb is being actively dragged this frame → drive the live preview from `lo`/`hi`.
    pub dragging: bool,
    /// A thumb was released this frame → commit `lo`/`hi` exactly once.
    pub released: bool,
}

/// Inputs for one double-thumb strip.
pub struct StripInputs<'a> {
    pub id_salt: &'a str,
    pub v_min: f64,
    pub v_max: f64,
    pub lo: &'a mut f64,
    pub hi: &'a mut f64,
    pub accent: Color32,
}

/// A custom double-thumb range strip (egui `Slider` is single-value). Mutates
/// `lo`/`hi` live during drag; reports whether it is dragging and/or released.
pub fn double_thumb_strip(ui: &mut egui::Ui, inputs: StripInputs<'_>) -> StripDrag {
    let StripInputs {
        id_salt,
        v_min,
        v_max,
        lo,
        hi,
        accent,
    } = inputs;
    let (rect, _bg) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), CONTROL_SLIDER_HEIGHT),
        Sense::hover(),
    );
    let painter = ui.painter();
    let (x0, x1, mid_y) = (
        rect.left() + CONTROL_SLIDER_INSET,
        rect.right() - CONTROL_SLIDER_INSET,
        rect.center().y,
    );
    // Critical f32/f64 cast: value_to_frac returns f64; pixel coords are f32.
    let to_x = |v: f64| x0 + (value_to_frac(v, v_min, v_max) as f32) * (x1 - x0);
    let to_v = |x: f32| frac_to_value(((x - x0) / (x1 - x0)).clamp(0.0, 1.0) as f64, v_min, v_max);

    // The track + thumbs need enough contrast on the white card. The utility
    // rail plus direction accent make the "bounds" handles read clearly.
    ui_kit::paint_control_rail(painter, x0, x1, mid_y);
    if !ui.is_enabled() {
        return StripDrag::default();
    }
    let (lo_x, hi_x) = (to_x(*lo), to_x(*hi));
    ui_kit::paint_control_rail_segment(painter, lo_x, hi_x, mid_y, accent);

    let rail_rect = Rect::from_min_max(
        Pos2::new(x0.min(x1), mid_y - 8.0),
        Pos2::new(x0.max(x1), mid_y + 8.0),
    );
    let segment_pad = 32.0;
    let active_rect = Rect::from_min_max(
        Pos2::new((lo_x.min(hi_x) - segment_pad).max(x0), mid_y - 8.0),
        Pos2::new((lo_x.max(hi_x) + segment_pad).min(x1), mid_y + 8.0),
    );
    let segment_resp = ui.interact(
        rail_rect,
        ui.id().with((id_salt, 2u8)),
        Sense::click_and_drag(),
    );
    let mut click_committed = false;
    if segment_resp.drag_started() || segment_resp.drag_stopped() || segment_resp.clicked() {
        if let Some(pointer) = segment_resp.interact_pointer_pos() {
            if !active_rect.contains(pointer) {
                let width = (*hi - *lo).abs();
                let center = to_v(pointer.x);
                let moved = slide_window(
                    (v_min, v_max),
                    (center - width / 2.0, center + width / 2.0),
                    0.0,
                );
                *lo = moved.0;
                *hi = moved.1;
                click_committed = segment_resp.clicked();
            }
        }
    }
    if segment_resp.dragged() {
        let span = v_max - v_min;
        let delta_px = ui.input(|i| i.pointer.delta().x);
        let delta = (delta_px as f64 / (x1 - x0) as f64) * span;
        let moved = slide_window((v_min, v_max), (*lo, *hi), delta);
        *lo = moved.0;
        *hi = moved.1;
    }

    let r = CONTROL_THUMB_RADIUS;
    let thumb = |center| Rect::from_center_size(center, Vec2::splat(r * 2.0));
    let (lo_center, hi_center) = thumb_centers(lo_x, hi_x, x0, x1, mid_y);
    let lo_resp = ui.interact(
        thumb(lo_center),
        ui.id().with((id_salt, 0u8)),
        Sense::drag(),
    );
    if lo_resp.dragged() {
        let delta_x = lo_resp.drag_delta().x;
        if delta_x != 0.0 {
            *lo = to_v(to_x(*lo) + delta_x).min(*hi);
        }
    }
    let hi_resp = ui.interact(
        thumb(hi_center),
        ui.id().with((id_salt, 1u8)),
        Sense::drag(),
    );
    if hi_resp.dragged() {
        let delta_x = hi_resp.drag_delta().x;
        if delta_x != 0.0 {
            *hi = to_v(to_x(*hi) + delta_x).max(*lo);
        }
    }
    let painter = ui.painter();
    let (thumb_fill, thumb_stroke) = ui_kit::control_thumb_style(accent);
    let (lo_center, hi_center) = thumb_centers(to_x(*lo), to_x(*hi), x0, x1, mid_y);
    painter.circle(lo_center, r, thumb_fill, thumb_stroke);
    painter.circle(hi_center, r, thumb_fill, thumb_stroke);
    StripDrag {
        dragging: lo_resp.dragged() || hi_resp.dragged() || segment_resp.dragged(),
        released: lo_resp.drag_stopped()
            || hi_resp.drag_stopped()
            || segment_resp.drag_stopped()
            || click_committed,
    }
}

pub fn thumb_centers(lo_x: f32, hi_x: f32, x0: f32, x1: f32, mid_y: f32) -> (Pos2, Pos2) {
    let dx = (hi_x - lo_x).abs();
    if dx < THUMB_MIN_CENTER_GAP {
        let half = THUMB_MIN_CENTER_GAP * 0.5;
        let center = ((lo_x + hi_x) * 0.5).clamp(x0 + half, x1 - half);
        (
            Pos2::new(center - half, mid_y),
            Pos2::new(center + half, mid_y),
        )
    } else {
        (Pos2::new(lo_x, mid_y), Pos2::new(hi_x, mid_y))
    }
}

pub fn slide_window(axis: (f64, f64), window: (f64, f64), delta: f64) -> (f64, f64) {
    let width = window.1 - window.0;
    if width <= 0.0 {
        return clamp_pair(window.0 + delta, window.1 + delta);
    }
    let mut lo = window.0 + delta;
    let mut hi = window.1 + delta;
    if lo < axis.0 {
        lo = axis.0;
        hi = axis.0 + width;
    }
    if hi > axis.1 {
        hi = axis.1;
        lo = axis.1 - width;
    }
    clamp_pair(lo, hi)
}

/// Linear value-to-`[0, 1]` fraction across the axis, clamped. Degenerate axis → 0.0.
pub fn value_to_frac(v: f64, v_min: f64, v_max: f64) -> f64 {
    let span = v_max - v_min;
    if span.abs() <= f64::EPSILON {
        return 0.0;
    }
    ((v - v_min) / span).clamp(0.0, 1.0)
}

/// Inverse of `value_to_frac`. Degenerate axis → `v_min`.
pub fn frac_to_value(t: f64, v_min: f64, v_max: f64) -> f64 {
    let span = v_max - v_min;
    if span.abs() <= f64::EPSILON {
        return v_min;
    }
    v_min + t.clamp(0.0, 1.0) * span
}

/// Keep `lo <= hi` (the lo thumb cannot pass the hi thumb and vice versa).
pub fn clamp_pair(lo: f64, hi: f64) -> (f64, f64) {
    if lo > hi {
        (hi, hi)
    } else {
        (lo, hi)
    }
}
