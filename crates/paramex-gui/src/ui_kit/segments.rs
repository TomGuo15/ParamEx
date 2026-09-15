//! Segmented-control recipes for workspace toggles and graph mode tabs.

use eframe::egui::{
    self, Color32, CornerRadius, FontId, Margin, Stroke, StrokeKind, WidgetInfo, WidgetType,
};

use crate::richtext;
use crate::theme::{
    radius, tokens, type_scale, utility_white_alpha, BANNER_SEGMENT_HOVER_ALPHA, INK_RAISED,
};

use super::bold_family;
use super::buttons::{
    add_state_button, button_label_job, filled_states, outlined_states, variant_states_in, Variant,
    BUTTON_HEIGHT, SEMANTIC_BUTTON_HOVER_ALPHA, SEMANTIC_BUTTON_PRESS_ALPHA,
};

pub const HEADER_TAB_HEIGHT: f32 = 20.0;
const HEADER_TAB_UNDERLINE_HEIGHT: f32 = 2.0;
const HEADER_TAB_UNDERLINE_INSET: f32 = 6.0;
const BANNER_SEGMENT_HEIGHT: f32 = 26.0;
const BANNER_SEGMENT_GAP: f32 = 2.0;
const BANNER_SEGMENT_MIN_WIDTH: f32 = 40.0;

/// Where a segmented control sits: inside a white card, or on the ink banner.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SegStyle {
    Card,
    Banner,
}

/// Hover fill for an INACTIVE segment. Card: a soft primary alpha
/// wash; Banner: a light utility wash over the ink track (rest is transparent).
fn segment_hover_fill(style: SegStyle) -> Color32 {
    match style {
        SegStyle::Card => tokens().accent_soft,
        SegStyle::Banner => utility_white_alpha(BANNER_SEGMENT_HOVER_ALPHA),
    }
}

/// Pure: `(fill, text color)` for one segment given its style and whether it is active.
pub fn segment_colors(style: SegStyle, active: bool) -> (Color32, Color32) {
    let t = tokens();
    match (style, active) {
        (SegStyle::Card, true) => (Color32::TRANSPARENT, t.primary),
        (SegStyle::Card, false) => (Color32::TRANSPARENT, t.ink_soft),
        (SegStyle::Banner, true) => (t.surface, t.ink),
        (SegStyle::Banner, false) => (Color32::TRANSPARENT, t.surface),
    }
}

/// An N-segment control. Card tabs are flat, intrinsic-width title-rail actions;
/// banner segments retain their rounded track. `Some(w)` fixes segment width.
/// Labels may carry `<sub>`/`<sup>` markup. Returns the clicked index.
pub fn segmented(
    ui: &mut egui::Ui,
    labels: &[&str],
    active: usize,
    style: SegStyle,
    seg_w: Option<f32>,
) -> Option<usize> {
    segmented_with_accessibility_labels(ui, labels, &[], active, style, seg_w)
}

/// [`segmented`] with a parallel list of accesskit names, so a visual label
/// such as `"TLM"` can expose `"TLM guide"` to assistive technology and tests.
/// Segments beyond the end of `accessibility_labels` keep their visual label.
pub fn segmented_with_accessibility_labels(
    ui: &mut egui::Ui,
    labels: &[&str],
    accessibility_labels: &[&str],
    active: usize,
    style: SegStyle,
    seg_w: Option<f32>,
) -> Option<usize> {
    match style {
        SegStyle::Card => card_header_tabs(ui, labels, accessibility_labels, active, seg_w),
        SegStyle::Banner => banner_segments(ui, labels, accessibility_labels, active, seg_w),
    }
}

fn set_accessibility_label(ui: &egui::Ui, resp: &egui::Response, label: Option<&&str>) {
    if let Some(label) = label {
        resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), *label));
    }
}

/// The banner toggle: white lifted pill for the active segment, transparent
/// segments with white labels otherwise, all on the black raised track.
fn banner_segments(
    ui: &mut egui::Ui,
    labels: &[&str],
    accessibility_labels: &[&str],
    active: usize,
    seg_w: Option<f32>,
) -> Option<usize> {
    let t = tokens();
    let mut clicked = None;
    egui::Frame::new()
        .fill(INK_RAISED)
        .stroke(Stroke::NONE)
        .corner_radius(CornerRadius::same(radius::WIDGET))
        .inner_margin(Margin::same(2))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = BANNER_SEGMENT_GAP;
                let n = labels.len().max(1) as f32;
                let w = seg_w.unwrap_or_else(|| {
                    ((ui.available_width() - BANNER_SEGMENT_GAP * (n - 1.0)) / n)
                        .max(BANNER_SEGMENT_MIN_WIDTH)
                });
                for (idx, label) in labels.iter().enumerate() {
                    let is_active = idx == active;
                    let (mut fill, mut text) = segment_colors(SegStyle::Banner, is_active);
                    if !ui.is_enabled() {
                        fill = t.surface;
                        text = t.ink;
                    }
                    // Every segment carries its explicit fill. An explicit fill
                    // pins every widget state, so the hover wash is hand-rolled
                    // off the PREVIOUS frame's response (the file-row idiom):
                    // inactive segments only; the active segment is the current
                    // page and inert to hover.
                    let id = ui.next_auto_id();
                    let hovered =
                        !is_active && ui.ctx().read_response(id).is_some_and(|r| r.hovered());
                    if hovered {
                        fill = segment_hover_fill(SegStyle::Banner);
                    }
                    let job = richtext::layout_sub_sup(
                        label,
                        FontId::new(type_scale::CONTROL, bold_family(ui)),
                        text,
                    );
                    let btn = egui::Button::new(job)
                        .stroke(Stroke::NONE)
                        .corner_radius(CornerRadius::same(radius::SEGMENT))
                        .min_size(egui::vec2(w, BANNER_SEGMENT_HEIGHT))
                        .fill(fill);
                    let resp = ui.add(btn);
                    set_accessibility_label(ui, &resp, accessibility_labels.get(idx));
                    if resp.clicked() {
                        clicked = Some(idx);
                    }
                }
            });
        });
    clicked
}

/// Flat title-rail tabs: the active tab carries a primary underline, inactive
/// tabs wash on hover, and keyboard focus draws a primary ring.
fn card_header_tabs(
    ui: &mut egui::Ui,
    labels: &[&str],
    accessibility_labels: &[&str],
    active: usize,
    seg_w: Option<f32>,
) -> Option<usize> {
    let t = tokens();
    let mut clicked = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (idx, label) in labels.iter().enumerate() {
            let is_active = idx == active;
            let (_, mut text) = segment_colors(SegStyle::Card, is_active);
            if !ui.is_enabled() {
                text = t.ink;
            }
            let id = ui.next_auto_id();
            let hovered = !is_active && ui.ctx().read_response(id).is_some_and(|r| r.hovered());
            let fill = if hovered {
                segment_hover_fill(SegStyle::Card)
            } else {
                Color32::TRANSPARENT
            };
            let job = richtext::layout_sub_sup(
                label,
                FontId::new(type_scale::HEADER_TAB, bold_family(ui)),
                text,
            );
            let button = egui::Button::new(job)
                .stroke(Stroke::NONE)
                .corner_radius(CornerRadius::same(radius::INNER))
                .min_size(egui::vec2(0.0, HEADER_TAB_HEIGHT))
                .fill(fill);
            let resp = match seg_w {
                Some(width) => ui.add_sized(egui::vec2(width, HEADER_TAB_HEIGHT), button),
                None => ui.add(button),
            };
            set_accessibility_label(ui, &resp, accessibility_labels.get(idx));
            if is_active {
                let underline = egui::Rect::from_min_max(
                    egui::pos2(
                        resp.rect.left() + HEADER_TAB_UNDERLINE_INSET,
                        resp.rect.bottom() - HEADER_TAB_UNDERLINE_HEIGHT,
                    ),
                    egui::pos2(
                        resp.rect.right() - HEADER_TAB_UNDERLINE_INSET,
                        resp.rect.bottom(),
                    ),
                );
                ui.painter()
                    .rect_filled(underline, CornerRadius::same(1), t.primary);
            }
            if resp.has_focus() {
                ui.painter().rect_stroke(
                    resp.rect.shrink(1.0),
                    CornerRadius::same(radius::INNER),
                    Stroke::new(1.0_f32, t.primary),
                    StrokeKind::Inside,
                );
            }
            if resp.clicked() {
                clicked = Some(idx);
            }
        }
    });
    clicked
}

/// Like [`segmented`] but each segment carries its OWN brand color (e.g. the forward/backward
/// LINE colors) so the toggle doubles as the graph legend and no separate legend row is
/// needed. The active segment is FILLED with its color (white label); the inactive one
/// is OUTLINED in it. Returns the clicked index.
pub fn segmented_two_colored(
    ui: &mut egui::Ui,
    labels: [&str; 2],
    active: usize,
    colors: [Color32; 2],
) -> Option<usize> {
    let mut clicked = None;
    ui.horizontal(|ui| {
        // Split the full row width between the two segments so the toggle fills the card.
        let seg_w = ((ui.available_width() - ui.spacing().item_spacing.x) / 2.0).max(40.0);
        for (idx, label) in labels.iter().enumerate() {
            let color = colors[idx];
            let (states, text) = if !ui.is_enabled() {
                (variant_states_in(ui, Variant::Secondary), tokens().ink)
            } else if idx == active {
                (filled_states(color), tokens().surface)
            } else {
                (
                    outlined_states(
                        color,
                        color,
                        SEMANTIC_BUTTON_HOVER_ALPHA,
                        SEMANTIC_BUTTON_PRESS_ALPHA,
                    ),
                    color,
                )
            };
            let content = button_label_job(ui, label, text);
            let resp = add_state_button(
                ui,
                content,
                states,
                Some(egui::vec2(seg_w, BUTTON_HEIGHT)),
                egui::vec2(0.0, BUTTON_HEIGHT),
            );
            if resp.clicked() {
                clicked = Some(idx);
            }
        }
    });
    clicked
}
