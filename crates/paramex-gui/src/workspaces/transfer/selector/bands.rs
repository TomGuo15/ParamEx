//! Band polygons + colors. The graph component set uses the Studio Stellar group
//! only: forward/max = `suisei-main`, backward/median = `suisei-dark`. Bands and
//! data series are always SOLID — dashing is reserved for the fit lines.
//! Data series, band fill, band stroke, and fit lines use the shared theme
//! colors so the two directions stay consistent everywhere.

use eframe::egui::Color32;

use crate::theme::{token_alpha, SUISEI_DARK, SUISEI_MAIN};

pub const FORWARD_FILL_ALPHA: u8 = 36;
pub const BACKWARD_FILL_ALPHA: u8 = 26;

/// Four corners of a full-height band across the x-range `[x0, x1]` and live y-range
/// `[y0, y1]`.
pub fn band_rect_points(x0: f64, x1: f64, y0: f64, y1: f64) -> Vec<[f64; 2]> {
    vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
}

/// Forward fill (`suisei-main`, ~14% alpha) — both graphs.
pub fn forward_fill() -> Color32 {
    token_alpha(SUISEI_MAIN, FORWARD_FILL_ALPHA)
}

/// Backward fill (`suisei-dark`, ~10% alpha) — both graphs.
pub fn backward_fill() -> Color32 {
    token_alpha(SUISEI_DARK, BACKWARD_FILL_ALPHA)
}

/// What a pointer-down on the band grabbed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grab {
    EdgeLo,
    EdgeHi,
    Whole,
}

/// Classify a grab at data-x `px` against window `[lo,hi]` with an edge tolerance in
/// data units. Edge wins over Whole; outside the band → None.
pub fn classify_grab(px: f64, lo: f64, hi: f64, edge_data: f64) -> Option<Grab> {
    if (px - lo).abs() <= edge_data {
        Some(Grab::EdgeLo)
    } else if (px - hi).abs() <= edge_data {
        Some(Grab::EdgeHi)
    } else if px > lo && px < hi {
        Some(Grab::Whole)
    } else {
        None
    }
}

/// Cursor language for a (potential) band grab: edges resize horizontally, the
/// interior grabs (slides) — telegraphing what a press would do before it happens.
pub fn cursor_for_grab(grab: Grab, dragging: bool) -> eframe::egui::CursorIcon {
    use eframe::egui::CursorIcon;
    match grab {
        Grab::EdgeLo | Grab::EdgeHi => CursorIcon::ResizeHorizontal,
        Grab::Whole if dragging => CursorIcon::Grabbing,
        Grab::Whole => CursorIcon::Grab,
    }
}

/// Nearest value in a sorted-ascending slice (clamps to the ends). `xs` must be sorted.
pub fn snap_to_nearest_x(xs: &[f64], target: f64) -> f64 {
    if xs.is_empty() {
        return target;
    }
    match xs.binary_search_by(|v| v.partial_cmp(&target).unwrap_or(std::cmp::Ordering::Less)) {
        Ok(i) => xs[i],
        Err(0) => xs[0],
        Err(i) if i >= xs.len() => xs[xs.len() - 1],
        Err(i) => {
            let (a, b) = (xs[i - 1], xs[i]);
            if (target - a).abs() <= (b - target).abs() {
                a
            } else {
                b
            }
        }
    }
}
