//! Headless full-app snapshots via wgpu: the real `ParamExApp` layout rendered
//! to PNG, so font, theme, and layout regressions are caught pixel-for-pixel
//! instead of by a manual smoke run.
//!
//! Run with `cargo test -p paramex-gui --test app_snapshot`. Each scene is
//! compared against its committed baseline under `tests/snapshots/`; a missing
//! baseline fails the test rather than silently creating one. To accept an
//! intentional pixel change, rerun with `UPDATE_SNAPSHOTS=1` set, review the
//! rewritten PNGs, and commit them; a failed comparison leaves `.new.png` and
//! `.diff.png` artifacts beside the baseline for inspection.

mod common;

use common::loaded_tlm_app as seed_tlm_app;
use egui_kittest::kittest::NodeT;
use egui_kittest::{kittest::Queryable, Harness};
use paramex_core::transfer::{OutputCurve as TransferOutputCurve, OutputDataset, Session};
use paramex_gui::app::ParamExApp;
use paramex_gui::state::Workspace;

const DOUBLE: &str = include_str!("../../paramex-core/tests/fixtures/parse/corpus_double.csv");
const SINGLE: &str = include_str!("../../paramex-core/tests/fixtures/parse/corpus_single_a.csv");

fn painted_text_rects(
    shape: &eframe::egui::epaint::Shape,
    needle: &str,
    rects: &mut Vec<egui::Rect>,
) {
    match shape {
        eframe::egui::epaint::Shape::Text(text) if text.galley.job.text == needle => {
            rects.push(text.visual_bounding_rect());
        }
        eframe::egui::epaint::Shape::Vec(shapes) => {
            for shape in shapes {
                painted_text_rects(shape, needle, rects);
            }
        }
        _ => {}
    }
}

fn painted_text_count(shape: &eframe::egui::epaint::Shape, needle: &str) -> usize {
    let mut rects = Vec::new();
    painted_text_rects(shape, needle, &mut rects);
    rects.len()
}

/// A realistic app state: two loaded files (one selected, double-sweep) plus one
/// ingestion-error row — so file rows, the selected style, the error row, the
/// metric tiles, the results table, and the selector are all on screen.
fn seed_app() -> ParamExApp {
    let mut s = Session::new();
    let id1 = s
        .add_curve(common::parse_transfer_fixture(DOUBLE, "corpus_double.csv"))
        .unwrap();
    s.add_curve(common::parse_transfer_fixture(
        SINGLE,
        "corpus_single_a.csv",
    ))
    .unwrap();
    assert!(s.select_file(&id1));
    let mut app = ParamExApp::from_session(s);
    app.transfer_mut().record_ingest_error(
        "bad_device.csv".to_string(),
        // The REAL two-sentence parse diagnostic (core::parse), so the scenes
        // exercise the wrapped error row, not a short stand-in.
        "No usable transfer curve found in bad_device.csv. Check that the file \
         contains Vg and Id columns with at least 12 valid positive-current rows."
            .to_string(),
    );
    app
}

fn seed_many_files_app() -> ParamExApp {
    let mut s = Session::new();
    let mut ids = Vec::new();
    for idx in 0..24 {
        let id = s
            .add_curve(common::parse_transfer_fixture(
                DOUBLE,
                &format!("A_{idx:02}.csv"),
            ))
            .unwrap();
        ids.push(id);
    }
    if let Some(first) = ids.first() {
        assert!(s.select_file(first));
    }
    ParamExApp::from_session(s)
}

fn seed_selected_warning_app() -> ParamExApp {
    let mut s = Session::new();
    let id = s
        .add_curve(common::partial_transfer_curve("partial_curve.csv"))
        .unwrap();
    assert!(s.select_file(&id));
    ParamExApp::from_session(s)
}

fn seed_transfer_output_app() -> ParamExApp {
    let mut s = Session::new();
    let id = s
        .add_curve(common::parse_transfer_fixture(DOUBLE, "corpus_double.csv"))
        .unwrap();
    assert!(s.select_file(&id));
    assert!(s
        .replace_output(
            &id,
            OutputDataset {
                name: "corpus_double_output.csv".to_string(),
                curves: vec![
                    TransferOutputCurve {
                        vg: 5.0,
                        vd: vec![0.0, 1.0, 2.0, 3.0],
                        id: vec![0.0, 1.0e-6, 1.7e-6, 2.5e-6],
                    },
                    TransferOutputCurve {
                        vg: 1.0,
                        vd: vec![0.0, 1.0, 2.0, 3.0],
                        id: vec![0.0, 0.2e-6, 0.34e-6, 0.5e-6],
                    },
                    TransferOutputCurve {
                        vg: 3.0,
                        vd: vec![0.0, 1.0, 2.0, 3.0],
                        id: vec![0.0, 0.6e-6, 1.02e-6, 1.5e-6],
                    },
                ],
                source_path: None,
            },
        )
        .is_ok());
    ParamExApp::from_session(s)
}

/// Snapshot the seeded Transfer app at `size`. The caller holds the GPU guard.
fn snapshot_seed_app(name: &str, size: egui::Vec2) {
    let mut harness = common::app_harness_at_size(seed_app(), size);
    assert!(harness.query_by_label("Extraction OK.").is_none());
    harness.snapshot(name);
}

fn transfer_output_harness(size: egui::Vec2, app: ParamExApp) -> Harness<'static, ParamExApp> {
    let mut harness = common::app_harness_at_size(app, size);
    harness.get_by_label("Output Fit").click();
    harness.run();
    harness.run();
    harness
}

#[test]
fn banner_exposes_transfer_as_active_workspace_navigation() {
    let _guard = crate::common::wgpu_guard();
    let harness = common::app_harness(seed_app());

    assert!(harness.get_by_label("ParamEx").rect().is_positive());
    // accesskit sees the plain label text: "Transfer"
    assert!(harness.get_by_label("Transfer").rect().is_positive());
    assert!(harness.get_by_label("TLM").rect().is_positive());
    assert!(harness.get_by_label("Technical guide").rect().is_positive());
    assert!(harness.query_by_label("?").is_none());
}

#[test]
fn technical_guide_tabs_show_exact_contracts_and_equations() {
    let _guard = crate::common::wgpu_guard();
    let mut harness = common::app_harness(seed_app());

    harness.get_by_label("Technical guide").click();
    harness.run();
    harness.run();

    assert!(harness.get_by_label("INPUT").rect().is_positive());
    assert!(harness
        .get_by_label("At least 12 measured points across the gate sweep.")
        .rect()
        .is_positive());
    assert!(harness
        .get_by_label("Hysteresis needs forward + reverse sweeps (≥12 points each).")
        .rect()
        .is_positive());
    assert!(harness.query_by_label("DATA GUIDE").is_none());
    assert!(harness
        .query_by_label("Accepted files and pairing rules for each workspace.")
        .is_none());
    harness.snapshot("app_data_guide");

    harness.get_by_label("TLM guide").click();
    harness.run();
    harness.run();
    assert!(harness
        .get_by_label("Numeric folder = channel length in µm (for example, 50 means 50 µm).")
        .rect()
        .is_positive());
    assert!(harness
        .get_by_label("List(*) sheet: vg · abs_id · abs_is")
        .rect()
        .is_positive());
    assert!(harness
        .get_by_label("Setup(*) sheet: VD; else Fallback VD.")
        .rect()
        .is_positive());
    assert!(harness.query_by_label("List*").is_none());
    assert!(harness.query_by_label("Setup*").is_none());
    assert!(harness
        .get_by_label(
            "Need ≥2 lengths (≥3 for R2). Primary: highest-current device per L; median: diagnostic. m is slope (Ω/µm), not sheet resistance."
        )
        .rect()
        .is_positive());
    harness.snapshot("app_data_guide_tlm");

    harness.get_by_label("Close guide").click();
    harness.run();
    harness.run();
}

#[test]
fn technical_guide_blocks_workspace_interaction_until_closed() {
    let _guard = crate::common::wgpu_guard();
    let mut harness = common::app_harness(seed_app());

    let tlm = harness.get_by_label("TLM").rect().center();
    harness.get_by_label("Technical guide").click();
    harness.run();
    harness.run();

    harness
        .input_mut()
        .events
        .push(egui::Event::PointerMoved(tlm));
    for pressed in [true, false] {
        harness.input_mut().events.push(egui::Event::PointerButton {
            pos: tlm,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    harness.run();
    harness.run();

    assert!(
        harness.query_by_label("ANALYSIS").is_none(),
        "the guide must block workspace navigation behind it"
    );
    assert!(
        harness.query_by_label("TECHNICAL GUIDE").is_none(),
        "clicking the modal backdrop should dismiss the guide"
    );
}

/// The app at its real 1280×800 window size — what the user actually sees.
#[test]
fn render_real_window() {
    let _guard = crate::common::wgpu_guard();
    let mut harness = common::app_harness(seed_app());
    assert!(harness.query_by_label("Extraction OK.").is_none());
    for tick in ["-2", "2", "4"] {
        let count: usize = harness
            .output()
            .shapes
            .iter()
            .map(|shape| painted_text_count(&shape.shape, tick))
            .sum();
        assert_eq!(
            count, 2,
            "both production-width selector plots must paint the {tick} V tick"
        );
    }
    harness.snapshot("app_real");
}

#[test]
fn render_transfer_output_window() {
    let _guard = crate::common::wgpu_guard();
    let mut harness =
        transfer_output_harness(egui::Vec2::new(1280.0, 800.0), seed_transfer_output_app());
    let output_labels = harness.get_all_by_label("OUTPUT").count();
    assert!(
        output_labels >= 2,
        "Output Fit should render the OUTPUT panel header in addition to the file-list output badge"
    );
    harness.get_by_label("VG 1 \u{2192} 5 V");
    assert!(harness.get_by_label("Clear All").rect().is_positive());
    let table_top = harness.get_by_label("Output file").rect().bottom();
    for value in ["corpus_double.csv", "corpus_double_output.csv", "Family"] {
        let mut candidates = Vec::new();
        for clipped in &harness.output().shapes {
            let mut rects = Vec::new();
            painted_text_rects(&clipped.shape, value, &mut rects);
            candidates.extend(
                rects
                    .into_iter()
                    .filter(|rect| rect.top() >= table_top)
                    .map(|rect| (rect, clipped.clip_rect)),
            );
        }
        let fully_visible = candidates
            .iter()
            .any(|(rect, clip)| clip.contains_rect(*rect));
        assert!(
            fully_visible,
            "Output results must show the complete identity/Fit value `{value}`: {candidates:?}"
        );
    }
    harness.snapshot("app_transfer_output");
}

/// A tall canvas so the whole center column (incl. the results table, which at
/// 800px can fall below the fold) is visible in one image for inspection.
#[test]
fn render_tall_inspection() {
    let _guard = crate::common::wgpu_guard();
    snapshot_seed_app("app_tall", egui::Vec2::new(1280.0, 1500.0));
}

#[test]
fn render_tall_empty_inspection() {
    let _guard = crate::common::wgpu_guard();
    let mut harness = common::app_harness_at_size(
        ParamExApp::from_session(Session::new()),
        egui::Vec2::new(1280.0, 1500.0),
    );
    harness.snapshot("app_tall_empty");
}

#[test]
fn render_transfer_output_tall_window() {
    let _guard = crate::common::wgpu_guard();
    let mut harness =
        transfer_output_harness(egui::Vec2::new(1280.0, 1500.0), seed_transfer_output_app());
    harness.get_by_label("VG 1 \u{2192} 5 V");
    harness.snapshot("app_transfer_output_tall");
}

#[test]
fn render_transfer_output_tall_empty_window() {
    let _guard = crate::common::wgpu_guard();
    let mut harness = transfer_output_harness(egui::Vec2::new(1280.0, 1500.0), seed_app());
    harness.snapshot("app_transfer_output_tall_empty");
}

/// A maximized-style 1920×1080 window: proves the responsive shell balances the
/// columns (side columns at their caps) and grows the center graphs/table to fill,
/// rather than ballooning one column or overlapping.
#[test]
fn render_wide_window() {
    let _guard = crate::common::wgpu_guard();
    snapshot_seed_app("app_wide", egui::Vec2::new(1920.0, 1080.0));
}

/// Variant buttons must give hover feedback: an explicit `Button::fill` would
/// pin every state, so the state engine routes fills through scoped visuals.
/// Two scenes: a hovered filled-primary (darkened fill) and a hovered
/// outlined-danger (red wash). At-rest renders are covered by the `app_*`
/// baselines, which the state engine must leave identical.
#[test]
fn render_button_hover_primary() {
    let _guard = crate::common::wgpu_guard();
    let mut harness = common::app_harness(seed_app());
    let center = harness.get_by_label("Load Transfer").rect().center();
    harness
        .input_mut()
        .events
        .push(egui::Event::PointerMoved(center));
    harness.run();
    harness.run();
    harness.snapshot("app_button_hover_primary");
}

#[test]
fn render_button_hover_danger() {
    let _guard = crate::common::wgpu_guard();
    let mut harness = common::app_harness(seed_app());
    let center = harness.get_by_label("Clear All").rect().center();
    harness
        .input_mut()
        .events
        .push(egui::Event::PointerMoved(center));
    harness.run();
    harness.run();
    harness.snapshot("app_button_hover_danger");
}

/// An INACTIVE segment washes on hover (Banner: light
/// wash on the ink track; Card: page tint one step darker). The hover is
/// hand-rolled off the previous frame's response, so it needs its own
/// pointer-driven render guards.
#[test]
fn render_segment_hover_banner() {
    let _guard = crate::common::wgpu_guard();
    let mut harness = common::app_harness(seed_app());
    // On the Transfer page the banner's inactive "TLM" segment is the only
    // node with that exact label.
    let center = harness.get_by_label("TLM").rect().center();
    harness
        .input_mut()
        .events
        .push(egui::Event::PointerMoved(center));
    harness.run();
    harness.run();
    harness.snapshot("app_segment_hover_banner");
}

#[test]
fn render_segment_hover_card_tab() {
    let _guard = crate::common::wgpu_guard();
    let mut harness = common::app_harness(seed_tlm_app());
    // The TLM results card's inactive middle tab (markup strips to this text).
    let center = harness.get_by_label("Fits vs VG").rect().center();
    harness
        .input_mut()
        .events
        .push(egui::Event::PointerMoved(center));
    harness.run();
    harness.run();
    harness.snapshot("app_segment_hover_card_tab");
}

#[test]
fn render_many_files_window() {
    let _guard = crate::common::wgpu_guard();
    let mut harness = common::app_harness(seed_many_files_app());
    harness.snapshot("app_many_files");
}

#[test]
fn render_selected_warning_window() {
    let _guard = crate::common::wgpu_guard();
    let mut harness = common::app_harness(seed_selected_warning_app());
    let warning_badge = harness
        .query_all_by_label("WARN")
        .map(|node| node.rect())
        .max_by(|a, b| a.top().total_cmp(&b.top()))
        .expect("SELECTED header should show a WARN badge");
    assert!(warning_badge.is_positive());
    let header_filename = harness
        .query_all_by_label("partial_curve.csv")
        .map(|node| node.rect())
        .max_by(|a, b| a.top().total_cmp(&b.top()))
        .expect("SELECTED header should name the partial result");
    assert!(header_filename.is_positive());
    assert!(harness.get_by_label("Clear All").rect().is_positive());
    harness.snapshot("app_selected_warning");

    harness
        .input_mut()
        .events
        .push(egui::Event::PointerMoved(warning_badge.center()));
    harness.run();
    harness.run();
    let _ = harness.get_by_label("Some metrics could not be extracted.");
}

fn seed_tlm_load_error_app() -> ParamExApp {
    let mut app = ParamExApp::from_session(Session::new());
    app.set_active_workspace(Workspace::Tlm);
    app.tlm_mut()
        .push_load_error("No valid TLM workbooks were found.".to_string());
    app
}

#[test]
fn render_tlm_window() {
    let _guard = crate::common::wgpu_guard();
    let mut harness = common::app_harness(seed_tlm_app());
    assert!(harness
        .query_by_label("Nearest measured gate voltage.")
        .is_none());
    assert!(harness
        .query_by_label("Default: strongest median current.")
        .is_none());
    assert!(harness
        .query_by_label("Used on next Load Folder.")
        .is_none());
    assert!(harness.query_by_label("Fit OK.").is_none());
    harness.get_by_label("VG -40 V");
    harness.snapshot("app_tlm");
}

#[test]
fn render_tlm_clean_group_window() {
    let _guard = crate::common::wgpu_guard();
    let mut app = seed_tlm_app();
    let clean_group = app
        .tlm()
        .group_list()
        .expect("corpus analyzed")
        .groups
        .iter()
        .find(|group| group.warnings.is_empty())
        .map(|group| group.group.clone())
        .expect("corpus contains a clean-fit group");
    assert!(app.tlm_mut().select_group(&clean_group));
    let mut harness = common::app_harness(app);
    harness.get_by_label("Fit quality acceptable.");
    harness.snapshot("app_tlm_clean");
}

#[test]
fn render_tlm_load_error_window() {
    let _guard = crate::common::wgpu_guard();
    let mut harness = common::app_harness(seed_tlm_load_error_app());
    let _ = harness.get_by_label("Dismiss");
    assert!(harness.query_by_label("Folder layout:").is_none());
    assert!(harness
        .get_by_label("Export Sweep CSV")
        .accesskit_node()
        .is_disabled());
    assert!(harness
        .get_by_label("Export TLM CSV")
        .accesskit_node()
        .is_disabled());
    assert!(harness.get_by_label("Clear All").rect().is_positive());
    harness.snapshot("app_tlm_load_error");
}

#[test]
fn tlm_load_error_stays_inside_bento_at_screenshot_height() {
    let _guard = crate::common::wgpu_guard();
    let mut harness =
        common::app_harness_at_size(seed_tlm_load_error_app(), egui::Vec2::new(1280.0, 759.0));

    assert!(harness.query_by_label("Folder layout:").is_none());
    assert!(harness.get_by_label("Clear All").rect().is_positive());
    let groups = harness.get_by_label("GROUPS").rect();
    let results = harness.get_by_label("RESULTS").rect();
    assert!(
        groups.top() <= results.top() - 20.0,
        "TLM load-error state should spend the compact DATA height on GROUPS: GROUPS top {} vs RESULTS top {}",
        groups.top(),
        results.top()
    );
    harness.snapshot("app_tlm_load_error_759");
}

#[test]
fn render_tlm_sweep_tab() {
    // A load always lands on the Results tab (covered by `app_tlm`), so force the
    // V_G-sweep tab over the same seed — without this scene the sweep table body
    // would have zero render coverage.
    let _guard = crate::common::wgpu_guard();
    let mut app = seed_tlm_app();
    app.tlm_mut()
        .set_results_tab(paramex_gui::workspaces::tlm::state::TlmTab::Sweep);
    let mut harness = common::app_harness(app);
    harness.snapshot("app_tlm_sweep");
}

/// The V_G picker strip with its thumb MID-RAIL: guards that the rail stays a
/// plain grey track with NO trailing fill (a fill-to-thumb reads as range
/// semantics, but this strip picks a single point;
/// every other TLM scene has the thumb at index 0 where a regression would be
/// zero-width and invisible). Also exercises a non-default gate voltage
/// through the analysis/results cards.
#[test]
fn render_tlm_mid_vg() {
    let _guard = crate::common::wgpu_guard();
    let mut app = seed_tlm_app();
    let mid = {
        let vgs = app.tlm().vg_picker().expect("corpus analyzed").vg_values;
        vgs[vgs.len() / 2]
    };
    app.tlm_mut().recompute_at_vg(mid);
    let mut harness = common::app_harness(app);
    harness.get_by_label("VG -20 V");
    harness.snapshot("app_tlm_mid_vg");
}

#[test]
fn render_tlm_tall_inspection() {
    let _guard = crate::common::wgpu_guard();
    let mut harness = common::app_harness_at_size(seed_tlm_app(), egui::Vec2::new(1280.0, 1500.0));
    harness.snapshot("app_tlm_tall");
}

#[test]
fn tlm_data_stays_content_fit_at_tall_window() {
    let _guard = crate::common::wgpu_guard();
    let harness = common::app_harness_at_size(seed_tlm_app(), egui::Vec2::new(1280.0, 1500.0));

    let data = harness.get_by_label("DATA").rect();
    let analysis = harness.get_by_label("ANALYSIS").rect();
    let groups = harness.get_by_label("GROUPS").rect();

    assert!(
        analysis.top() - data.top() <= 300.0,
        "DATA absorbed tall-window slack: DATA top {} ANALYSIS top {}",
        data.top(),
        analysis.top()
    );
    assert!(
        groups.top() - analysis.top() <= 230.0,
        "tall-window slack should move below the input pair, not inside DATA: \
         ANALYSIS top {} GROUPS top {}",
        analysis.top(),
        groups.top()
    );
}

#[test]
fn render_empty_window() {
    let _guard = crate::common::wgpu_guard();
    let mut harness = common::app_harness(ParamExApp::from_session(Session::new()));
    harness.snapshot("app_empty");
}

#[test]
fn empty_transfer_window_keeps_primary_actions_visible() {
    let _guard = crate::common::wgpu_guard();
    let harness = common::app_harness(ParamExApp::from_session(Session::new()));

    assert!(harness.get_by_label("Load Transfer").rect().is_positive());
    assert!(harness.get_by_label("Load Output").rect().is_positive());
    assert!(harness.get_by_label("Load Folder").rect().is_positive());
    assert!(harness.get_by_label("FIT").rect().is_positive());
    assert!(harness.get_by_label("SELECTED").rect().is_positive());
    assert!(harness
        .query_by_label("Load or select a transfer curve to see transfer fit.")
        .is_none());
    assert!(harness
        .query_by_label("Load or select a transfer curve to see file metrics.")
        .is_none());
    assert!(harness.get_by_label("RESULTS").rect().is_positive());
    assert!(harness
        .query_by_label("No transfer results to show.")
        .is_none());
    assert!(harness
        .get_by_label("Export CSV")
        .accesskit_node()
        .is_disabled());
    assert!(harness
        .get_by_label("Transfer Fit")
        .accesskit_node()
        .is_disabled());
    assert!(harness
        .get_by_label("Output Fit")
        .accesskit_node()
        .is_disabled());
    assert!(harness.get_by_label("GEOMETRY").rect().is_positive());
    assert!(harness
        .get_by_label("Apply W/L to All Files")
        .accesskit_node()
        .is_disabled());
    assert!(harness
        .get_by_label("Measured Cox (nF/cm2)")
        .rect()
        .is_positive());
    assert!(harness.query_by_label("Remove layer").is_none());
    assert!(harness.get_by_label("Estimate Cox").rect().is_positive());
    assert!(harness.query_by_label("Use Estimated Cox").is_none());
}

/// Clicking the banner TLM segment switches the workspace; the empty TLM page
/// keeps the primary load action and card shell visible.
#[test]
fn tlm_toggle_switches_and_empty_states_render() {
    let _guard = crate::common::wgpu_guard();
    let mut harness = common::app_harness(ParamExApp::from_session(Session::new()));
    harness.get_by_label("TLM").click();
    harness.run();
    let _ = harness.get_by_label("DATA");
    let _ = harness.get_by_label("GROUPS");
    let _ = harness.get_by_label("FIT");
    let _ = harness.get_by_label("RESULTS");
    let _ = harness.get_by_label("SELECTED");
    assert!(harness.query_by_label("Folder layout:").is_none());
    let _ = harness.get_by_label("Load Folder");
}

/// Empty-TLM snapshot: primary actions stay visible while cold-start cards stay quiet.
#[test]
fn render_tlm_empty_window() {
    let _guard = crate::common::wgpu_guard();
    let mut app = ParamExApp::from_session(Session::new());
    app.set_active_workspace(Workspace::Tlm);
    let mut harness = common::app_harness(app);
    assert!(harness
        .get_by_label("Export Sweep CSV")
        .accesskit_node()
        .is_disabled());
    assert!(harness
        .get_by_label("Export TLM CSV")
        .accesskit_node()
        .is_disabled());
    assert!(harness
        .query_by_label("Load TLM workbooks to see TLM results.")
        .is_none());
    assert!(harness
        .query_by_label("Load TLM workbooks to see the TLM fit.")
        .is_none());
    harness.snapshot("app_tlm_empty");
}

#[test]
fn tlm_input_cards_stay_content_fit_at_short_window() {
    let _guard = crate::common::wgpu_guard();
    let harness = common::app_harness_at_size(seed_tlm_app(), egui::Vec2::new(1280.0, 720.0));

    let groups = harness.get_by_label("GROUPS").rect();
    let results = harness.get_by_label("RESULTS").rect();
    assert!(
        groups.top() <= results.top() - 20.0,
        "TLM input cards should stay content-fit at a short window: \
         GROUPS top {} vs RESULTS top {}",
        groups.top(),
        results.top()
    );
}
