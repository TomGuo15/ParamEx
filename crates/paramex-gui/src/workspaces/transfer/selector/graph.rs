//! One selector graph (egui_plot::Plot) + the pure fit-line helpers.

mod axis;

use eframe::egui;
use egui_plot::{Line, LineStyle, Plot, PlotBounds, PlotPoints, PlotResponse, Polygon};
use paramex_core::transfer::WindowedFitResult;

/// Slope epsilon matching the shared numeric floor. The preview line is
/// drawn only for a well-conditioned fit.
const SLOPE_EPSILON: f64 = 1e-12;

/// The preview fit-line gate (live-drag PREVIEW only): >=5 points AND finite,
/// non-degenerate slope AND finite intercept.
pub fn preview_gate(r: &WindowedFitResult) -> bool {
    r.points >= 5 && r.slope.is_finite() && r.slope.abs() > SLOPE_EPSILON && r.intercept.is_finite()
}

/// The post-commit DISPLAYED fit-line gate — deliberately WEAKER than `preview_gate`
/// (the oracle's full-figure fit draws on finite slope/intercept + >=2 points; the
/// strict >=5/slope-eps gate is preview-only). Use this for the committed render path.
pub fn committed_line_gate(r: &WindowedFitResult) -> bool {
    r.slope.is_finite() && r.intercept.is_finite() && r.points >= 2
}

/// One band to draw: window + fill/stroke. Bands and data are always solid;
/// only the fit line dashes — see `plot_kit::fit_line_endpoints` for the
/// shared endpoint math.
pub struct BandDraw {
    pub window: (f64, f64),
    pub fill: egui::Color32,
    pub stroke: egui::Color32,
}

/// One branch data series to draw in the plot.
pub struct SeriesDraw<'a> {
    pub name: &'static str,
    pub points: &'a [[f64; 2]],
    pub color: egui::Color32,
}

/// Static plot configuration shared by one selector graph render.
pub struct GraphConfig<'a> {
    pub id_source: &'a str,
    pub title: &'a str,
    pub x_label: &'a str,
    pub y_label: &'a str,
    pub x_bounds: (f64, f64),
    pub y_bounds: [f64; 2],
    pub y_log: bool,
    pub height: f32,
    pub show_scale_values: bool,
}

/// Render one selector graph and return its PlotResponse for hit-testing.
/// `series`/`fit` are owned `Vec<[f64;2]>` from the snapshot; Line/Polygon are built
/// FRESH here (they are lifetime-bound to the closure). `height` lets the caller size
/// the plot to fill the card. There is no legend because the direction toggle is
/// color-coded; the rich-text x/y axis titles render inside the plot.
pub fn render_graph(
    ui: &mut egui::Ui,
    config: GraphConfig<'_>,
    series: &[SeriesDraw<'_>],
    fit: Option<([[f64; 2]; 2], egui::Color32)>,
    bands: &[BandDraw],
) -> PlotResponse<()> {
    let GraphConfig {
        id_source,
        title,
        x_label,
        y_label,
        x_bounds,
        y_bounds,
        y_log,
        height,
        show_scale_values,
    } = config;
    // Title may carry <sub>/<sup> markup (e.g. "V<sub>TH</sub> fit window") — render it
    // through richtext so the subscript shows (was plain RichText → "VTH").
    crate::plot_kit::title_label(ui, title);
    // In-plot axis titles + muted 11px ticks (the shared instrument-plot voice;
    // the y-title auto-rotates -90°).
    let base = egui::TextStyle::Body.resolve(ui.style());
    let x_hints = axis::x_axis(x_label, base.clone(), show_scale_values);
    let y_hints = axis::y_axis(y_label, base, y_log, y_bounds, show_scale_values);
    let xstep = crate::plot_kit::nice_axis_step(x_bounds.1 - x_bounds.0);
    let plot = axis::with_y_grid_spacer(
        crate::plot_kit::quiet_grid(
            Plot::new(id_source)
                .height(height)
                .sense(egui::Sense::click_and_drag())
                .allow_drag(false)
                .allow_zoom(false)
                .allow_scroll(false)
                .allow_boxed_zoom(false),
        ),
        y_log,
        y_bounds,
    )
    .x_grid_spacer(move |_input| crate::plot_kit::grid_marks(x_bounds.0, x_bounds.1, xstep))
    // The hover readout (egui_plot shows NO tooltip without a formatter):
    // app formatters, plain text (tooltips don't render markup).
    .label_formatter(move |_name, p| {
        if show_scale_values {
            axis::hover_label(y_log, p.x, p.y)
        } else {
            String::new()
        }
    })
    .custom_x_axes(vec![x_hints])
    .custom_y_axes(vec![y_hints]);
    let disabled = !ui.is_enabled();
    let muted = crate::theme::tokens().ink_soft;
    let mark_color = |color: egui::Color32| if disabled { muted } else { color };
    let fill_color = |color: egui::Color32| {
        if disabled {
            crate::theme::token_alpha(muted, color.a())
        } else {
            color
        }
    };
    let response = plot.show(ui, |plot_ui| {
        // Pin a fixed frame EVERY frame (overrides any stray pan/zoom).
        plot_ui.set_plot_bounds(PlotBounds::from_min_max(
            [x_bounds.0, y_bounds[0]],
            [x_bounds.1, y_bounds[1]],
        ));
        // Read the LIVE y-range AFTER pinning for full-height bands.
        let yr = plot_ui.plot_bounds().range_y();
        let (y0, y1) = (*yr.start(), *yr.end());
        // z-order: bands first, then scatter, then dashed fit on top.
        for b in bands {
            let visible_window = crate::plot_kit::visible_band_window(b.window, x_bounds);
            plot_ui.polygon(
                Polygon::new(
                    "band",
                    PlotPoints::from(super::bands::band_rect_points(
                        visible_window.0,
                        visible_window.1,
                        y0,
                        y1,
                    )),
                )
                .fill_color(fill_color(b.fill))
                .stroke(crate::plot_kit::band_stroke(mark_color(b.stroke))),
            );
        }
        for s in series {
            plot_ui.line(
                Line::new(s.name, PlotPoints::from(s.points.to_vec()))
                    .color(mark_color(s.color))
                    .width(1.5_f32),
            );
        }
        if let Some((fp, color)) = fit {
            plot_ui.line(
                Line::new("fit", PlotPoints::from(fp.to_vec()))
                    .color(mark_color(color))
                    .width(1.5_f32)
                    .style(LineStyle::dashed_loose()),
            );
        }
    });
    // No legend row: the forward/backward direction toggle is color-coded to the
    // line colors, which carries the same information. Axis titles render inside the
    // plot (AxisHints), so nothing is hand-painted below it.
    response
}
