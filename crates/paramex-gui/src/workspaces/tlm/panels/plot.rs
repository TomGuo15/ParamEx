//! TLM plot: R_total-vs-L scatter for the selected group at the selected V_G, with
//! the max-current fit (dashed) and the median-current fit (dashed). All numbers come
//! from `paramex_core::tlm`; this module only shapes them for `egui_plot`.
//!
//! Conventions follow `workspaces/transfer/selector/graph.rs` (the shared `plot_kit` voice):
//! - Pinned bounds every frame via `set_plot_bounds`.
//! - Y-axis SI/engineering tick formatter (`format_ui::eng_tick`), muted 11px ticks.
//! - In-plot rich axis titles (y rotated); quiet uniform grid; hover readout.
//! - Data = solid; fits = dashed (transfer convention).
//! - Series hues from `workspaces/transfer/selector/bands` (forward = suisei-main,
//!   backward/median = suisei-dark).

use eframe::egui;
use egui_plot::{Line, LineStyle, MarkerShape, Plot, PlotBounds, PlotPoints, Points};

mod series;

use crate::format_ui::fmt_vg;
use crate::plot_kit::{self, LegendMark};
use crate::theme::{SUISEI_DARK, SUISEI_MAIN};
use crate::ui_kit;
use crate::workspaces::tlm::state::TlmState;

pub use series::{median_points, plot_bounds, scatter_points};

pub fn show(ui: &mut egui::Ui, tlm: &TlmState) {
    ui_kit::card_slot(ui, |ui| {
        let gate_metadata = tlm
            .selected_vg()
            .map(|vg| format!("V<sub>G</sub> {}", fmt_vg(vg)));
        ui_kit::section_header(ui, "FIT", gate_metadata.as_deref());
        let model = tlm.selected_group_analysis().map_or_else(
            || {
                let none = None;
                let (x_bounds, y_bounds) = series::plot_bounds(&[], &[], &[none, none]);
                series::PlotModel {
                    scatter_points: Vec::new(),
                    median_points: Vec::new(),
                    max_line: None,
                    median_line: None,
                    x_bounds,
                    y_bounds,
                }
            },
            series::plot_model,
        );
        let series::PlotModel {
            scatter_points,
            median_points,
            max_line,
            median_line,
            x_bounds,
            y_bounds,
        } = model;
        let has_plot_data = !scatter_points.is_empty() || !median_points.is_empty();

        let fwd = SUISEI_MAIN;
        let bwd = SUISEI_DARK;

        // In-plot axis titles + muted 11px ticks (the shared instrument-plot voice;
        // the y-title auto-rotates -90°).
        let base = egui::TextStyle::Body.resolve(ui.style());
        let x_hints = plot_kit::muted_axis(egui_plot::Axis::X, "L (\u{00B5}m)", base.clone())
            .formatter(move |mark: egui_plot::GridMark, _range| {
                plot_kit::numeric_tick_label(has_plot_data, mark.value)
            });
        let y_hints =
            plot_kit::muted_axis(egui_plot::Axis::Y, "R<sub>total</sub> (\u{2126})", base)
                .formatter(move |mark: egui_plot::GridMark, _range| {
                    plot_kit::engineering_tick_label(has_plot_data, mark.value)
                });

        plot_kit::quiet_grid(
            Plot::new("tlm_plot")
                // Only the legend row remains below the plot (the x-title moved inside).
                .height(plot_kit::plot_body_height(ui.available_height()))
                .allow_drag(false)
                .allow_zoom(false)
                .allow_scroll(false)
                .allow_boxed_zoom(false),
        )
        .custom_x_axes(vec![x_hints])
        .custom_y_axes(vec![y_hints])
        // Hover readout (egui_plot shows NO tooltip without a formatter):
        // app formatters, plain text (tooltips don't render markup).
        .label_formatter(|_name, p| {
            plot_kit::data_tooltip(
                "",
                &[
                    (
                        "L",
                        format!("{} \u{00B5}m", crate::format_ui::fmt_num3(p.x)),
                    ),
                    (
                        "Rtotal",
                        format!("{}\u{2126}", crate::format_ui::eng_tick(p.y)),
                    ),
                ],
            )
        })
        .show(ui, |plot_ui| {
            // Pin bounds every frame (prevents auto-zoom drift).
            plot_ui.set_plot_bounds(PlotBounds::from_min_max(
                [x_bounds[0], y_bounds[0]],
                [x_bounds[1], y_bounds[1]],
            ));
            // Z-order: fits first (below), then data on top.
            // Transfer convention: data = solid, fits = dashed.
            if let Some(l) = max_line {
                plot_ui.line(
                    Line::new("max fit", PlotPoints::from(l.to_vec()))
                        .color(fwd)
                        .width(1.5_f32)
                        .style(LineStyle::dashed_loose()),
                );
            }
            if let Some(l) = median_line {
                plot_ui.line(
                    Line::new("median fit", PlotPoints::from(l.to_vec()))
                        .color(bwd)
                        .width(1.5_f32)
                        .style(LineStyle::dashed_loose()),
                );
            }
            plot_ui.points(
                Points::new("max measured", PlotPoints::from(scatter_points))
                    .radius(3.5_f32)
                    .color(fwd),
            );
            plot_ui.points(
                Points::new("median measured", PlotPoints::from(median_points))
                    .radius(3.0_f32)
                    .shape(MarkerShape::Diamond)
                    .color(bwd),
            );
        });

        // Legend row centered below the plot (the x-title lives inside the plot now).
        // Entries pair by series: max/median, measured before fit.
        if has_plot_data {
            plot_kit::centered_legend_row(
                ui,
                &[
                    (fwd, LegendMark::Dot, "max measured"),
                    (fwd, LegendMark::DashedLine, "max fit"),
                    (bwd, LegendMark::Diamond, "median measured"),
                    (bwd, LegendMark::DashedLine, "median fit"),
                ],
            );
        } else {
            plot_kit::centered_legend_row(ui, &[]);
        }
    });
}
