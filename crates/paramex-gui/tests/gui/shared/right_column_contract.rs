use egui_kittest::{kittest::Queryable, Harness};
use paramex_core::transfer::Session;
use paramex_gui::app::ParamExApp;
use paramex_gui::ui_kit::CARD_INNER_MARGIN;

use crate::common::{app_harness, parse_transfer_fixture};

const DOUBLE: &str =
    include_str!("../../../../paramex-core/tests/fixtures/parse/corpus_double.csv");

fn many_files_app(count: usize) -> ParamExApp {
    let mut session = Session::new();
    let mut ids = Vec::new();
    for idx in 0..count {
        let id = session
            .add_curve(parse_transfer_fixture(DOUBLE, &format!("A_{idx:02}.csv")))
            .unwrap();
        ids.push(id);
    }
    if let Some(first) = ids.first() {
        assert!(session.select_file(first));
    }

    ParamExApp::from_session(session)
}

fn loaded_app() -> ParamExApp {
    many_files_app(6)
}

#[test]
fn right_column_groups_geometry_and_cox_into_two_setup_cards() {
    let _guard = crate::common::wgpu_guard();
    let harness = app_harness(loaded_app());

    assert!(harness.get_by_label("GEOMETRY").rect().is_positive());
    assert!(harness.get_by_label("COX").rect().is_positive());
    assert!(harness.query_by_label("IMPORT GEOMETRY").is_none());
    assert!(harness.query_by_label("Cox STACK ESTIMATOR").is_none());
}

/// Reveal the `Use Estimated Cox` action: it is HIDDEN until an estimate exists
/// (see `cox_commit::use_estimated_cox_is_hidden_until_an_estimate_exists`), so the
/// visibility guards below must first run the estimator (the default 3.9/300 nm
/// layer always yields a finite estimate).
fn reveal_use_estimated(harness: &mut Harness<'_, ParamExApp>) {
    harness.get_by_label("Estimate Cox").scroll_to_me();
    harness.run();
    harness.get_by_label("Estimate Cox").click();
    harness.run();
    harness.run();
}

#[test]
fn cox_use_estimated_action_is_fully_visible_at_real_window_size() {
    let _guard = crate::common::wgpu_guard();
    let mut harness = app_harness(loaded_app());

    reveal_use_estimated(&mut harness);
    harness.get_by_label("Use Estimated Cox").scroll_to_me();
    harness.run();

    let use_estimated = harness.get_by_label("Use Estimated Cox").rect();
    assert!(
        use_estimated.bottom() <= 780.0,
        "Use Estimated Cox lacks bottom clearance in the real window: {use_estimated:?}"
    );
}

#[test]
fn cox_use_estimated_action_survives_os_framed_window_height() {
    let _guard = crate::common::wgpu_guard();
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1280.0, 700.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            loaded_app()
        });
    harness.run();
    harness.run();

    reveal_use_estimated(&mut harness);
    harness.get_by_label("Use Estimated Cox").scroll_to_me();
    harness.run();

    let use_estimated = harness.get_by_label("Use Estimated Cox").rect();
    assert!(
        use_estimated.bottom() <= 680.0,
        "Use Estimated Cox is too low in a framed-window-height capture: {use_estimated:?}"
    );
}

#[test]
fn right_column_uses_content_sized_cox_card() {
    let _guard = crate::common::wgpu_guard();
    let harness = app_harness(ParamExApp::from_session(Session::new()));

    let selected_title = harness.get_by_label("SELECTED").rect();
    let results_title = harness.get_by_label("RESULTS").rect();
    let cox_title = harness.get_by_label("COX").rect();
    let add_layer = harness.get_by_label("Add Layer").rect();
    let estimate = harness.get_by_label("Estimate Cox").rect();

    assert!(
        cox_title.top() >= selected_title.top() + 16.0
            && cox_title.top() >= results_title.top() + 16.0,
        "Cox setup card should be content-sized, not stretched to the SELECTED/RESULTS band: selected={selected_title:?}, results={results_title:?}, cox={cox_title:?}"
    );
    crate::common::assert_same_raster_edge(
        "Cox action-button top edge",
        add_layer.top(),
        estimate.top(),
        harness.ctx.pixels_per_point(),
    );
    crate::common::assert_same_raster_edge(
        "Cox action-button bottom edge",
        add_layer.bottom(),
        estimate.bottom(),
        harness.ctx.pixels_per_point(),
    );
    assert!(
        add_layer.right() <= estimate.left() - 1.0,
        "Cox action buttons should split the row left-to-right: add={add_layer:?}, estimate={estimate:?}"
    );
    let inner_bottom = 800.0 - CARD_INNER_MARGIN as f32;
    let add_clearance = inner_bottom - add_layer.bottom();
    let estimate_clearance = inner_bottom - estimate.bottom();
    assert!(
        add_clearance >= 2.0 && estimate_clearance >= 2.0,
        "Cox action buttons are clipped by the content-sized card: add={add_layer:?}, estimate={estimate:?}, clearances={add_clearance:.1}/{estimate_clearance:.1}"
    );
}

#[test]
fn compact_right_column_keeps_geometry_table_headers_inside_card() {
    let _guard = crate::common::wgpu_guard();
    let harness = app_harness(ParamExApp::from_session(Session::new()));

    let apply = harness.get_by_label("Apply W/L to All Files").rect();
    let length = harness
        .query_all_by_label("L (µm)")
        .map(|node| node.rect())
        .max_by(|a, b| a.right().total_cmp(&b.right()))
        .expect("an L header/input label should render");

    assert!(
        crate::common::raster_pixel(length.right())
            <= crate::common::raster_pixel(apply.right()),
        "Geometry table headers should fit inside the shared right card: length={length:?}, apply={apply:?}"
    );
    assert!(
        harness.query_by_label("Source").is_none(),
        "Geometry table should not spend narrow right-column width on a Source column"
    );
}
