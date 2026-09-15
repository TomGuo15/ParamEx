//! Shared plot title, axis, tick, and grid chrome.

use eframe::egui;

use crate::theme::{token_alpha, tokens, type_scale};

/// The muted tick-label font (the muted size, smaller than Body so the data
/// outranks the axis furniture).
pub fn tick_font() -> egui::FontId {
    egui::FontId::new(type_scale::MUTED, egui::FontFamily::Proportional)
}

/// Muted plot-furniture text: tick labels, axis titles, and legend labels all
/// sit behind the data marks with the same Utility gray voice.
pub fn muted_text_color() -> egui::Color32 {
    tokens().ink_soft
}

/// Plot panel titles sit above instrument plots and use the same strong card
/// title voice, while staying owned by the plot system rather than each caller.
pub fn title_text_color() -> egui::Color32 {
    tokens().ink
}

pub fn title_font(ui: &egui::Ui) -> egui::FontId {
    let base = egui::TextStyle::Body.resolve(ui.style());
    egui::FontId::new(base.size, crate::ui_kit::bold_family(ui))
}

pub fn title_label(ui: &mut egui::Ui, markup: &str) -> egui::Response {
    ui.label(crate::richtext::layout_sub_sup(
        markup,
        title_font(ui),
        title_text_color(),
    ))
}

/// The faint Studio Stellar grid tint shared by all instrument plots.
pub const GRID_ALPHA: u8 = 128;

pub fn grid_color() -> egui::Color32 {
    token_alpha(tokens().border, GRID_ALPHA)
}

/// One quiet axis: muted ticks + an in-plot rich title. The sub/sup-aware
/// LayoutJob carries its own ink_soft color (the axis widget's fallback text color
/// only applies to plain text); the y-title auto-rotates -90°.
pub fn muted_axis(
    axis: egui_plot::Axis,
    title_markup: &str,
    base: egui::FontId,
) -> egui_plot::AxisHints<'static> {
    let soft = muted_text_color();
    let job = crate::richtext::layout_sub_sup(title_markup, base, soft);
    egui_plot::AxisHints::new(axis)
        .label(job)
        .tick_label_color(soft)
        .tick_label_font(tick_font())
}

pub fn visible_y_axis_gutter(
    axis: egui_plot::AxisHints<'static>,
    has_tick_labels: bool,
    min_thickness: f32,
) -> egui_plot::AxisHints<'static> {
    if has_tick_labels {
        axis.min_thickness(min_thickness)
    } else {
        axis
    }
}

/// Quiet uniform grid: an explicit faint Stellar tint (a `border` alpha-tint —
/// in-group, per the palette rule), no step-keyed strength fade — every line the
/// same weight, sitting under the data and the band fills.
pub fn quiet_grid(plot: egui_plot::Plot<'_>) -> egui_plot::Plot<'_> {
    plot.show_grid(true).grid_color(grid_color()).grid_fade(0.0)
}
