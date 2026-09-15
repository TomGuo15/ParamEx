//! Selector graph axis and hover-label policy.

use eframe::egui;
use egui_plot::{Axis, AxisHints, GridMark, Plot};

use crate::workspaces::transfer::layout;

pub(super) fn x_axis(
    title_markup: &str,
    base: egui::FontId,
    show_scale_values: bool,
) -> AxisHints<'static> {
    let hints = crate::plot_kit::muted_axis(Axis::X, title_markup, base)
        .label_spacing(layout::x_axis_label_spacing());
    if show_scale_values {
        hints
    } else {
        hints.formatter(|_mark: GridMark, _range| String::new())
    }
}

pub(super) fn y_axis(
    title_markup: &str,
    base: egui::FontId,
    y_log: bool,
    y_bounds: [f64; 2],
    show_scale_values: bool,
) -> AxisHints<'static> {
    let mut hints = crate::plot_kit::muted_axis(Axis::Y, title_markup, base);
    if !show_scale_values {
        hints = hints.formatter(|_mark: GridMark, _range| String::new());
    } else if y_log {
        let step = crate::plot_kit::decade_label_step(
            y_bounds[0].ceil() as i64,
            y_bounds[1].floor() as i64,
        );
        hints = hints
            .formatter(move |mark: GridMark, _range| {
                crate::plot_kit::log_decade_tick_label(show_scale_values, mark.value, step)
            })
            .label_spacing(egui::Rangef::new(10.0, 16.0));
    } else {
        // VT (sqrt) axis values are tiny, so engineering/SI ticks read better
        // than plain decimals.
        hints = hints.formatter(|mark: GridMark, _range| {
            crate::plot_kit::engineering_tick_label(true, mark.value)
        });
    }
    hints.min_thickness(layout::Y_AXIS_MIN_THICKNESS)
}

pub(super) fn with_y_grid_spacer<'a>(plot: Plot<'a>, y_log: bool, y_bounds: [f64; 2]) -> Plot<'a> {
    if !y_log {
        return plot;
    }

    // SS uses log10|Id| internally: a grid line at every integer decade, labels on a
    // sparse subset so the axis reads as a scale, not a wall of text. Marks are
    // generated from the known pinned y_bounds, not the pre-set_plot_bounds
    // input range.
    let (yb0, yb1) = (y_bounds[0], y_bounds[1]);
    let step = crate::plot_kit::decade_label_step(yb0.ceil() as i64, yb1.floor() as i64);
    plot.y_grid_spacer(move |_input| {
        let lo = yb0.ceil() as i64;
        let hi = yb1.floor() as i64;
        // Report the labeled pitch as step_size so egui_plot preserves decade
        // grid lines on short plots while the formatter decides which decades
        // carry labels.
        (lo..=hi)
            .map(|n| GridMark {
                value: n as f64,
                step_size: step as f64,
            })
            .collect()
    })
}

pub(super) fn hover_label(y_log: bool, x: f64, y: f64) -> String {
    if y_log {
        crate::plot_kit::data_tooltip(
            "",
            &[
                ("Vg", format!("{} V", crate::format_ui::fmt_num3(x))),
                (
                    "|Id|",
                    format!("{}A", crate::format_ui::eng_tick(10f64.powf(y))),
                ),
            ],
        )
    } else {
        crate::plot_kit::data_tooltip(
            "",
            &[
                ("Vg", format!("{} V", crate::format_ui::fmt_num3(x))),
                (
                    "\u{221A}|Id|",
                    format!("{}\u{221A}A", crate::format_ui::eng_tick(y)),
                ),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::hover_label;

    #[test]
    fn hover_labels_report_physical_current_units() {
        let subthreshold = hover_label(true, 1.5, -9.0);
        assert!(subthreshold.contains("|Id| 1nA"));
        assert!(!subthreshold.contains("log"));

        let threshold = hover_label(false, 1.5, 0.002);
        assert!(threshold.contains("\u{221A}|Id| 2m\u{221A}A"));
    }
}
