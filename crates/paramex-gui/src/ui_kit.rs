//! Reusable design-system primitives: white cards with a soft shadow, compact
//! single-row title rails, flat local tabs/actions, right-aligned numeric inputs,
//! and the standard body-button variants. All colors come from [`crate::theme`].
//!
//! Header titles may contain `<sub>`/`<sup>` markup (rendered via [`crate::richtext`]),
//! so e.g. `"GATE OXIDE C<sub>ox</sub>"` renders correctly. Titles are passed in their
//! final case (no auto-uppercasing -- that would corrupt the markup tags).

use std::sync::LazyLock;

use eframe::egui::{self, FontFamily};

mod buttons;
mod cards;
mod headers;
mod inputs;
mod metrics;
mod scroll;
mod segments;
mod selection;
mod sliders;
mod status;
mod text;

pub use buttons::{
    button, button_full, close_button, colored_button, header_action, output_action_icon_button,
    OutputActionIcon, Variant, BUTTON_HEIGHT, CLOSE_BUTTON_HOVER_ALPHA, CLOSE_BUTTON_PRESS_ALPHA,
    HEADER_ACTION_HEIGHT, SECONDARY_BUTTON_HOVER_ALPHA, SECONDARY_BUTTON_PRESS_ALPHA,
    SEMANTIC_BUTTON_HOVER_ALPHA, SEMANTIC_BUTTON_PRESS_ALPHA,
};
pub use cards::{card, card_frame, card_slot, CARD_INNER_MARGIN};
pub use headers::{
    header_action_row, header_nav_action_row, right_aligned, section_header, HEADER_RAIL_HEIGHT,
};
pub use inputs::{
    inline_paired_settings_row_sized, inline_settings_row_commit, settings_cell_commit,
    singleline_edit, singleline_edit_commit, terminal_numeric_row, COMPACT_NUMERIC_INPUT_WIDTH,
    INPUT_LABEL_GAP,
};
pub use metrics::{
    metric_label, metric_label_color, metric_table_cell, metric_value, metric_value_color,
    readout_unit_color, readout_unit_label, readout_value_color, readout_value_label,
};
pub use scroll::scroll_body;
pub use segments::{
    segment_colors, segmented, segmented_two_colored, segmented_with_accessibility_labels,
    SegStyle, HEADER_TAB_HEIGHT,
};
pub use selection::{
    selectable_row_response, selection_bar, selection_row_fill, selection_row_frame,
    selection_row_stroke,
};
pub use sliders::{
    control_thumb_style, discrete_slider_input, paint_control_rail, paint_control_rail_segment,
    CONTROL_RAIL_COLOR, CONTROL_SLIDER_HEIGHT, CONTROL_SLIDER_INSET, CONTROL_THUMB_RADIUS,
    CONTROL_THUMB_RING_WIDTH,
};
pub use status::{
    compact_error_notice, file_error_row, file_error_summary, list_row_title_status,
    load_error_summary, semantic_badge, semantic_badge_colors, status_badge_line, BadgeTone,
    StatusLineText, FILE_ERROR_SUMMARY_MAX_CHARS,
};
pub use text::{
    field_label, field_label_rich, file_row_gutter, muted_label, muted_row_title_label,
    muted_wrapped_label, row_title_color, row_title_label, truncated_row_title_label,
};

/// Per-context flag (set by `theme::install`) marking that the `"bold"` font family
/// has been registered. Lets `bold_family` fall back safely in tests that render a
/// panel without installing the theme (egui panics on an unbound `Name` family).
pub(crate) const BOLD_READY_FLAG: &str = "paramex_bold_font_ready";

/// The registered bold family, built once: `FontFamily::Name` holds an
/// `Arc<str>`, so cloning the cached value per call avoids re-allocating the
/// name on every label.
static BOLD_FAMILY: LazyLock<FontFamily> = LazyLock::new(|| FontFamily::Name("bold".into()));

/// The custom bold font family -- only if `theme::install` registered it on this
/// context; otherwise the proportional font (so un-themed harness tests don't panic).
pub fn bold_family(ui: &egui::Ui) -> FontFamily {
    let ready = ui.ctx().data(|d| {
        d.get_temp::<bool>(egui::Id::new(BOLD_READY_FLAG))
            .unwrap_or(false)
    });
    if ready {
        BOLD_FAMILY.clone()
    } else {
        FontFamily::Proportional
    }
}
