//! Cox dielectric-stack estimator rendering and policy.

use eframe::egui;
use egui_notify::Toasts;
use paramex_core::transfer::calculate_stack_cox_nf_per_cm2;
use paramex_core::transfer::Session;

use crate::format_ui::fmt_fixed2;
use crate::state::EditBuffers;
use crate::ui_kit::{self, Variant};
use crate::workspaces::transfer::state::CoxUi;

const ACTION_GAP: f32 = 2.0;
const ACTION_BOTTOM_CLEARANCE: f32 = 4.0;
const ESTIMATE_LABEL_HEIGHT: f32 = 16.0;
const ESTIMATED_ACTION_BOTTOM_CLEARANCE: f32 = 8.0;
const MAX_VISIBLE_LAYER_ROWS: f32 = 4.0;
const CLOSE_BUTTON_W: f32 = 20.0;

pub(super) fn render_stack_estimator(
    ui: &mut egui::Ui,
    session: &mut Session,
    cox: &mut CoxUi,
    edits: &mut EditBuffers,
    toasts: &mut Toasts,
) {
    let row_slot_h = ui.spacing().interact_size.y + ui.spacing().item_spacing.y;
    let row_count = cox.layers().len() as f32;
    let max_rows_h = MAX_VISIBLE_LAYER_ROWS * row_slot_h;
    let action_h = action_block_height(cox);
    let rows_h = max_rows_h.min((ui.available_height() - action_h).max(0.0));
    if layer_rows_need_scroll(row_count, max_rows_h, rows_h) {
        ui_kit::scroll_body(ui, "cox_stack_layers_body", rows_h, |ui| {
            render_layer_rows(ui, cox);
        });
    } else {
        ui.allocate_ui(egui::vec2(ui.available_width(), rows_h), |ui| {
            render_layer_rows(ui, cox);
        });
    }
    ui.add_space((ui.available_height() - action_h).max(0.0));
    render_stack_actions(ui, session, cox, edits, toasts);
}

fn layer_rows_need_scroll(row_count: f32, required_height: f32, available_height: f32) -> bool {
    row_count > MAX_VISIBLE_LAYER_ROWS || required_height > available_height
}

fn action_block_height(cox: &CoxUi) -> f32 {
    let mut height = ui_kit::BUTTON_HEIGHT;
    if cox.estimate_value().is_some() {
        height += ACTION_GAP
            + ESTIMATE_LABEL_HEIGHT
            + ACTION_GAP
            + ui_kit::BUTTON_HEIGHT
            + ESTIMATED_ACTION_BOTTOM_CLEARANCE;
    } else {
        height += ACTION_BOTTOM_CLEARANCE;
    }
    height
}

fn render_layer_rows(ui: &mut egui::Ui, cox: &mut CoxUi) {
    if layer_rows(ui, cox) {
        cox.clear_estimate();
    }
}

fn layer_rows(ui: &mut egui::Ui, cox: &mut CoxUi) -> bool {
    // Layer rows (eps_r, thickness_nm). Edits never recompute. A compact "x"
    // keeps delete on the row at narrow right-column width.
    let mut remove: Option<usize> = None;
    let mut layer_changed = false;
    let can_remove_layer = cox.can_remove_layer();
    for (index, layer) in cox.layers_mut().iter_mut().enumerate() {
        ui.horizontal(|ui| {
            let pair_w =
                (ui.available_width() - CLOSE_BUTTON_W - ui.spacing().item_spacing.x).max(0.0);
            let (eps_text, th_text) = layer.texts_mut();
            let (eps_changed, thickness_changed) = ui_kit::inline_paired_settings_row_sized(
                ui,
                pair_w,
                "\u{03B5}<sub>r</sub>",
                eps_text,
                "t (nm)",
                th_text,
            );
            layer_changed |= eps_changed || thickness_changed;
            if can_remove_layer {
                ui_kit::right_aligned(ui, |ui| {
                    let resp =
                        ui_kit::close_button(ui, "Remove layer").on_hover_text("Remove this layer");
                    if resp.clicked() {
                        remove = Some(index);
                    }
                });
            }
        });
    }
    if let Some(index) = remove {
        if cox.remove_layer(index) {
            layer_changed = true;
        }
    }
    layer_changed
}

fn render_stack_actions(
    ui: &mut egui::Ui,
    session: &mut Session,
    cox: &mut CoxUi,
    edits: &mut EditBuffers,
    toasts: &mut Toasts,
) {
    let mut add_layer = false;
    let mut estimate = false;
    ui.columns(2, |cols| {
        add_layer = ui_kit::button_full(&mut cols[0], "Add Layer", Variant::Secondary).clicked();
        estimate = ui_kit::button_full(&mut cols[1], "Estimate C<sub>ox</sub>", Variant::Secondary)
            .clicked();
    });
    if add_layer {
        cox.add_default_layer();
        cox.clear_estimate();
    }

    ui.add_space(ACTION_GAP);
    if cox.estimate_value().is_some() {
        ui_kit::field_label_rich(ui, cox.estimate_label());
        ui.add_space(ACTION_GAP);
    }
    if estimate {
        let value = calculate_stack_cox_nf_per_cm2(&cox.layers_data());
        if value.is_finite() {
            cox.set_estimate(
                format!(
                    "Estimated C<sub>ox</sub>: {} nF/cm<sup>2</sup>",
                    fmt_fixed2(value)
                ),
                value,
            );
        } else {
            cox.clear_estimate();
            toasts.warning(
                "Enter positive relative permittivity (epsilon_r) and thickness for every layer.",
            );
        }
    }
    if let Some(value) = cox.estimate_value() {
        ui.add_space(ACTION_GAP);
        if ui_kit::button_full(ui, "Use Estimated C<sub>ox</sub>", Variant::Secondary).clicked() {
            if session.set_cox(value).is_ok() {
                edits.forget("cox:value"); // re-sync the value field to the new committed Cox
                cox.set_estimate_label(format!(
                    "Using estimated C<sub>ox</sub>: {} nF/cm<sup>2</sup>",
                    fmt_fixed2(value)
                ));
                toasts.warning(
                    "Estimated Cox copied into the extraction field. Measured Cox is preferred.",
                );
            } else {
                cox.clear_estimate();
                toasts.warning("The estimated Cox is no longer valid; estimate the stack again.");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_half_point_shortfall_enables_layer_scrolling() {
        assert!(layer_rows_need_scroll(4.0, 120.0, 119.5));
        assert!(!layer_rows_need_scroll(4.0, 120.0, 120.0));
    }
}
