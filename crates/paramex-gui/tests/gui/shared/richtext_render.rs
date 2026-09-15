//! Visual regression guard for the `<sub>`/`<sup>` renderer.
//!
//! `richtext::layout_sub_sup` builds sub/superscripts from ordinary ASCII at a smaller
//! size + `TextFormat::valign` (font-independent — no Unicode subscript tofu). The unit
//! tests in `richtext.rs` assert the section/valign *mechanism*; this renders
//! the real app label strings to a PNG so the *visible* offset (subscripts low,
//! superscripts high) is guarded against regressions. Update with `UPDATE_SNAPSHOTS=1`.

use egui_kittest::Harness;

#[test]
fn sub_sup_labels_render_with_visible_offset() {
    let mut h = Harness::builder()
        .with_size(egui::Vec2::new(900.0, 560.0))
        .build_ui(|ui| {
            paramex_gui::theme::install(ui.ctx());
            ui.spacing_mut().item_spacing.y = 16.0;
            let base = egui::FontId::proportional(40.0);
            // The real markup strings the app feeds the renderer (selected_metrics /
            // cox / results headers).
            for markup in [
                "V<sub>TH</sub>",
                "C<sub>ox</sub>",
                "\u{00B5}<sub>sat</sub> (cm<sup>2</sup> V<sup>-1</sup> s<sup>-1</sup>)",
                "I<sub>on</sub>/I<sub>off</sub>",
                "SS (mV dec<sup>-1</sup>)",
                "\u{0394}V<sub>TH,hyst</sub>   3 \u{00D7} 10<sup>6</sup>",
            ] {
                ui.label(paramex_gui::richtext::layout_sub_sup(
                    markup,
                    base.clone(),
                    egui::Color32::BLACK,
                ));
            }
        });
    h.run();
    h.run();
    crate::common::snapshot(&mut h, "richtext_labels");
}

/// strip_markup is the renderer's own tag-stripping: identical to LayoutJob.text.
#[test]
fn strip_markup_matches_renderer_text() {
    assert_eq!(paramex_gui::richtext::strip_markup("V<sub>TH</sub>"), "VTH");
    assert_eq!(paramex_gui::richtext::strip_markup("plain"), "plain");
    assert_eq!(paramex_gui::richtext::strip_markup("a < b"), "a < b"); // unclosed '<' stays
}

#[test]
fn every_technical_guide_equation_svg_rasterizes() {
    let names = [
        "transfer-threshold",
        "transfer-subthreshold",
        "transfer-on-off-hysteresis",
        "tlm-current",
        "tlm-regression",
    ];
    for name in names {
        let path = format!("{}/assets/math/{name}.svg", env!("CARGO_MANIFEST_DIR"));
        let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("read {path}: {error}"));
        let image = egui_extras::image::load_svg_bytes(&bytes, &Default::default())
            .unwrap_or_else(|error| panic!("{name}.svg must parse and rasterize: {error}"));
        assert!(
            image.size[0] > 0 && image.size[1] > 0,
            "{name}.svg must have a positive raster size"
        );
        assert!(
            image.pixels.iter().any(|pixel| pixel.a() > 0),
            "{name}.svg must contain visible equation paths"
        );
    }
}
