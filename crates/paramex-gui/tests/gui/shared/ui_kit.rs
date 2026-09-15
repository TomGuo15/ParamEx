//! behavioral contracts of the shared UI kit: color roles, rail geometry
//! relations, rendered button/input geometry, and the error-row summaries.

use eframe::egui;
use egui_kittest::{kittest::Queryable, Harness};

#[test]
fn segment_colors_by_style_and_state() {
    use paramex_gui::ui_kit::{segment_colors, SegStyle};
    let t = paramex_gui::theme::tokens();
    assert_eq!(
        segment_colors(SegStyle::Card, true),
        (egui::Color32::TRANSPARENT, t.primary)
    );
    assert_eq!(
        segment_colors(SegStyle::Card, false),
        (egui::Color32::TRANSPARENT, t.ink_soft)
    );
    assert_eq!(segment_colors(SegStyle::Banner, true), (t.surface, t.ink));
    assert_eq!(segment_colors(SegStyle::Banner, false).1, t.surface);
}

/// Card tabs and header actions must sit on the same rail height as the title
/// so a rail with either kind of control never changes height.
#[test]
fn card_title_rail_controls_share_the_rail_height() {
    use paramex_gui::ui_kit::{HEADER_ACTION_HEIGHT, HEADER_RAIL_HEIGHT, HEADER_TAB_HEIGHT};

    assert_eq!(HEADER_TAB_HEIGHT, HEADER_RAIL_HEIGHT);
    assert_eq!(HEADER_ACTION_HEIGHT, HEADER_RAIL_HEIGHT);
}

#[test]
fn standard_button_render_paths_share_height() {
    use paramex_gui::ui_kit::{self, Variant, BUTTON_HEIGHT};

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(240.0, 200.0))
        .build_ui(|ui| {
            ui_kit::button_full(ui, "Clear All", Variant::Danger);
            ui_kit::button_full(ui, "Estimate C<sub>ox</sub>", Variant::Secondary);
            ui_kit::button_full(ui, "Use Estimated C<sub>ox</sub>", Variant::Secondary);
            ui_kit::button(ui, "Export CSV", Variant::Primary);
        });
    harness.run();

    for label in [
        "Clear All",
        "Estimate Cox",
        "Use Estimated Cox",
        "Export CSV",
    ] {
        let rect = harness.get_by_label(label).rect();
        crate::common::assert_same_raster_edge(
            &format!("{label} standard button height"),
            rect.bottom(),
            rect.top() + BUTTON_HEIGHT * harness.ctx.pixels_per_point(),
            harness.ctx.pixels_per_point(),
        );
    }
}

#[test]
fn card_slot_keeps_oversized_content_from_pushing_the_next_slot() {
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(260.0, 180.0))
        .build_ui(|ui| {
            ui.allocate_ui(egui::Vec2::new(220.0, 80.0), |ui| {
                paramex_gui::ui_kit::card_slot(ui, |ui| {
                    for idx in 0..12 {
                        ui.label(format!("overflow row {idx}"));
                    }
                });
            });
            ui.label("AFTER SLOT");
        });
    harness.run();

    let after = harness.get_by_label("AFTER SLOT").rect();
    assert!(
        after.top() <= 92.0,
        "oversized card content pushed the following slot to {after:?}"
    );
}

#[test]
fn selection_row_fill_states() {
    use paramex_gui::ui_kit::selection_row_fill;
    let t = paramex_gui::theme::tokens();
    assert_eq!(selection_row_fill(true, false), t.accent_soft);
    assert_eq!(selection_row_fill(true, true), t.accent_soft); // selected wins
    assert_eq!(selection_row_fill(false, true), t.surface_muted);
    assert_eq!(
        selection_row_fill(false, false),
        eframe::egui::Color32::TRANSPARENT
    );
}

#[test]
fn numeric_inputs_share_one_width_and_alignment_recipe() {
    use paramex_gui::ui_kit::{self, COMPACT_NUMERIC_INPUT_WIDTH};

    let mut fixed = "12.5".to_string();
    let mut pair_left = "12.5".to_string();
    let mut pair_right = "12.5".to_string();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(320.0, 120.0))
        .build_ui(|ui| {
            ui_kit::singleline_edit(ui, &mut fixed, COMPACT_NUMERIC_INPUT_WIDTH);
            let row_w = ui.available_width();
            ui_kit::inline_paired_settings_row_sized(
                ui,
                row_w,
                "W",
                &mut pair_left,
                "L",
                &mut pair_right,
            );
        });
    harness.run();

    let rects: Vec<_> = harness
        .get_all_by_role(egui::accesskit::Role::TextInput)
        .map(|node| node.rect())
        .collect();
    assert_eq!(rects.len(), 3);
    crate::common::assert_same_raster_edge(
        "compact numeric input width",
        rects[0].right(),
        rects[0].left() + COMPACT_NUMERIC_INPUT_WIDTH * harness.ctx.pixels_per_point(),
        harness.ctx.pixels_per_point(),
    );
    for rect in &rects[1..] {
        crate::common::assert_same_raster_span(
            "numeric input height",
            (rect.top(), rect.bottom()),
            (rects[0].top(), rects[0].bottom()),
            harness.ctx.pixels_per_point(),
        );
    }
}

/// In the paired row each markup label sits left of its own input and shares
/// its vertical center at every supported display scale.
#[test]
fn numeric_field_labels_stay_left_of_their_inputs() {
    use paramex_gui::ui_kit;

    for ppp in crate::common::RASTER_TEST_SCALES {
        let mut first = "1".to_string();
        let mut second = "2".to_string();
        let mut harness = Harness::builder()
            .with_size(egui::vec2(280.0, 100.0))
            .with_pixels_per_point(ppp)
            .build_ui(|ui| {
                let row_w = ui.available_width();
                ui_kit::inline_paired_settings_row_sized(
                    ui,
                    row_w,
                    "V<sub>TH</sub>",
                    &mut first,
                    "I<sub>off</sub>",
                    &mut second,
                );
            });
        harness.run();

        let labels = [
            harness.get_by_label("VTH").rect(),
            harness.get_by_label("Ioff").rect(),
        ];
        let mut inputs: Vec<_> = harness
            .get_all_by_role(egui::accesskit::Role::TextInput)
            .map(|node| node.rect())
            .collect();
        inputs.sort_by(|a, b| a.left().total_cmp(&b.left()));
        assert!(
            inputs[0].right() < inputs[1].left(),
            "the paired cells must sit side by side: {inputs:?}"
        );
        for (label, input) in labels.into_iter().zip(inputs) {
            assert!(
                label.right() < input.left(),
                "every numeric field label should sit left of its input: label={label:?}, input={input:?}"
            );
            crate::common::assert_raster_centers_aligned(
                "numeric field label/input baseline",
                label.center().y,
                input.center().y,
                harness.ctx.pixels_per_point(),
            );
        }
    }
}

#[test]
fn selected_row_outline_uses_primary_token() {
    use paramex_gui::ui_kit::selection_row_stroke;
    let t = paramex_gui::theme::tokens();
    assert_eq!(selection_row_stroke(true).color, t.primary);
    assert_eq!(selection_row_stroke(false).color, t.border);
}

#[test]
fn metric_label_value_colors_use_shared_palette_roles() {
    let t = paramex_gui::theme::tokens();
    assert_eq!(paramex_gui::ui_kit::metric_label_color(), t.ink_soft);
    assert_eq!(paramex_gui::ui_kit::metric_value_color(), t.ink);
    assert_eq!(paramex_gui::ui_kit::readout_value_color(), t.ink);
    assert_eq!(paramex_gui::ui_kit::readout_unit_color(), t.ink_soft);
}

#[test]
fn file_error_rows_use_compact_visible_summary() {
    assert_eq!(
        paramex_gui::ui_kit::file_error_summary(
            "No usable transfer curve found in output_curve.xlsx. Check that the file contains Vg and Id columns with at least 12 valid positive-current rows.",
        ),
        "No usable transfer curve"
    );
    assert_eq!(paramex_gui::ui_kit::file_error_summary(""), "Import failed");

    let summary = paramex_gui::ui_kit::file_error_summary(
        "A very long custom diagnostic that should not take over the compact file card. Extra detail follows.",
    );
    assert!(summary.ends_with("..."));
    assert!(summary.chars().count() <= paramex_gui::ui_kit::FILE_ERROR_SUMMARY_MAX_CHARS + 3);
}

#[test]
fn load_error_rows_use_compact_visible_summary() {
    assert_eq!(
        paramex_gui::ui_kit::load_error_summary("No valid TLM workbooks were found."),
        "No valid TLM workbooks"
    );
    assert_eq!(
        paramex_gui::ui_kit::load_error_summary(
            "Could not load the selected folder. Expected folder › group › length-µm › *.xlsx.",
        ),
        "Folder did not match TLM layout"
    );
    assert_eq!(paramex_gui::ui_kit::load_error_summary(""), "Load failed");
}

#[test]
fn slider_thumbs_use_shared_surface_style() {
    let t = paramex_gui::theme::tokens();
    let (fill, stroke) = paramex_gui::ui_kit::control_thumb_style(t.primary);
    assert_eq!(fill, t.surface);
    assert_eq!(stroke.color, t.primary);
    assert_eq!(stroke.width, paramex_gui::ui_kit::CONTROL_THUMB_RING_WIDTH);
    assert_eq!(
        paramex_gui::ui_kit::CONTROL_RAIL_COLOR,
        paramex_gui::theme::UTILITY_GRAY
    );
}

#[test]
fn row_title_colors_use_shared_palette_roles() {
    let t = paramex_gui::theme::tokens();
    assert_eq!(paramex_gui::ui_kit::row_title_color(), t.ink);
}
