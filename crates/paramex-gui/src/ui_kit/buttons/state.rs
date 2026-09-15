//! Button visual-state engine shared by button and segmented-control recipes.

use eframe::egui::{self, Color32, CornerRadius, Response, Stroke};

use crate::theme::{radius, token_alpha, tokens};

use super::{
    Variant, SECONDARY_BUTTON_HOVER_ALPHA, SECONDARY_BUTTON_PRESS_ALPHA,
    SEMANTIC_BUTTON_HOVER_ALPHA, SEMANTIC_BUTTON_PRESS_ALPHA,
};

/// Per-state `(fill, stroke)` pairs plus the corner radius for a button. An
/// explicit `Button::fill`/`stroke` pins every state (egui documents that it
/// overrides the hover effects), so the pairs land in scoped widget visuals
/// instead: egui's `button_style` reads frame fill from `weak_bg_fill` and
/// stroke from `bg_stroke` per state.
#[derive(Clone, Copy)]
pub(in crate::ui_kit) struct ButtonStates {
    rest: (Color32, Stroke),
    hover: (Color32, Stroke),
    press: (Color32, Stroke),
    corner_radius: u8,
}

fn hairline(fill: Color32, stroke: Color32) -> (Color32, Stroke) {
    (fill, Stroke::new(1.0_f32, stroke))
}

impl ButtonStates {
    fn outlined(
        rest: (Color32, Color32),
        hover: (Color32, Color32),
        press: (Color32, Color32),
    ) -> Self {
        Self {
            rest: hairline(rest.0, rest.1),
            hover: hairline(hover.0, hover.1),
            press: hairline(press.0, press.1),
            corner_radius: radius::WIDGET,
        }
    }

    /// Fill-only states with no outline in any state (title-rail actions).
    pub(in crate::ui_kit) fn borderless(rest: Color32, hover: Color32, press: Color32) -> Self {
        Self {
            rest: (rest, Stroke::NONE),
            hover: (hover, Stroke::NONE),
            press: (press, Stroke::NONE),
            corner_radius: radius::WIDGET,
        }
    }

    pub(in crate::ui_kit) fn with_corner_radius(mut self, corner_radius: u8) -> Self {
        self.corner_radius = corner_radius;
        self
    }

    /// Install the four widget states on `ui`'s style. Call inside a scope so
    /// the visuals do not leak past the button.
    pub(in crate::ui_kit) fn apply(self, ui: &mut egui::Ui) {
        let set = |wv: &mut egui::style::WidgetVisuals, (fill, stroke): (Color32, Stroke)| {
            wv.weak_bg_fill = fill;
            wv.bg_fill = fill;
            wv.bg_stroke = stroke;
            wv.corner_radius = CornerRadius::same(self.corner_radius);
        };
        let widgets = &mut ui.style_mut().visuals.widgets;
        set(&mut widgets.noninteractive, self.rest);
        set(&mut widgets.inactive, self.rest);
        set(&mut widgets.hovered, self.hover);
        set(&mut widgets.active, self.press);
    }
}

/// States for a FILLED button (Primary, the direction-colored buttons):
/// hover keeps the fill but swaps to a contrasting approved-token stroke;
/// press lands on the Studio Stellar dark token.
pub(in crate::ui_kit) fn filled_states(color: Color32) -> ButtonStates {
    let t = tokens();
    let hover_stroke = if color == t.ink { t.primary } else { t.ink };
    ButtonStates::outlined((color, color), (color, hover_stroke), (t.ink, t.ink))
}

/// States for an OUTLINED button (white fill, hue text/stroke): hover/press use
/// explicit alpha washes of the button's own palette token over the card.
pub(in crate::ui_kit) fn outlined_states(
    rest_stroke: Color32,
    hue: Color32,
    hover_alpha: u8,
    press_alpha: u8,
) -> ButtonStates {
    ButtonStates::outlined(
        (tokens().surface, rest_stroke),
        (token_alpha(hue, hover_alpha), hue),
        (token_alpha(hue, press_alpha), hue),
    )
}

/// States for a soft semantic filled button (Warning). Text stays Studio dark,
/// so press/hover feedback uses only approved-token stroke changes.
fn soft_semantic_filled_states(fill: Color32) -> ButtonStates {
    let t = tokens();
    ButtonStates::outlined((fill, fill), (fill, t.ink), (fill, t.primary))
}

/// The rest pairs must equal `variant_colors`' fill/stroke exactly; the at-rest
/// render is contractually unchanged by the state engine.
fn variant_states(v: Variant) -> ButtonStates {
    let t = tokens();
    match v {
        Variant::Primary => filled_states(t.primary),
        // Secondary rests on the neutral `border` hairline and only takes its
        // brand outline on hover (mirrors the theme-wide hover language).
        Variant::Secondary => outlined_states(
            t.border,
            t.primary,
            SECONDARY_BUTTON_HOVER_ALPHA,
            SECONDARY_BUTTON_PRESS_ALPHA,
        ),
        Variant::Danger => outlined_states(
            t.red,
            t.red,
            SEMANTIC_BUTTON_HOVER_ALPHA,
            SEMANTIC_BUTTON_PRESS_ALPHA,
        ),
        Variant::Warning => soft_semantic_filled_states(t.yellow),
    }
}

/// Like [`variant_states`] but honoring the host ui's enabled state: a disabled
/// button keeps the white/grey rest pair in every state (no hover
/// feedback on an inert control).
pub(in crate::ui_kit) fn variant_states_in(ui: &egui::Ui, v: Variant) -> ButtonStates {
    if ui.is_enabled() {
        variant_states(v)
    } else {
        let t = tokens();
        let inert = (t.surface, t.ink_soft);
        ButtonStates::outlined(inert, inert, inert)
    }
}

/// Add `content` as a button whose fill/stroke come from `states` via scoped
/// widget visuals (widget corners, the shared 30px-min-height chain). `size`
/// pins the slot (full-width buttons); `None` is content-width.
pub(in crate::ui_kit) fn add_state_button(
    ui: &mut egui::Ui,
    content: impl egui::IntoAtoms<'static>,
    states: ButtonStates,
    size: Option<egui::Vec2>,
    min_size: egui::Vec2,
) -> Response {
    ui.scope(|ui| {
        states.apply(ui);
        let btn = egui::Button::new(content).min_size(min_size);
        match size {
            Some(s) => ui.add_sized(s, btn),
            None => ui.add(btn),
        }
    })
    .inner
}
