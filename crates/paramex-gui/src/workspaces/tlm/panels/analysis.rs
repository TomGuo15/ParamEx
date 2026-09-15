//! The ANALYSIS card (left column, below DATA — all the input cards sit
//! on the left): a V_G picker strip over the measured values plus a direct-entry box.

use eframe::egui::{self, Pos2, Rect, Sense, Vec2};
use egui_notify::Toasts;

use crate::format_ui::{fmt_num3, DASH};
use crate::state::EditBuffers;
use crate::theme::SUISEI_MAIN;
use crate::ui_kit;
use crate::ui_kit::{CONTROL_SLIDER_HEIGHT, CONTROL_SLIDER_INSET, CONTROL_THUMB_RADIUS};
use crate::workspaces::tlm::state::TlmState;

const BLANK_SKELETON_VALUE: &str = " ";

/// Parse a V_G commit. The engine snaps any finite value to the nearest measurement.
pub fn commit_vg(text: &str) -> Result<f64, &'static str> {
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or("Gate voltage must be a number.")
}

/// Index of the measured V_G nearest to `selected`.
pub fn vg_slider_index(vg_values: &[f64], selected: f64) -> usize {
    let mut best = 0;
    let mut best_d = f64::INFINITY;
    for (i, &v) in vg_values.iter().enumerate() {
        let d = (v - selected).abs();
        if d < best_d {
            best = i;
            best_d = d;
        }
    }
    best
}

/// Single-thumb picker using the shared custom-control style: gray rounded rail
/// and a surface thumb with an accent ring. Mutates `idx` live while dragging; returns a commit
/// on pointer release/click or a discrete keyboard/accessibility action.
fn vg_strip(ui: &mut egui::Ui, id_salt: &str, values: &[f64], idx: &mut usize, width: f32) -> bool {
    let (rect, _bg) =
        ui.allocate_exact_size(Vec2::new(width, CONTROL_SLIDER_HEIGHT), Sense::hover());
    // Same inset as the double-thumb strip: an end thumb sits fully inside the
    // card instead of clipping against its margin.
    let (x0, x1, mid_y) = (
        rect.left() + CONTROL_SLIDER_INSET,
        rect.right() - CONTROL_SLIDER_INSET,
        rect.center().y,
    );
    let span = (values.len().max(2) - 1) as f32;
    let to_x = |i: usize| x0 + (i as f32 / span) * (x1 - x0);
    let to_i = |x: f32| {
        (((((x - x0) / (x1 - x0)).clamp(0.0, 1.0) * span).round()) as usize)
            .min(values.len().saturating_sub(1))
    };

    // Click/drag anywhere on the track band, not just the thumb. The release
    // frame MUST also read the pointer: `idx` is re-derived from the COMMITTED
    // V_G every frame, and `dragged()` is already false when `drag_stopped()`
    // fires — without this arm the whole drag would be thrown away at commit
    // time (the strip moved but nothing ever changed).
    let band = Rect::from_min_max(Pos2::new(x0, mid_y - 8.0), Pos2::new(x1, mid_y + 8.0));
    let resp = ui.interact(band, ui.id().with(id_salt), Sense::click_and_drag());
    let before = *idx;
    if resp.dragged() || resp.clicked() || resp.drag_stopped() {
        if let Some(p) = resp.interact_pointer_pos() {
            *idx = to_i(p.x);
        }
    }

    let enabled = ui.is_enabled();
    let mut committed = resp.drag_stopped() || resp.clicked();
    let mut focused = false;
    if enabled {
        let (delta, requested_value, has_focus) = ui_kit::discrete_slider_input(ui, &resp);
        focused = has_focus;
        if delta != 0 && !values.is_empty() {
            *idx = (*idx as isize + delta).clamp(0, values.len() as isize - 1) as usize;
            committed |= *idx != before;
        }
        if let Some(value) = requested_value {
            *idx = vg_slider_index(values, value);
            committed |= *idx != before;
        }
    }

    let value = values.get(*idx).copied().unwrap_or(0.0);
    let min = values.iter().copied().reduce(f64::min).unwrap_or(0.0);
    let max = values.iter().copied().reduce(f64::max).unwrap_or(0.0);
    resp.widget_info(|| egui::WidgetInfo::slider(enabled, value, "Gate voltage VG (V)"));
    ui.ctx().accesskit_node_builder(resp.id, |builder| {
        use egui::accesskit::Action;
        builder.set_min_numeric_value(min);
        builder.set_max_numeric_value(max);
        if enabled {
            builder.add_action(Action::SetValue);
            if *idx + 1 < values.len() {
                builder.add_action(Action::Increment);
            }
            if *idx > 0 {
                builder.add_action(Action::Decrement);
            }
        }
    });

    let painter = ui.painter();
    // Plain grey rail, NO trailing fill: this strip picks a single V_G point,
    // and a fill-to-thumb reads as "amount selected" (range semantics). The
    // accent segment belongs to the double-thumb RANGE strip only.
    ui_kit::paint_control_rail(painter, x0, x1, mid_y);
    if !ui.is_enabled() {
        return false;
    }

    let (thumb_fill, thumb_stroke) = ui_kit::control_thumb_style(SUISEI_MAIN);
    painter.circle(
        Pos2::new(to_x(*idx), mid_y),
        CONTROL_THUMB_RADIUS,
        thumb_fill,
        thumb_stroke,
    );
    if focused {
        painter.circle_stroke(
            Pos2::new(to_x(*idx), mid_y),
            CONTROL_THUMB_RADIUS + 3.0,
            ui.visuals().selection.stroke,
        );
    }
    committed
}

/// ANALYSIS card: a slider across the measured V_G values + a type-in box. Both
/// re-analyze the held dataset in memory (`recompute_at_vg`, which snaps to the
/// nearest measured V_G) — collected and applied after render. The slider commits
/// on release, not per drag-frame: a recompute per frame would re-fit every group
/// on every mouse move.
pub fn show(ui: &mut egui::Ui, tlm: &mut TlmState, edits: &mut EditBuffers, toasts: &mut Toasts) {
    let mut pick: Option<f64> = None;
    let mut invalid: Option<&'static str> = None;
    ui_kit::card_slot(ui, |ui| {
        ui_kit::section_header(ui, "ANALYSIS", None);
        let picker = tlm.vg_picker();
        ui_kit::field_label_rich(ui, "Gate voltage V<sub>G</sub>");
        let selected = picker.map_or(0.0, |picker| picker.selected_vg);

        ui.add_enabled_ui(picker.is_some(), |ui| {
            let values = picker.map_or(&[0.0, 1.0][..], |picker| picker.vg_values);
            let mut idx = picker.map_or(0, |picker| vg_slider_index(picker.vg_values, selected));
            let before = idx;
            ui.horizontal(|ui| {
                // Reserve room for the readout: 34px number slot + "V" + the two
                // inter-item gaps (~10px each side) — 56 was the single-label
                // budget and the split number/unit overflowed the card edge.
                let strip_w = (ui.available_width() - 68.0).max(60.0);
                let strip_committed = vg_strip(ui, "tlm_vg_strip", values, &mut idx, strip_w);
                // The number tracks the thumb live mid-drag (idx mutates while
                // dragging) inside a FIXED right-aligned slot, so the "V" unit
                // stays anchored instead of jittering with the number's width.
                if let Some(&v) = values.get(idx) {
                    ui.allocate_ui_with_layout(
                        egui::vec2(34.0, 18.0),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if picker.is_some() {
                                ui_kit::readout_value_label(ui, fmt_num3(v));
                            } else {
                                ui_kit::readout_value_label(ui, DASH);
                            }
                        },
                    );
                    ui_kit::readout_unit_label(ui, "V");
                }
                // Pointer changes commit on release/click, never mid-drag; keyboard and
                // assistive actions are already discrete. Recomputing mid-drag would
                // re-fit every group on every mouse move.
                if picker.is_some() && strip_committed && idx != before {
                    if let Some(&v) = values.get(idx) {
                        pick = Some(v);
                    }
                }
            });
        });

        ui.add_space(6.0);
        let key = "tlm:vg";
        let committed =
            picker.map_or_else(|| BLANK_SKELETON_VALUE.to_string(), |_| fmt_num3(selected));
        if let Some(text) = ui
            .add_enabled_ui(picker.is_some(), |ui| {
                ui_kit::inline_settings_row_commit(ui, edits, key, "V<sub>G</sub> (V)", &committed)
            })
            .inner
        {
            // The V_G strip renders just above this field; clicking it steals the field's
            // focus, so the field's lost_focus then commits its STALE value the same frame.
            // The strip's fresh pick (set above) is authoritative — don't let the stale
            // field commit override it. (Forgetting the buffer wouldn't help: it re-seeds
            // from the pre-pick `committed` value, same as Reset-to-Auto.)
            if pick.is_none() {
                match commit_vg(&text) {
                    Ok(v) => pick = Some(v),
                    Err(msg) => invalid = Some(msg),
                }
            }
        }
    });
    if let Some(msg) = invalid {
        toasts.warning(msg);
    }
    if let Some(v) = pick {
        tlm.recompute_at_vg(v);
    }
}
