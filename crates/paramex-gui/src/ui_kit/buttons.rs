//! Body buttons, title-rail actions, and the hand-painted close/icon buttons.
//! Every recipe reads its colors from the palette tokens and its fill/stroke
//! feedback from the shared [`state::ButtonStates`] engine.

use eframe::egui::{self, text::LayoutJob, Color32, FontId, Response};

use crate::richtext;
use crate::theme::{radius, token_alpha, tokens, type_scale};

use super::bold_family;

mod state;

use state::ButtonStates;
pub(super) use state::{add_state_button, filled_states, outlined_states, variant_states_in};

/// Button variants in the palette's semantic roles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variant {
    /// Brand-blue fill, white text.
    Primary,
    /// White fill, brand-blue text, neutral hairline.
    Secondary,
    /// White fill, red text and border.
    Danger,
    /// Yellow fill, ink text and yellow border.
    Warning,
}

pub const BUTTON_HEIGHT: f32 = 30.0;
pub const HEADER_ACTION_HEIGHT: f32 = 20.0;
pub const SECONDARY_BUTTON_HOVER_ALPHA: u8 = 15;
pub const SECONDARY_BUTTON_PRESS_ALPHA: u8 = 31;
pub const SEMANTIC_BUTTON_HOVER_ALPHA: u8 = 20;
pub const SEMANTIC_BUTTON_PRESS_ALPHA: u8 = 41;
pub const CLOSE_BUTTON_HOVER_ALPHA: u8 = 26;
pub const CLOSE_BUTTON_PRESS_ALPHA: u8 = 41;
/// Square hit area of the hand-painted close and icon buttons.
const ICON_BUTTON_SIZE: f32 = 20.0;
const ICON_BUTTON_STROKE_WIDTH: f32 = 1.5;

pub(super) fn button_label_job(ui: &egui::Ui, markup: &str, color: Color32) -> LayoutJob {
    richtext::layout_sub_sup(
        markup,
        FontId::new(type_scale::CONTROL, bold_family(ui)),
        color,
    )
}

fn variant_colors(v: Variant) -> (Color32, Color32, Color32) {
    let t = tokens();
    match v {
        Variant::Primary => (t.primary, t.surface, t.primary),
        Variant::Secondary => (t.surface, t.primary, t.border),
        Variant::Danger => (t.surface, t.red, t.red),
        // Yellow is too quiet as small text on white. Use it as the button
        // surface/stroke and keep Studio dark copy for contrast.
        Variant::Warning => (t.yellow, t.ink, t.yellow),
    }
}

/// `(fill, text, stroke)` for a variant button given the host ui's enabled
/// state. egui paints a disabled Ui at `disabled_alpha` (0.5), which turns the
/// brand fill/outline into a washed-out "half-pressed" button (pale blue,
/// white text) — render disabled buttons white/grey instead, with colors
/// picked to survive the alpha multiply (ink text → mid-grey, `ink_soft`
/// hairline → light grey).
fn variant_colors_in(ui: &egui::Ui, v: Variant) -> (Color32, Color32, Color32) {
    if ui.is_enabled() {
        variant_colors(v)
    } else {
        let t = tokens();
        (t.surface, t.ink, t.ink_soft)
    }
}

/// A full-width variant button (the common case inside cards). The label may
/// carry `<sub>`/`<sup>` markup (e.g. "Estimate C<sub>ox</sub>") — the shared
/// `button_label_job` parses it.
pub fn button_full(ui: &mut egui::Ui, label: &str, v: Variant) -> Response {
    let w = ui.available_width();
    let text = variant_colors_in(ui, v).1;
    let states = variant_states_in(ui, v);
    let content = button_label_job(ui, label, text);
    add_state_button(
        ui,
        content,
        states,
        Some(egui::vec2(w, BUTTON_HEIGHT)),
        egui::vec2(0.0, BUTTON_HEIGHT),
    )
}

/// A content-width variant button (for side-by-side rows).
pub fn button(ui: &mut egui::Ui, label: &str, v: Variant) -> Response {
    let text = variant_colors_in(ui, v).1;
    let states = variant_states_in(ui, v);
    let content = button_label_job(ui, label, text);
    add_state_button(ui, content, states, None, egui::vec2(0.0, BUTTON_HEIGHT))
}

/// Compact borderless action for the shared card-title rail. Primary actions
/// use brand-blue text; secondary actions stay quiet until hover.
pub fn header_action(ui: &mut egui::Ui, label: &str, v: Variant) -> Response {
    let t = tokens();
    let enabled = ui.is_enabled();
    let (text, hue) = if !enabled {
        (t.ink, t.ink_soft)
    } else {
        match v {
            Variant::Primary => (t.primary, t.primary),
            Variant::Secondary => (t.ink_soft, t.primary),
            Variant::Danger => (t.red, t.red),
            Variant::Warning => (t.ink, t.yellow),
        }
    };
    let rest = Color32::TRANSPARENT;
    let (hover, press) = if enabled {
        (
            token_alpha(hue, SECONDARY_BUTTON_HOVER_ALPHA),
            token_alpha(hue, SECONDARY_BUTTON_PRESS_ALPHA),
        )
    } else {
        (rest, rest)
    };
    let states = ButtonStates::borderless(rest, hover, press).with_corner_radius(radius::CHIP);
    let content = richtext::layout_sub_sup(
        label,
        FontId::new(type_scale::HEADER_ACTION, bold_family(ui)),
        text,
    );

    ui.scope(|ui| {
        states.apply(ui);
        ui.spacing_mut().button_padding = egui::vec2(2.0, 0.0);
        ui.add(egui::Button::new(content).min_size(egui::vec2(0.0, HEADER_ACTION_HEIGHT)))
    })
    .inner
}

/// Allocate the square hit area shared by the hand-painted icon buttons, paint
/// its hover/press wash, and return `(rect, response, enabled, glyph color)`:
/// quiet grey at rest, `hue` with a wash on hover/press, muted with no wash
/// when the enclosing scope is disabled.
fn icon_button_base(ui: &mut egui::Ui, hue: Color32) -> (egui::Rect, Response, bool, Color32) {
    let enabled = ui.is_enabled();
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(egui::Vec2::splat(ICON_BUTTON_SIZE), sense);
    let t = tokens();
    let (wash, glyph) = if !enabled {
        (Color32::TRANSPARENT, t.ink_soft)
    } else if resp.is_pointer_button_down_on() {
        (token_alpha(hue, CLOSE_BUTTON_PRESS_ALPHA), hue)
    } else if resp.hovered() {
        (token_alpha(hue, CLOSE_BUTTON_HOVER_ALPHA), hue)
    } else {
        (Color32::TRANSPARENT, t.ink_soft)
    };
    if wash != Color32::TRANSPARENT {
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(radius::SEGMENT), wash);
    }
    (rect, resp, enabled, glyph)
}

/// Finish an icon button: pointer cursor when enabled and the accesskit name
/// the kittest guards target.
fn icon_button_finish(mut resp: Response, enabled: bool, label: &str) -> Response {
    if enabled {
        resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    let owned = label.to_owned();
    resp.widget_info(move || {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, owned.clone())
    });
    resp
}

/// A borderless close/remove affordance: a hand-painted crisp cross, quiet gray
/// at rest, washing red with a pointer cursor on hover. A boxed text-glyph "×"
/// button reads as foreign next to the painted chrome, so the cross is painted.
/// `label` is the accesskit name the kittest guards target ("Dismiss", "Remove
/// layer", …). Honors a disabled enclosing `add_enabled_ui` scope: no click
/// sense, muted cross, no wash.
pub fn close_button(ui: &mut egui::Ui, label: &str) -> Response {
    let (rect, resp, enabled, cross) = icon_button_base(ui, tokens().red);
    let c = rect.center();
    let r = 3.5;
    let s = egui::Stroke::new(ICON_BUTTON_STROKE_WIDTH, cross);
    ui.painter()
        .line_segment([c + egui::vec2(-r, -r), c + egui::vec2(r, r)], s);
    ui.painter()
        .line_segment([c + egui::vec2(-r, r), c + egui::vec2(r, -r)], s);
    icon_button_finish(resp, enabled, label)
}

#[derive(Clone, Copy)]
pub enum OutputActionIcon {
    Attach,
    Detach,
}

/// A borderless attach/detach arrow for output-curve rows, sharing the close
/// button's hit area and wash language with the brand hue instead of red.
pub fn output_action_icon_button(
    ui: &mut egui::Ui,
    label: &str,
    icon: OutputActionIcon,
) -> Response {
    let (rect, resp, enabled, stroke_color) = icon_button_base(ui, tokens().primary);
    let c = rect.center();
    let s = egui::Stroke::new(ICON_BUTTON_STROKE_WIDTH, stroke_color);
    let dir = match icon {
        OutputActionIcon::Attach => -1.0,
        OutputActionIcon::Detach => 1.0,
    };
    ui.painter().line_segment(
        [
            c + egui::vec2(0.0, -6.0 * dir),
            c + egui::vec2(0.0, 2.0 * dir),
        ],
        s,
    );
    ui.painter().line_segment(
        [
            c + egui::vec2(-3.5, -1.5 * dir),
            c + egui::vec2(0.0, 2.0 * dir),
        ],
        s,
    );
    ui.painter().line_segment(
        [
            c + egui::vec2(3.5, -1.5 * dir),
            c + egui::vec2(0.0, 2.0 * dir),
        ],
        s,
    );
    ui.painter().line_segment(
        [
            c + egui::vec2(-5.0, 6.0 * dir),
            c + egui::vec2(5.0, 6.0 * dir),
        ],
        s,
    );
    icon_button_finish(resp, enabled, label)
}

/// A content-width button filled with an explicit brand color + white label (used for
/// the single-sweep "Forward" marker so it matches the forward line color).
pub fn colored_button(ui: &mut egui::Ui, label: &str, color: Color32) -> Response {
    let w = ui.available_width();
    let (states, text) = if ui.is_enabled() {
        (filled_states(color), tokens().surface)
    } else {
        (variant_states_in(ui, Variant::Secondary), tokens().ink)
    };
    let content = button_label_job(ui, label, text);
    add_state_button(
        ui,
        content,
        states,
        Some(egui::vec2(w, BUTTON_HEIGHT)),
        egui::vec2(0.0, BUTTON_HEIGHT),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_buttons_match_their_text_and_outline_colors() {
        let t = tokens();
        assert_eq!(variant_colors(Variant::Danger), (t.surface, t.red, t.red));
        assert_eq!(
            variant_colors(Variant::Warning),
            (t.yellow, t.ink, t.yellow)
        );
    }

    #[test]
    fn primary_button_uses_utility_surface_text() {
        let t = tokens();
        assert_eq!(
            variant_colors(Variant::Primary),
            (t.primary, t.surface, t.primary)
        );
    }
}
