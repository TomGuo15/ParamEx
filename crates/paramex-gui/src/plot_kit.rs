//! Shared instrument-plot furniture (the quiet-plot voice): muted tick labels,
//! in-plot rich axis titles, and the uniform faint grid shared by all workspaces.

mod chrome;
mod legend;

use eframe::egui;

pub use self::chrome::{
    grid_color, muted_axis, muted_text_color, quiet_grid, tick_font, title_font, title_label,
    title_text_color, visible_y_axis_gutter, GRID_ALPHA,
};
pub use self::legend::{
    centered_legend_row, legend_entry, legend_entry_width, legend_label, legend_label_width,
    legend_row_width, legend_swatch, LegendMark,
};

pub const PLOT_FOOTER_RESERVE: f32 = 26.0;
pub const MIN_PLOT_BODY_H: f32 = 120.0;

pub fn plot_body_height(available: f32) -> f32 {
    (available - PLOT_FOOTER_RESERVE).max(MIN_PLOT_BODY_H)
}

/// Two endpoints `[[x_lo, y_lo], [x_hi, y_hi]]` of the line `y = slope*x +
/// intercept` across `[x_lo, x_hi]` (a 2-point extrapolation, NOT clipped to any
/// fit window). `None` if either `y` is non-finite (e.g. a NaN fit). Shared by
/// the selector graphs (VT sqrt space, SS log10 space — the fitter's y is
/// already transformed) and the TLM R_total-vs-L plot.
pub fn fit_line_endpoints(
    slope: f64,
    intercept: f64,
    x_lo: f64,
    x_hi: f64,
) -> Option<[[f64; 2]; 2]> {
    let y_lo = slope * x_lo + intercept;
    let y_hi = slope * x_hi + intercept;
    if y_lo.is_finite() && y_hi.is_finite() {
        Some([[x_lo, y_lo], [x_hi, y_hi]])
    } else {
        None
    }
}

/// A "nice" linear axis step (1/2/5 × 10^k) capped at six labelled ticks across
/// `span`, so a compact value axis gets readable round ticks instead of either
/// egui_plot's default single-label collapse or an over-dense grid whose labels
/// are suppressed. `span` is the data range; the result is the tick pitch.
pub fn nice_axis_step(span: f64) -> f64 {
    let raw = (span / 6.0).max(f64::MIN_POSITIVE);
    let mut step = nice_step_from_raw(raw);
    while (span / step).floor() as usize + 1 > 6 {
        step = next_nice_step(step);
    }
    step
}

fn nice_step_from_raw(raw: f64) -> f64 {
    let mag = 10f64.powf(raw.log10().floor());
    let norm = raw / mag;
    mag * if norm < 1.5 {
        1.0
    } else if norm < 3.0 {
        2.0
    } else if norm < 7.0 {
        5.0
    } else {
        10.0
    }
}

fn next_nice_step(step: f64) -> f64 {
    let mag = 10f64.powf(step.log10().floor());
    let norm = step / mag;
    if norm < 1.5 {
        2.0 * mag
    } else if norm < 3.5 {
        5.0 * mag
    } else {
        10.0 * mag
    }
}

/// Integer-multiple grid marks of `step` across `[lo, hi]` — the tick set for a
/// value axis whose pitch came from [`nice_axis_step`]. Empty if `step` is
/// non-finite or non-positive.
pub fn grid_marks(lo: f64, hi: f64, step: f64) -> Vec<egui_plot::GridMark> {
    if !step.is_finite() || step <= 0.0 {
        return Vec::new();
    }
    let k0 = (lo / step).ceil() as i64;
    let k1 = (hi / step).floor() as i64;
    (k0..=k1)
        .map(|k| egui_plot::GridMark {
            value: k as f64 * step,
            step_size: step,
        })
        .collect()
}

pub fn numeric_tick_label(show: bool, value: f64) -> String {
    if show {
        crate::format_ui::fmt_num3(value)
    } else {
        String::new()
    }
}

pub fn engineering_tick_label(show: bool, value: f64) -> String {
    if show {
        crate::format_ui::eng_tick(value)
    } else {
        String::new()
    }
}

/// Label every `step`-th decade so a dense log axis shows no more than roughly
/// six labels. Every decade still gets a grid line and tick mark.
pub fn decade_label_step(lo_decade: i64, hi_decade: i64) -> i64 {
    let count = (hi_decade - lo_decade).max(0) + 1;
    ((count + 5) / 6).max(1)
}

pub fn log_decade_tick_label(show: bool, value: f64, step: i64) -> String {
    if !show || step <= 0 {
        return String::new();
    }
    let n = value.round() as i64;
    if (value - n as f64).abs() <= 1.0e-9 && n % step == 0 {
        format!("1e{n}")
    } else {
        String::new()
    }
}

pub fn data_tooltip(name: &str, rows: &[(&str, String)]) -> String {
    let mut lines = Vec::with_capacity(rows.len() + usize::from(!name.is_empty()));
    if !name.is_empty() {
        lines.push(name.to_string());
    }
    lines.extend(rows.iter().map(|(label, value)| format!("{label} {value}")));
    lines.join("\n")
}

/// Selector fit-window bands use a crisp plot-owned outline so the graph panel
/// does not hand-roll plot chrome.
pub const BAND_STROKE_WIDTH: f32 = 1.0;
/// Fraction of the x-span a full-range band edge is nudged inside the plot.
const FULL_RANGE_BAND_EDGE_INSET_FRACTION: f64 = 0.0125;

pub fn band_stroke(color: egui::Color32) -> egui::Stroke {
    egui::Stroke::new(BAND_STROKE_WIDTH, color)
}

/// Drawing-only window for a band whose real range touches the plot bounds.
/// A full-range auto fit otherwise paints its vertical edges exactly on the plot
/// border, where the stroke can disappear. The committed fit window is unchanged;
/// only the visual band edge is nudged just inside the graph.
pub fn visible_band_window(window: (f64, f64), x_bounds: (f64, f64)) -> (f64, f64) {
    let (mut lo, mut hi) = window;
    let (x0, x1) = x_bounds;
    let span = x1 - x0;
    if !span.is_finite() || span <= 0.0 {
        return window;
    }
    let tol = span.abs() * 1e-9;
    let inset = span * FULL_RANGE_BAND_EDGE_INSET_FRACTION;
    if (lo - x0).abs() <= tol {
        lo += inset;
    }
    if (hi - x1).abs() <= tol {
        hi -= inset;
    }
    if lo < hi {
        (lo, hi)
    } else {
        window
    }
}
