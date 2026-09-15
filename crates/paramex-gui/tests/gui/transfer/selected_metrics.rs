use egui_kittest::{kittest::Queryable, Harness};
use paramex_core::transfer::{ExtractionStatus, MetricResult, Session};
use paramex_gui::app::ParamExApp;
use paramex_gui::format_ui::{fmt_ratio, DASH};
use paramex_gui::workspaces::transfer::panels::selected_metrics;

// ── Render guard: the sweep table must fit the card ───────────────────────────────

const DOUBLE: &str =
    include_str!("../../../../paramex-core/tests/fixtures/parse/corpus_double.csv");

fn selected_result(session: &Session) -> &MetricResult {
    session
        .selected_file_metrics_projection()
        .expect("selected file")
        .result
}

struct SelectedMetricsHarnessApp {
    session: Session,
    size: egui::Vec2,
}

impl eframe::App for SelectedMetricsHarnessApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.allocate_ui(self.size, |ui| {
            selected_metrics::show(ui, &self.session);
        });
    }
}

fn selected_metrics_harness(session: Session) -> Harness<'static, SelectedMetricsHarnessApp> {
    selected_metrics_harness_with_size(session, egui::Vec2::new(280.0, 300.0))
}

fn selected_metrics_harness_with_size(
    session: Session,
    size: egui::Vec2,
) -> Harness<'static, SelectedMetricsHarnessApp> {
    let state = SelectedMetricsHarnessApp { session, size };
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(size.x + 40.0, size.y + 40.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();
    harness.run();
    harness
}

fn loaded_selected_session() -> Session {
    let mut s = Session::new();
    let id = s
        .add_curve(crate::common::parse_transfer_fixture(
            DOUBLE,
            "corpus_double.csv",
        ))
        .unwrap();
    assert!(s.select_file(&id));
    s
}

fn partial_dual_selected_session() -> Session {
    let mut curve = crate::common::partial_transfer_curve("partial_double.csv");
    curve.vg.extend(curve.vg.clone().into_iter().rev());
    curve.id_abs.extend(curve.id_abs.clone().into_iter().rev());
    let mut session = Session::new();
    let id = session.add_curve(curve).unwrap();
    assert!(session.select_file(&id));
    assert!(selected_result(&session).has_backward_sweep);
    session
}

fn partial_selected_session() -> Session {
    let mut s = Session::new();
    let id = s
        .add_curve(crate::common::partial_transfer_curve("partial_curve.csv"))
        .unwrap();
    assert!(s.select_file(&id));
    assert_eq!(selected_result(&s).status, ExtractionStatus::Partial);
    s
}

fn assert_rect_static(label: &str, empty: egui::Rect, loaded: egui::Rect, pixels_per_point: f32) {
    crate::common::assert_same_raster_rect(label, empty, loaded, pixels_per_point);
}

#[test]
fn selected_header_keeps_file_identity_and_warning_visible() {
    let loaded = selected_metrics_harness(loaded_selected_session());
    let mut partial = selected_metrics_harness(partial_selected_session());

    let loaded_file = loaded.get_by_label("corpus_double.csv").rect();
    assert!(
        loaded_file.bottom() < loaded.get_by_label("Device").rect().top(),
        "selected filename should stay in the card header: {loaded_file:?}"
    );

    let status = partial.get_by_label("WARN").rect();
    let file = partial.get_by_label("partial_curve.csv").rect();
    let device = partial.get_by_label("Device").rect();
    let shell = paramex_gui::layout::ShellRects::from_content(egui::Rect::from_min_size(
        egui::Pos2::ZERO,
        egui::vec2(1280.0, 800.0),
    ));
    let inner_right = shell.left.right() - paramex_gui::ui_kit::CARD_INNER_MARGIN as f32;
    assert!(
        crate::common::raster_pixel(status.right())
            <= crate::common::raster_pixel(inner_right)
            && crate::common::raster_pixel(file.right())
                <= crate::common::raster_pixel(status.left())
            && status.bottom() < device.top(),
        "warning filename/status should stay visible in the SELECTED header: file={file:?}, status={status:?}, device={device:?}, inner_right={inner_right:.1}"
    );

    let warning_message = "Some metrics could not be extracted.";
    assert!(partial.query_by_label(warning_message).is_none());
    partial
        .input_mut()
        .events
        .push(egui::Event::PointerMoved(status.center()));
    partial.run();
    partial.run();
    assert!(partial.query_by_label(warning_message).is_some());
}

/// With a dual-sweep file selected, the widest sweep-metric values
/// ("1.00e-16 A", "15.2T") rendered cut off mid-text at the SELECTED
/// card's right edge — the 3-column table's ideal width (~278px) exceeds the
/// card's inner width (~243px) and nothing shrank it. Every value cell must end
/// inside the card.
#[test]
fn dual_sweep_value_cells_stay_inside_the_card() {
    let mut s = Session::new();
    let id1 = s
        .add_curve(crate::common::parse_transfer_fixture(
            DOUBLE,
            "corpus_double.csv",
        ))
        .unwrap();
    assert!(s.select_file(&id1));
    let backward_ratio = fmt_ratio(selected_result(&s).on_off_ratio_backward);
    assert!(!backward_ratio.contains("× 10"));
    let app = ParamExApp::from_session(s);

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1280.0, 800.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            app
        });
    harness.run();
    harness.run();

    let card_left = harness.get_by_label("SELECTED").rect().left();
    // The card's true inner right edge comes from the shell geometry.
    let shell = paramex_gui::layout::ShellRects::from_content(egui::Rect::from_min_size(
        egui::pos2(0.0, 0.0),
        egui::vec2(1280.0, 800.0),
    ));
    let card_right = shell.left.right() - paramex_gui::ui_kit::CARD_INNER_MARGIN as f32;

    // The two widest value strings (accesskit sees the markup-stripped text).
    for text in ["1.00e-16 A".to_string(), backward_ratio] {
        let in_card: Vec<egui::Rect> = harness
            .get_all_by_label(&text)
            .map(|n| n.rect())
            .filter(|r| r.left() < card_right)
            .collect();
        assert!(
            !in_card.is_empty(),
            "expected at least one '{text}' cell in the SELECTED card \
             (label text or layout changed? update this guard)"
        );
        for rect in in_card {
            assert!(
                crate::common::raster_pixel(rect.left()) >= crate::common::raster_pixel(card_left)
                    && crate::common::raster_pixel(rect.right())
                        <= crate::common::raster_pixel(card_right),
                "'{text}' cell spans {:.1}..{:.1} but the card's inner width is \
                 {card_left:.1}..{card_right:.1} — the sweep table overflows the \
                 card edge",
                rect.left(),
                rect.right()
            );
        }
    }
}

/// The SELECTED card's content (Device block + 7-row sweep table) must fit
/// the fixed bottom band vertically — no dormant vertical scrollbar at rest.
/// The last sweep row (I_on/I_off) is the bottom-most content;
/// its value cell must end above the card's inner bottom edge.
#[test]
fn dual_sweep_content_fits_the_card_vertically() {
    let mut s = Session::new();
    let id1 = s
        .add_curve(crate::common::parse_transfer_fixture(
            DOUBLE,
            "corpus_double.csv",
        ))
        .unwrap();
    assert!(s.select_file(&id1));
    let forward_ratio = fmt_ratio(selected_result(&s).on_off_ratio_forward);
    assert!(!forward_ratio.contains("× 10"));
    let app = ParamExApp::from_session(s);

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1280.0, 800.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            app
        });
    harness.run();
    harness.run();

    let shell = paramex_gui::layout::ShellRects::from_content(egui::Rect::from_min_size(
        egui::pos2(0.0, 0.0),
        egui::vec2(1280.0, 800.0),
    ));
    let card_right = shell.left.right() - paramex_gui::ui_kit::CARD_INNER_MARGIN as f32;
    let card_bottom = shell.left.bottom() - paramex_gui::ui_kit::CARD_INNER_MARGIN as f32;

    // The bottom-most row's widest value, keeping only the left card's node.
    let last_row: Vec<egui::Rect> = harness
        .get_all_by_label(&forward_ratio)
        .map(|n| n.rect())
        .filter(|r| r.left() < card_right)
        .collect();
    assert!(
        !last_row.is_empty(),
        "expected the I_on/I_off value in the SELECTED card \
         (label text or layout changed? update this guard)"
    );
    for rect in last_row {
        assert!(
            crate::common::raster_pixel(rect.bottom()) <= crate::common::raster_pixel(card_bottom),
            "the last sweep row ends at {:.1} but the card's inner bottom is \
             {card_bottom:.1} — the SELECTED content overflows vertically \
             (dormant scrollbar at rest)",
            rect.bottom()
        );
    }
}

/// Empty state still renders the real SELECTED structure. It must fit the
/// same fixed card slot as loaded data; otherwise the app rearranges on load in
/// practice because users see clipped/scrolling content before selecting a file.
#[test]
fn empty_structural_rows_fit_the_card_vertically() {
    let app = ParamExApp::from_session(Session::new());
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1280.0, 800.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            app
        });
    harness.run();
    harness.run();

    let shell = paramex_gui::layout::ShellRects::from_content(egui::Rect::from_min_size(
        egui::pos2(0.0, 0.0),
        egui::vec2(1280.0, 800.0),
    ));
    let card_left = shell.left.left() + paramex_gui::ui_kit::CARD_INNER_MARGIN as f32;
    let card_right = shell.left.right() - paramex_gui::ui_kit::CARD_INNER_MARGIN as f32;
    let card_bottom = shell.left.bottom() - paramex_gui::ui_kit::CARD_INNER_MARGIN as f32;
    let last_label = harness
        .get_all_by_label("Ion/Ioff")
        .map(|node| node.rect())
        .filter(|r| {
            crate::common::raster_pixel(r.left()) >= crate::common::raster_pixel(card_left)
                && crate::common::raster_pixel(r.right()) <= crate::common::raster_pixel(card_right)
        })
        .max_by(|a, b| a.bottom().total_cmp(&b.bottom()))
        .expect("expected empty Ion/Ioff row in SELECTED");

    assert!(
        crate::common::raster_pixel(last_label.bottom())
            <= crate::common::raster_pixel(card_bottom),
        "empty SELECTED structure ends at {:.1} but the card's inner bottom is \
         {card_bottom:.1}; the last metric row is clipped before any file is loaded",
        last_label.bottom()
    );
}

#[test]
fn empty_selected_bottom_slack_matches_the_card_margin() {
    let app = ParamExApp::from_session(Session::new());
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1280.0, 800.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            app
        });
    harness.run();
    harness.run();

    let shell = paramex_gui::layout::ShellRects::from_content(egui::Rect::from_min_size(
        egui::pos2(0.0, 0.0),
        egui::vec2(1280.0, 800.0),
    ));
    let card_left = shell.left.left() + paramex_gui::ui_kit::CARD_INNER_MARGIN as f32;
    let card_right = shell.left.right() - paramex_gui::ui_kit::CARD_INNER_MARGIN as f32;
    let card_bottom = shell.left.bottom() - paramex_gui::ui_kit::CARD_INNER_MARGIN as f32;
    let last_label = harness
        .get_all_by_label("Ion/Ioff")
        .map(|node| node.rect())
        .filter(|r| {
            crate::common::raster_pixel(r.left()) >= crate::common::raster_pixel(card_left)
                && crate::common::raster_pixel(r.right()) <= crate::common::raster_pixel(card_right)
        })
        .max_by(|a, b| a.bottom().total_cmp(&b.bottom()))
        .expect("expected empty Ion/Ioff row in SELECTED");

    let bottom_slack = card_bottom - last_label.bottom();
    let max_slack = 2.0;
    assert!(
        bottom_slack <= max_slack,
        "empty SELECTED leaves {bottom_slack:.1}px below the last metric row; \
         expected no more than {max_slack:.1}px so the bottom margin matches \
         the card rhythm"
    );
}

#[test]
fn empty_selected_file_uses_dash_placeholders() {
    let harness = selected_metrics_harness(Session::new());

    let dash_count = harness.query_all_by_label(DASH).count();
    assert!(
        dash_count >= 8,
        "empty Transfer SELECTED should reserve metric rows with dash placeholders"
    );
}

#[test]
fn selected_device_metric_labels_stay_static_between_empty_and_loaded() {
    let empty = selected_metrics_harness(Session::new());
    let loaded = selected_metrics_harness(loaded_selected_session());

    for label in ["W", "L", "W/L", "\u{0394}VTH,hyst"] {
        assert_rect_static(
            label,
            empty.get_by_label(label).rect(),
            loaded.get_by_label(label).rect(),
            empty.ctx.pixels_per_point(),
        );
    }
}

#[test]
fn selected_device_metric_labels_stay_static_between_empty_and_warning() {
    let empty = selected_metrics_harness(Session::new());
    let warning = selected_metrics_harness(partial_selected_session());

    for label in ["W", "L", "W/L", "\u{0394}VTH,hyst"] {
        assert_rect_static(
            label,
            empty.get_by_label(label).rect(),
            warning.get_by_label(label).rect(),
            empty.ctx.pixels_per_point(),
        );
    }
}

#[test]
fn sweep_metric_columns_stay_static_between_loaded_files() {
    let extracted = selected_metrics_harness(loaded_selected_session());
    let partial = selected_metrics_harness(partial_dual_selected_session());
    let single = selected_metrics_harness(partial_selected_session());

    for header in ["Metric", "Forward", "Backward"] {
        assert_rect_static(
            header,
            extracted.get_by_label(header).rect(),
            partial.get_by_label(header).rect(),
            extracted.ctx.pixels_per_point(),
        );
    }
    crate::common::assert_same_raster_edge(
        "single/dual sweep value column left",
        single.get_by_label("Value").rect().left(),
        extracted.get_by_label("Forward").rect().left(),
        extracted.ctx.pixels_per_point(),
    );
}
