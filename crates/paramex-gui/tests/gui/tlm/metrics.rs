use crate::common;
use eframe::egui;
use egui_kittest::{kittest::Queryable, Harness};
use paramex_core::tlm::AnalyzedDataset;
use paramex_core::tlm::{
    FileStatus, GroupAnalysis, Status, TlmCurve, TlmDataset, TlmFitSummary, TlmSample, VdSource,
};
use paramex_gui::format_ui::DASH;
use paramex_gui::workspaces::tlm::panels::columns::{RESULT_COLS, SWEEP_COLS};
use paramex_gui::workspaces::tlm::panels::labels;
use paramex_gui::workspaces::tlm::panels::metrics::{self, group_tiles};
use paramex_gui::workspaces::tlm::state::TlmState;

struct TlmMetricsHarnessApp {
    tlm: TlmState,
    size: egui::Vec2,
}

impl eframe::App for TlmMetricsHarnessApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.allocate_ui(self.size, |ui| {
            metrics::show(ui, &self.tlm);
        });
    }
}

fn metrics_harness(tlm: TlmState) -> Harness<'static, TlmMetricsHarnessApp> {
    metrics_harness_at_ppp(tlm, 1.0)
}

fn metrics_harness_at_ppp(
    tlm: TlmState,
    pixels_per_point: f32,
) -> Harness<'static, TlmMetricsHarnessApp> {
    let state = TlmMetricsHarnessApp {
        tlm,
        size: egui::Vec2::new(280.0, 310.0),
    };
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(320.0, 350.0))
        .with_pixels_per_point(pixels_per_point)
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();
    harness.run();
    harness
}

fn assert_selected_terminal_row_is_tight(
    harness: &Harness<'_, paramex_gui::app::ParamExApp>,
    detail: &str,
) {
    let shell = paramex_gui::layout::ShellRects::from_content(egui::Rect::from_min_size(
        egui::pos2(0.0, 0.0),
        egui::vec2(1280.0, 800.0),
    ));
    let inset = paramex_gui::ui_kit::CARD_INNER_MARGIN as f32;
    let card_left = shell.right.left() + inset;
    let card_right = shell.right.right() - inset;
    let card_bottom = shell.right.bottom() - inset;
    let terminal = harness
        .get_all_by_label(detail)
        .map(|node| node.rect())
        .filter(|rect| {
            common::raster_pixel(rect.left()) >= common::raster_pixel(card_left)
                && common::raster_pixel(rect.right()) <= common::raster_pixel(card_right)
        })
        .max_by(|a, b| a.bottom().total_cmp(&b.bottom()))
        .unwrap_or_else(|| panic!("expected terminal SELECTED status detail {detail:?}"));
    let bottom_slack = card_bottom - terminal.bottom();

    assert!(
        (0.0..=6.0).contains(&bottom_slack),
        "TLM SELECTED terminal row leaves {bottom_slack:.1}px of inner-bottom slack; expected 0-6px: terminal={terminal:?}, inner_bottom={card_bottom:.1}"
    );
}

fn group() -> GroupAnalysis {
    GroupAnalysis {
        group: "process_a".into(),
        selected_vg: -1.0,
        points: vec![],
        fit: TlmFitSummary {
            intercept_ohm: 1234.0,
            rc_per_contact_ohm: 617.0,
            slope_ohm_per_um: 12.5,
            r_squared: 0.987,
            intercept_median_ohm: 1300.0,
            rc_per_contact_median_ohm: 650.0,
            slope_median_ohm_per_um: 13.0,
            r_squared_median: 0.95,
        },
        warnings: vec![],
    }
}

fn state_with_three_warnings() -> TlmState {
    let root = std::path::PathBuf::from("root");
    let curve = |file: &str, length_um: f64, current: f64| {
        TlmCurve::try_new(
            root.join(file).display().to_string(),
            "process_a".to_owned(),
            length_um,
            vec![TlmSample::try_new(-1.0, current, current).unwrap()],
            -0.5,
            VdSource::Fallback,
        )
        .unwrap()
    };
    let status = |file: &str, length_um: f64| FileStatus {
        file: file.to_owned(),
        group: "process_a".to_owned(),
        length_um: Some(length_um),
        status: Status::Ok,
        message: "Loaded with fallback V_D=-0.5 V".to_owned(),
        vd_source: VdSource::Fallback,
    };
    let dataset = TlmDataset::try_new(
        root.display().to_string(),
        vec![
            curve("10.xlsx", 10.0, 1.0e-6),
            curve("20.xlsx", 20.0, 0.5e-6),
        ],
        vec![status("10.xlsx", 10.0), status("20.xlsx", 20.0)],
    )
    .unwrap();
    let mut state = TlmState::default();
    state.install_analyzed(AnalyzedDataset::analyze(dataset));
    state
}

#[test]
fn group_tiles_returns_exactly_8() {
    let tiles = group_tiles(&group());
    assert_eq!(
        tiles.len(),
        8,
        "expected exactly 8 tiles, got {}",
        tiles.len()
    );
}

#[test]
fn tile_order_matches_spec() {
    let tiles = group_tiles(&group());
    assert_eq!(tiles[0].0, labels::TILE_RCONTACT);
    assert_eq!(tiles[1].0, labels::TILE_RC_PER_CONTACT);
    assert_eq!(tiles[2].0, labels::TILE_SLOPE);
    assert_eq!(tiles[3].0, labels::TILE_R2);
    assert_eq!(tiles[4].0, labels::TILE_RCONTACT_MED);
    assert_eq!(tiles[5].0, labels::TILE_RC_PER_CONTACT_MED);
    assert_eq!(tiles[6].0, labels::TILE_SLOPE_MED);
    assert_eq!(tiles[7].0, labels::TILE_R2_MED);

    let actual: Vec<&str> = tiles.iter().map(|(label, _)| *label).collect();
    assert_eq!(actual, labels::TILE_LABELS);
}

#[test]
fn fit_labels_distinguish_intercept_from_per_contact_resistance() {
    let intercept = "intercept (2R<sub>c</sub>)";

    assert_eq!(RESULT_COLS[1].label, format!("{intercept} (\u{2126})"));
    assert_eq!(SWEEP_COLS[2].label, format!("{intercept} (\u{2126})"));
    assert_eq!(labels::TILE_RCONTACT, format!("{intercept} (max)"));
    assert_eq!(labels::TILE_RCONTACT_MED, format!("{intercept} (median)"));
}

#[test]
fn no_label_contains_literal_underscore() {
    let tiles = group_tiles(&group());
    for (label, _) in &tiles {
        assert!(
            !label.contains('_'),
            "literal underscore in tile label: {label:?}"
        );
    }
}

#[test]
fn nan_renders_as_dash() {
    let mut g = group();
    g.fit.r_squared = f64::NAN;
    let tiles = group_tiles(&g);
    // tile[3] is TILE_R2
    assert_eq!(tiles[3].0, labels::TILE_R2);
    assert_eq!(tiles[3].1, DASH);
}

#[test]
fn empty_tlm_selected_group_uses_dash_placeholders() {
    let harness = metrics_harness(TlmState::default());

    let dash_count = harness.query_all_by_label(DASH).count();
    assert_eq!(
        dash_count, 8,
        "empty TLM SELECTED should keep dashes in metric rows, not in the header badge"
    );
}

#[test]
fn selected_group_columns_stay_static_between_empty_and_loaded() {
    let empty = metrics_harness(TlmState::default());
    let loaded_state = common::loaded_tlm_state();
    let loaded_value = group_tiles(
        loaded_state
            .selected_group_analysis()
            .expect("loaded TLM group"),
    )[0]
    .1
    .clone();
    let loaded = metrics_harness(loaded_state);

    for label in labels::TILE_LABELS {
        let text = paramex_gui::richtext::strip_markup(label);
        common::assert_same_raster_rect(
            &text,
            empty.get_by_label(&text).rect(),
            loaded.get_by_label(&text).rect(),
            empty.ctx.pixels_per_point(),
        );
    }
    let empty_value = empty
        .query_all_by_label(DASH)
        .next()
        .expect("first empty metric value");
    common::assert_same_raster_edge(
        "TLM SELECTED value column left",
        empty_value.rect().left(),
        loaded.get_by_label(&loaded_value).rect().left(),
        empty.ctx.pixels_per_point(),
    );
}

#[test]
fn selected_group_rows_keep_uniform_raster_spacing() {
    for pixels_per_point in common::RASTER_TEST_SCALES {
        let harness = metrics_harness_at_ppp(TlmState::default(), pixels_per_point);
        let tops: Vec<_> = labels::TILE_LABELS
            .iter()
            .map(|label| {
                harness
                    .get_by_label(&paramex_gui::richtext::strip_markup(label))
                    .rect()
                    .top()
            })
            .collect();
        let first = tops[0];
        for (idx, top) in tops.into_iter().enumerate() {
            common::assert_same_raster_edge(
                &format!("TLM SELECTED metric row {idx} top at {pixels_per_point} ppp"),
                top,
                first + idx as f32 * 24.0 * pixels_per_point,
                pixels_per_point,
            );
        }
    }
}

#[test]
fn loaded_selected_group_content_fits_the_shared_bottom_band() {
    let harness = common::app_harness(common::loaded_tlm_app());
    assert_selected_terminal_row_is_tight(&harness, "1 fit warning; see Results.");
}

#[test]
fn multiple_warning_summary_stays_compact() {
    let mut app = common::empty_workspace_app(paramex_gui::state::Workspace::Tlm);
    app.set_tlm_state(state_with_three_warnings());
    let harness = common::app_harness(app);

    assert_selected_terminal_row_is_tight(&harness, "3 fit warnings; see Results.");
}

#[test]
fn empty_selected_group_keeps_a_tight_terminal_status_row() {
    let harness = common::app_harness(common::empty_workspace_app(
        paramex_gui::state::Workspace::Tlm,
    ));

    assert_selected_terminal_row_is_tight(&harness, "No group selected.");
}

#[test]
fn clean_selected_group_keeps_a_tight_terminal_status_row() {
    let mut app = common::loaded_tlm_app();
    let clean_group = app
        .tlm()
        .group_list()
        .expect("loaded TLM groups")
        .groups
        .iter()
        .find(|group| group.warnings.is_empty())
        .map(|group| group.group.clone())
        .expect("fixture should contain a clean-fit group");
    assert!(app.tlm_mut().select_group(&clean_group));
    let harness = common::app_harness(app);

    assert_selected_terminal_row_is_tight(&harness, "Fit quality acceptable.");
}
