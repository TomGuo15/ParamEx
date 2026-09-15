use paramex_gui::plot_kit;

#[test]
fn plot_chrome_uses_palette_tokens_and_shared_type_scale() {
    let t = paramex_gui::theme::tokens();
    assert_eq!(plot_kit::muted_text_color(), t.ink_soft);
    assert_eq!(plot_kit::title_text_color(), t.ink);
    assert_eq!(
        plot_kit::grid_color(),
        paramex_gui::theme::token_alpha(t.border, plot_kit::GRID_ALPHA)
    );
    assert_eq!(
        plot_kit::tick_font().size,
        paramex_gui::theme::type_scale::MUTED
    );
    assert_eq!(plot_kit::band_stroke(t.primary).color, t.primary);
    assert_eq!(
        plot_kit::band_stroke(t.primary).width,
        plot_kit::BAND_STROKE_WIDTH
    );
}

#[test]
fn full_range_selector_band_edges_are_drawn_inside_plot_bounds() {
    let w = plot_kit::visible_band_window((-2.0, 2.0), (-2.0, 2.0));
    assert!(
        w.0 > -2.0 && w.1 < 2.0,
        "full-range band edges should be nudged inside the plot border: {w:?}"
    );

    let partial = plot_kit::visible_band_window((-1.0, 1.0), (-2.0, 2.0));
    assert_eq!(partial, (-1.0, 1.0));

    let left_touch = plot_kit::visible_band_window((-2.0, 1.0), (-2.0, 2.0));
    assert!(left_touch.0 > -2.0 && (left_touch.1 - 1.0).abs() < f64::EPSILON);
}

#[test]
fn nice_axis_step_keeps_compact_value_axes_labelable() {
    for span in [8.228_931_091_200_734e-6, 4.176_524_350_776_247e-6] {
        let step = plot_kit::nice_axis_step(span);
        let marks = plot_kit::grid_marks(0.0, span, step);
        assert!(
            marks.len() <= 6,
            "compact plot axis should not generate more than six labelled marks: span={span:e} step={step:e} marks={marks:?}"
        );
    }
}

#[test]
fn plot_tick_and_tooltip_helpers_are_shared() {
    assert_eq!(plot_kit::decade_label_step(-17, -2), 3);
    assert_eq!(plot_kit::decade_label_step(-6, -1), 1);
    assert_eq!(plot_kit::decade_label_step(-31, -2), 5);
    assert_eq!(plot_kit::decade_label_step(-3, -3), 1);
    assert_eq!(plot_kit::decade_label_step(-2, -17), 1);
    assert_eq!(plot_kit::numeric_tick_label(false, 1.25), "");
    assert_eq!(plot_kit::numeric_tick_label(true, 1.25), "1.25");
    assert_eq!(plot_kit::engineering_tick_label(false, 3.0e-6), "");
    assert_eq!(plot_kit::engineering_tick_label(true, 3.0e-6), "3\u{00B5}");
    assert_eq!(plot_kit::log_decade_tick_label(true, -6.0, 2), "1e-6");
    assert_eq!(plot_kit::log_decade_tick_label(true, -5.0, 2), "");
    assert_eq!(plot_kit::log_decade_tick_label(false, -6.0, 2), "");
    assert_eq!(
        plot_kit::data_tooltip(
            "Vg 5 V",
            &[("Vd", "2 V".to_string()), ("Id", "3\u{00B5}A".to_string())],
        ),
        "Vg 5 V\nVd 2 V\nId 3\u{00B5}A"
    );
    assert_eq!(
        plot_kit::data_tooltip("", &[("gm", "4\u{00B5}S".to_string())]),
        "gm 4\u{00B5}S"
    );
}
