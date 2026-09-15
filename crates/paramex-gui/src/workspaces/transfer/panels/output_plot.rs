//! Selected Transfer file's measured Id-Vd output curves.

use eframe::egui;
use egui_plot::{Axis, GridMark, Line, Plot, PlotBounds, PlotPoints, Polygon};
use paramex_core::transfer::{OutputDataset, Session};

use crate::format_ui::{
    eng_tick, fmt_num3, output_partial_fit_message, OUTPUT_NO_FINITE_POINTS_MESSAGE,
    OUTPUT_SUMMARY_UNAVAILABLE_MESSAGE,
};
use crate::plot_kit;
use crate::state::EditBuffers;
use crate::theme::{SUISEI_LIGHT, SUISEI_MAIN};
use crate::ui_kit::{self, BadgeTone, Variant};
use crate::workspaces::transfer::layout;
use crate::workspaces::transfer::selector::{bands, strip};

// Range strip, numeric row, and their gap. No legend/footer follows the fields.
const OUTPUT_CONTROLS_RESERVE: f32 = 53.0;
const OUTPUT_SUBPLOT_CAPTION_H: f32 = 18.0;

#[derive(Debug, Clone, PartialEq)]
pub struct OutputPlotSeries {
    pub vg: f64,
    pub points: Vec<[f64; 2]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutputPlotModel {
    pub series: Vec<OutputPlotSeries>,
    pub x_bounds: (f64, f64),
    pub y_bounds: (f64, f64),
}

#[derive(Debug, Clone, PartialEq)]
struct TransferPlotModel {
    points: Vec<[f64; 2]>,
    x_bounds: (f64, f64),
    y_bounds: (f64, f64),
}

/// Cached transfer + output plot models, keyed by session generation and file.
pub struct OutputPlotCache {
    generation: u64,
    file_id: Option<String>,
    transfer: TransferPlotModel,
    output: OutputPlotModel,
}

impl Default for OutputPlotCache {
    fn default() -> Self {
        Self {
            generation: u64::MAX,
            file_id: None,
            transfer: empty_transfer_model(),
            output: empty_output_model(),
        }
    }
}

impl OutputPlotCache {
    fn refresh(&mut self, session: &Session) {
        let selected = session.selected_output_file();
        let file_id = selected
            .as_ref()
            .map(|selected| selected.file_id.to_owned());
        let generation = session.generation();
        if self.generation == generation && self.file_id == file_id {
            return;
        }
        self.generation = generation;
        self.file_id = file_id;
        match selected {
            Some(selected) => {
                self.transfer = transfer_plot_model(selected.transfer_vg, selected.transfer_id_abs);
                self.output = selected
                    .output
                    .and_then(plot_model)
                    .unwrap_or_else(empty_output_model);
            }
            None => {
                self.transfer = empty_transfer_model();
                self.output = empty_output_model();
            }
        }
    }
}

pub fn plot_model(output: &OutputDataset) -> Option<OutputPlotModel> {
    let mut series = Vec::new();
    let (mut xmin, mut xmax) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut ymin, mut ymax) = (f64::INFINITY, f64::NEG_INFINITY);

    for curve in &output.curves {
        let mut points = Vec::new();
        for (&vd, &id) in curve.vd.iter().zip(curve.id.iter()) {
            if vd.is_finite() && id.is_finite() {
                let id = id.abs();
                xmin = xmin.min(vd);
                xmax = xmax.max(vd);
                ymin = ymin.min(id);
                ymax = ymax.max(id);
                points.push([vd, id]);
            }
        }
        if !points.is_empty() {
            series.push(OutputPlotSeries {
                vg: curve.vg,
                points,
            });
        }
    }

    if series.is_empty() {
        return None;
    }
    series.sort_by(|a, b| a.vg.total_cmp(&b.vg));

    Some(OutputPlotModel {
        series,
        x_bounds: padded_bounds(xmin, xmax),
        y_bounds: positive_padded_bounds(ymin, ymax),
    })
}

fn transfer_plot_model(vg: &[f64], id_abs: &[f64]) -> TransferPlotModel {
    let mut points = Vec::new();
    let (mut xmin, mut xmax) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut ymin, mut ymax) = (f64::INFINITY, f64::NEG_INFINITY);
    for (&vg, &id) in vg.iter().zip(id_abs.iter()) {
        if vg.is_finite() && id.is_finite() && id > 0.0 {
            let y = id.log10();
            xmin = xmin.min(vg);
            xmax = xmax.max(vg);
            ymin = ymin.min(y);
            ymax = ymax.max(y);
            points.push([vg, y]);
        }
    }
    TransferPlotModel {
        points,
        x_bounds: padded_tight_bounds(xmin, xmax),
        y_bounds: padded_tight_bounds(ymin, ymax),
    }
}

pub fn show(
    ui: &mut egui::Ui,
    session: &mut Session,
    edits: &mut EditBuffers,
    cache: &mut OutputPlotCache,
) {
    ui_kit::card_slot(ui, |ui| {
        let (selected_id, manual_range, warning) = {
            let selected = session.selected_output_file();
            let selected_id = selected
                .as_ref()
                .map(|selected| selected.file_id.to_owned());
            let manual_range = selected
                .as_ref()
                .is_some_and(|selected| selected.selected_fit_range.is_some());
            let warning = selected.as_ref().and_then(|selected| {
                let output = selected.output?;
                let has_finite_points = output.curves.iter().any(|curve| {
                    curve
                        .vd
                        .iter()
                        .zip(&curve.id)
                        .any(|(vd, id)| vd.is_finite() && id.is_finite())
                });
                if !has_finite_points {
                    Some(OUTPUT_NO_FINITE_POINTS_MESSAGE.to_owned())
                } else {
                    match selected.summary.as_ref() {
                        None => Some(OUTPUT_SUMMARY_UNAVAILABLE_MESSAGE.to_owned()),
                        Some(summary) if summary.fitted_lines < summary.total_lines => Some(
                            output_partial_fit_message(summary.fitted_lines, summary.total_lines),
                        ),
                        Some(_) => None,
                    }
                }
            });
            (selected_id, manual_range, warning)
        };
        let reset_clicked = ui_kit::header_action_row(ui, "OUTPUT", |ui| {
            let clicked = ui
                .add_enabled_ui(manual_range, |ui| {
                    ui_kit::header_action(ui, "Reset to Auto", Variant::Secondary).clicked()
                })
                .inner;
            if let Some(message) = warning.as_deref() {
                ui_kit::muted_label(ui, message);
                ui_kit::semantic_badge(ui, "warn", BadgeTone::Warning);
            }
            clicked
        });
        let mut commit_range = None;
        cache.refresh(session);
        if let Some(file_id) = selected_id.as_deref() {
            if reset_clicked {
                edits.forget(&format!("out:{file_id}:vds:lo"));
                edits.forget(&format!("out:{file_id}:vds:hi"));
                session.set_output_fit_range(file_id, None);
            }

            if let Some(selected) = session.selected_output_file() {
                let transfer_model = &cache.transfer;
                let model = &cache.output;
                if let Some(output) = selected.output {
                    let has_plot_data = !model.series.is_empty();
                    let control_range = output_control_range(
                        output,
                        selected.selected_fit_range,
                        selected
                            .summary
                            .as_ref()
                            .and_then(|summary| summary.fit_range),
                    );
                    let active_range = control_range.filter(|_| has_plot_data);
                    draw_plots(ui, transfer_model, model, active_range);
                    if let Some(range) = active_range {
                        let axis = output_control_axis(output, range);
                        fit_range_controls(
                            ui,
                            edits,
                            Some(file_id),
                            axis,
                            Some(range),
                            &mut commit_range,
                        );
                    } else {
                        fit_range_controls(ui, edits, Some(file_id), None, None, &mut commit_range);
                    }
                } else {
                    draw_plots(ui, transfer_model, model, None);
                    fit_range_controls(ui, edits, Some(file_id), None, None, &mut commit_range);
                }
            } else {
                draw_plots(ui, &empty_transfer_model(), &empty_output_model(), None);
                fit_range_controls(ui, edits, None, None, None, &mut commit_range);
            }
        } else {
            draw_plots(ui, &empty_transfer_model(), &empty_output_model(), None);
            fit_range_controls(ui, edits, None, None, None, &mut commit_range);
        }

        if let (Some(file_id), Some(range)) = (selected_id.as_deref(), commit_range) {
            session.set_output_fit_range(file_id, range);
        }
    });
}

fn empty_output_model() -> OutputPlotModel {
    OutputPlotModel {
        series: Vec::new(),
        x_bounds: (0.0, 1.0),
        y_bounds: (0.0, 1.0),
    }
}

fn empty_transfer_model() -> TransferPlotModel {
    TransferPlotModel {
        points: Vec::new(),
        x_bounds: (0.0, 1.0),
        y_bounds: (0.0, 1.0),
    }
}

fn output_control_range(
    output: &OutputDataset,
    selected_fit_range: Option<(f64, f64)>,
    automatic_fit_range: Option<(f64, f64)>,
) -> Option<(f64, f64)> {
    selected_fit_range
        .or(automatic_fit_range)
        .or_else(|| vd_bounds(output))
}

fn fit_range_controls(
    ui: &mut egui::Ui,
    edits: &mut EditBuffers,
    file_id: Option<&str>,
    axis: Option<(f64, f64)>,
    range: Option<(f64, f64)>,
    commit_range: &mut Option<Option<(f64, f64)>>,
) {
    let enabled = range.is_some();
    let empty_prefix = file_id
        .map(|id| format!("out:{id}:empty"))
        .unwrap_or_else(|| "out:empty".to_string());
    let strip_id = file_id
        .filter(|_| enabled)
        .map(|id| format!("out_strip:{id}"))
        .unwrap_or_else(|| format!("{empty_prefix}:strip"));
    let lo_key = file_id
        .filter(|_| enabled)
        .map(|id| format!("out:{id}:vds:lo"))
        .unwrap_or_else(|| format!("{empty_prefix}:lo"));
    let hi_key = file_id
        .filter(|_| enabled)
        .map(|id| format!("out:{id}:vds:hi"))
        .unwrap_or_else(|| format!("{empty_prefix}:hi"));
    let axis = axis.or_else(|| (!enabled).then_some((0.0, 1.0)));
    let range = range.unwrap_or((0.0, 1.0));
    let (mut lo, mut hi) = if range.0 <= range.1 {
        range
    } else {
        (range.1, range.0)
    };
    ui.add_enabled_ui(enabled, |ui| {
        if let Some((axis_lo, axis_hi)) = axis {
            let drag = strip::double_thumb_strip(
                ui,
                strip::StripInputs {
                    id_salt: &strip_id,
                    v_min: axis_lo,
                    v_max: axis_hi,
                    lo: &mut lo,
                    hi: &mut hi,
                    accent: SUISEI_MAIN,
                },
            );
            if drag.dragging || drag.released {
                *commit_range = Some(Some((lo, hi)));
            }
        }
        ui_kit::terminal_numeric_row(ui, |ui| {
            ui.spacing_mut().item_spacing.x = ui_kit::INPUT_LABEL_GAP;
            ui_kit::field_label_rich(ui, "V<sub>D</sub> min");
            edit_vds_edge(ui, edits, &lo_key, enabled.then_some(lo), |v| {
                *commit_range = Some(Some((v, hi)));
            });
            ui_kit::field_label_rich(ui, "V<sub>D</sub> max");
            edit_vds_edge(ui, edits, &hi_key, enabled.then_some(hi), |v| {
                *commit_range = Some(Some((lo, v)));
            });
        });
    });
}

fn output_control_axis(output: &OutputDataset, range: (f64, f64)) -> Option<(f64, f64)> {
    let (mut lo, mut hi) = vd_bounds(output)?;
    lo = lo.min(range.0).min(range.1);
    hi = hi.max(range.0).max(range.1);
    (lo != hi).then_some((lo, hi))
}

fn edit_vds_edge(
    ui: &mut egui::Ui,
    edits: &mut EditBuffers,
    key: &str,
    current: Option<f64>,
    mut on_commit: impl FnMut(f64),
) {
    let current_str = current.map(fmt_num3).unwrap_or_default();
    if let Some(text) = ui_kit::singleline_edit_commit(
        ui,
        edits,
        key,
        &current_str,
        ui_kit::COMPACT_NUMERIC_INPUT_WIDTH,
    ) {
        if let Ok(v) = text.trim().parse::<f64>() {
            on_commit(v);
        }
    }
}

fn vd_bounds(output: &OutputDataset) -> Option<(f64, f64)> {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for curve in &output.curves {
        for &vd in &curve.vd {
            if vd.is_finite() {
                lo = lo.min(vd);
                hi = hi.max(vd);
            }
        }
    }
    (lo.is_finite() && hi.is_finite() && lo != hi).then_some((lo, hi))
}

fn draw_plots(
    ui: &mut egui::Ui,
    transfer: &TransferPlotModel,
    output: &OutputPlotModel,
    fit_range: Option<(f64, f64)>,
) {
    let gate_metadata = gate_voltage_metadata(output);
    if crate::workspaces::transfer::plot_pair_should_stack(ui.available_size()) {
        let pair_h = (ui.available_height() - OUTPUT_CONTROLS_RESERVE).max(0.0);
        crate::workspaces::transfer::show_stacked_plot_pair(
            ui,
            "transfer_output_plot_pair",
            pair_h,
            |ui, index| {
                let plot_h = (ui.available_height()
                    - OUTPUT_SUBPLOT_CAPTION_H
                    - ui.spacing().item_spacing.y)
                    .max(0.0);
                if index == 0 {
                    subplot_caption(ui, "Transfer", None);
                    draw_transfer_plot(ui, transfer, plot_h);
                } else {
                    subplot_caption(ui, "Output", gate_metadata.as_deref());
                    draw_output_plot(ui, output, fit_range, plot_h);
                }
            },
        );
    } else {
        let plot_h = plot_height(
            (ui.available_height() - OUTPUT_SUBPLOT_CAPTION_H - ui.spacing().item_spacing.y)
                .max(0.0),
        );
        let gap = ui.spacing().item_spacing.x;
        let plot_w = ((ui.available_width() - gap) / 2.0).max(0.0);
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(plot_w, plot_h + OUTPUT_SUBPLOT_CAPTION_H),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    subplot_caption(ui, "Transfer", None);
                    draw_transfer_plot(ui, transfer, plot_h);
                },
            );
            ui.allocate_ui_with_layout(
                egui::vec2(plot_w, plot_h + OUTPUT_SUBPLOT_CAPTION_H),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    subplot_caption(ui, "Output", gate_metadata.as_deref());
                    draw_output_plot(ui, output, fit_range, plot_h);
                },
            );
        });
    }
}

fn gate_voltage_metadata(model: &OutputPlotModel) -> Option<String> {
    let first = model.series.first()?.vg;
    let last = model.series.last()?.vg;
    Some(if first == last {
        format!("V<sub>G</sub> {} V", fmt_num3(first))
    } else {
        format!(
            "V<sub>G</sub> {} \u{2192} {} V",
            fmt_num3(first),
            fmt_num3(last)
        )
    })
}

fn subplot_caption(ui: &mut egui::Ui, text: &str, metadata: Option<&str>) {
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), OUTPUT_SUBPLOT_CAPTION_H),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            plot_kit::title_label(ui, text);
            if let Some(metadata) = metadata {
                ui_kit::right_aligned(ui, |ui| {
                    ui_kit::field_label_rich(ui, metadata);
                });
            }
        },
    );
}

fn draw_transfer_plot(ui: &mut egui::Ui, model: &TransferPlotModel, height: f32) {
    let (xlo, xhi) = model.x_bounds;
    let (ylo, yhi) = model.y_bounds;
    let has_plot_data = !model.points.is_empty();
    let base = egui::TextStyle::Body.resolve(ui.style());
    let xstep = plot_kit::nice_axis_step(xhi - xlo);
    let ystep = plot_kit::nice_axis_step(yhi - ylo);
    let x_hints = plot_kit::muted_axis(Axis::X, "V<sub>G</sub> (V)", base.clone())
        .formatter(move |mark: GridMark, _range| {
            plot_kit::numeric_tick_label(has_plot_data, mark.value)
        })
        .label_spacing(layout::x_axis_label_spacing());
    let y_hints = plot_kit::muted_axis(Axis::Y, "|I<sub>D</sub>| (A)", base)
        .formatter(move |mark: GridMark, _range| {
            plot_kit::log_decade_tick_label(has_plot_data, mark.value, 1)
        })
        .min_thickness(layout::Y_AXIS_MIN_THICKNESS);

    plot_kit::quiet_grid(
        Plot::new("transfer_output_transfer_plot")
            .height(height)
            .allow_drag(false)
            .allow_zoom(false)
            .allow_scroll(false)
            .allow_boxed_zoom(false),
    )
    .custom_x_axes(vec![x_hints])
    .custom_y_axes(vec![y_hints])
    .x_grid_spacer(move |_input| plot_kit::grid_marks(xlo, xhi, xstep))
    .y_grid_spacer(move |_input| plot_kit::grid_marks(ylo, yhi, ystep))
    .label_formatter(|_name, p| {
        plot_kit::data_tooltip(
            "",
            &[
                ("Vg", format!("{} V", fmt_num3(p.x))),
                ("|Id|", format!("{}A", eng_tick(10f64.powf(p.y)))),
            ],
        )
    })
    .show(ui, |plot_ui| {
        plot_ui.set_plot_bounds(PlotBounds::from_min_max([xlo, ylo], [xhi, yhi]));
        plot_ui.line(
            Line::new("transfer", PlotPoints::from(model.points.clone()))
                .color(SUISEI_MAIN)
                .width(1.6_f32),
        );
    });
}

fn draw_output_plot(
    ui: &mut egui::Ui,
    model: &OutputPlotModel,
    fit_range: Option<(f64, f64)>,
    height: f32,
) {
    let (xlo, xhi) = model.x_bounds;
    let (ylo, yhi) = model.y_bounds;
    let has_plot_data = !model.series.is_empty();
    let base = egui::TextStyle::Body.resolve(ui.style());
    let xstep = plot_kit::nice_axis_step(xhi - xlo);
    let ystep = plot_kit::nice_axis_step(yhi - ylo);
    let x_hints = plot_kit::muted_axis(Axis::X, "V<sub>D</sub> (V)", base.clone())
        .formatter(move |mark: GridMark, _range| {
            plot_kit::numeric_tick_label(has_plot_data, mark.value)
        })
        .label_spacing(layout::x_axis_label_spacing());
    let y_hints = plot_kit::muted_axis(Axis::Y, "|I<sub>D</sub>| (A)", base)
        .formatter(move |mark: GridMark, _range| {
            plot_kit::engineering_tick_label(has_plot_data, mark.value)
        })
        .min_thickness(layout::Y_AXIS_MIN_THICKNESS);

    plot_kit::quiet_grid(
        Plot::new("transfer_output_curves_plot")
            .height(height)
            .allow_drag(false)
            .allow_zoom(false)
            .allow_scroll(false)
            .allow_boxed_zoom(false),
    )
    .custom_x_axes(vec![x_hints])
    .custom_y_axes(vec![y_hints])
    .x_grid_spacer(move |_input| plot_kit::grid_marks(xlo, xhi, xstep))
    .y_grid_spacer(move |_input| plot_kit::grid_marks(ylo, yhi, ystep))
    .label_formatter(|name, p| {
        plot_kit::data_tooltip(
            name,
            &[
                ("Vd", format!("{} V", fmt_num3(p.x))),
                ("|Id|", format!("{}A", eng_tick(p.y))),
            ],
        )
    })
    .show(ui, |plot_ui| {
        plot_ui.set_plot_bounds(PlotBounds::from_min_max([xlo, ylo], [xhi, yhi]));
        if let Some(range) = fit_range {
            let (lo, hi) = if range.0 <= range.1 {
                range
            } else {
                (range.1, range.0)
            };
            let visible = plot_kit::visible_band_window((lo, hi), (xlo, xhi));
            plot_ui.polygon(
                Polygon::new(
                    "V_D fit range",
                    PlotPoints::from(bands::band_rect_points(visible.0, visible.1, ylo, yhi)),
                )
                .fill_color(bands::forward_fill())
                .stroke(plot_kit::band_stroke(SUISEI_MAIN)),
            );
        }
        let count = model.series.len().max(1);
        for (idx, series) in model.series.iter().enumerate() {
            let frac = gate_frac(idx, count);
            let label = format!("V_G {} V", fmt_num3(series.vg));
            plot_ui.line(
                Line::new(label, PlotPoints::from(series.points.clone()))
                    .color(gate_ramp(frac))
                    .width(1.6_f32),
            );
        }
    });
}

fn plot_height(available: f32) -> f32 {
    (available - OUTPUT_CONTROLS_RESERVE).max(0.0)
}

fn gate_frac(idx: usize, count: usize) -> f64 {
    if count <= 1 {
        1.0
    } else {
        idx as f64 / (count - 1) as f64
    }
}

fn padded_bounds(min: f64, max: f64) -> (f64, f64) {
    if !min.is_finite() || !max.is_finite() {
        return (0.0, 1.0);
    }
    let anchor_lo = min >= 0.0;
    let anchor_hi = max <= 0.0;
    let mut lo = if anchor_lo { 0.0 } else { min };
    let mut hi = if anchor_hi { 0.0 } else { max };
    let span = hi - lo;

    if !span.is_finite() || span <= 0.0 {
        let pad = max.abs().max(min.abs()).max(1.0) * 0.05;
        return (lo - pad, hi + pad);
    }

    let pad = span * 0.04;
    if !anchor_lo {
        lo -= pad;
    }
    if !anchor_hi {
        hi += pad;
    }
    (lo, hi)
}

fn padded_tight_bounds(min: f64, max: f64) -> (f64, f64) {
    if !min.is_finite() || !max.is_finite() {
        return (0.0, 1.0);
    }
    let span = max - min;
    if !span.is_finite() || span <= 0.0 {
        let pad = max.abs().max(min.abs()).max(1.0) * 0.05;
        return (min - pad, max + pad);
    }
    let pad = span * 0.04;
    (min - pad, max + pad)
}

fn positive_padded_bounds(min: f64, max: f64) -> (f64, f64) {
    let (lo, hi) = padded_bounds(min, max);
    (lo.max(0.0), hi)
}

fn gate_ramp(frac: f64) -> egui::Color32 {
    let t = (0.35 + 0.65 * frac.clamp(0.0, 1.0)) as f32;
    let lerp = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    egui::Color32::from_rgb(
        lerp(SUISEI_LIGHT.r(), SUISEI_MAIN.r()),
        lerp(SUISEI_LIGHT.g(), SUISEI_MAIN.g()),
        lerp(SUISEI_LIGHT.b(), SUISEI_MAIN.b()),
    )
}

#[cfg(test)]
mod tests {
    use paramex_core::transfer::OutputCurve;

    use super::*;

    #[test]
    fn single_output_curve_uses_primary_blue() {
        assert_eq!(gate_ramp(gate_frac(0, 1)), SUISEI_MAIN);
        assert_eq!(gate_frac(0, 2), 0.0);
        assert_eq!(gate_frac(1, 2), 1.0);
    }

    #[test]
    fn output_current_bounds_never_go_below_zero() {
        let output = OutputDataset {
            name: "zero-current.csv".to_owned(),
            curves: vec![OutputCurve {
                vg: 0.0,
                vd: vec![0.0, 1.0],
                id: vec![0.0, 0.0],
            }],
            source_path: None,
        };
        let model = plot_model(&output).expect("finite output points");
        assert_eq!(model.y_bounds.0, 0.0);
        assert!(model.y_bounds.1 > 0.0);
    }

    #[test]
    fn output_current_bounds_preserve_sub_femtoampere_scale() {
        let output = OutputDataset {
            name: "small-current.csv".to_owned(),
            curves: vec![OutputCurve {
                vg: 0.0,
                vd: vec![0.0, 1.0],
                id: vec![0.0, 1.0e-20],
            }],
            source_path: None,
        };
        let model = plot_model(&output).expect("finite output points");
        assert_eq!(model.y_bounds.0, 0.0);
        assert!(model.y_bounds.1 > 1.0e-20);
        assert!(model.y_bounds.1 < 1.1e-20);
    }

    #[test]
    fn gate_voltage_metadata_describes_the_sorted_color_order() {
        let series = |vg| OutputPlotSeries {
            vg,
            points: Vec::new(),
        };
        let model = |series| OutputPlotModel {
            series,
            x_bounds: (0.0, 1.0),
            y_bounds: (0.0, 1.0),
        };

        assert_eq!(gate_voltage_metadata(&model(Vec::new())), None);
        assert_eq!(
            gate_voltage_metadata(&model(vec![series(5.0)])).as_deref(),
            Some("V<sub>G</sub> 5 V")
        );
        assert_eq!(
            gate_voltage_metadata(&model(vec![series(1.0), series(3.0), series(5.0)])).as_deref(),
            Some("V<sub>G</sub> 1 \u{2192} 5 V")
        );
    }
}
