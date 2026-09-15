//! Render-side guard for the pre-formatted TLM rows: a V_G recompute BETWEEN
//! frames must show the rebuilt result rows on screen (the render path reads
//! `TlmState::rows` and never builds rows itself, so a reducer that forgot to
//! rebuild — or a render path pinned to a stale source — would pass every pure
//! reducer test while the visible table stays stale).

use crate::common::{self, loaded_tlm_app as seed_tlm_app};
use eframe::egui;
use egui_kittest::{kittest::Queryable, Harness};
use paramex_core::tlm::AnalyzedDataset;
use paramex_core::tlm::{Status, TlmDataset};
use paramex_core::transfer::Session;
use paramex_gui::app::ParamExApp;
use paramex_gui::state::Workspace;
use paramex_gui::workspaces::tlm::state::TlmState;

fn seed_tlm_app_with_one_valid_and_one_failed_file() -> ParamExApp {
    let dataset = common::load_tlm_corpus();
    let valid = dataset
        .statuses()
        .iter()
        .find(|row| row.status == Status::Ok)
        .cloned()
        .expect("corpus has a valid workbook");
    let curve = dataset
        .curves()
        .iter()
        .find(|curve| curve.file_path().ends_with(&valid.file))
        .cloned()
        .expect("valid status has a curve");
    let failed = dataset
        .statuses()
        .iter()
        .find(|row| row.status != Status::Ok)
        .cloned()
        .expect("corpus has a failed workbook");
    let dataset = TlmDataset::try_new(dataset.root().to_owned(), vec![curve], vec![valid, failed])
        .expect("subset remains coherent");

    let mut tlm = TlmState::default();
    tlm.install_analyzed(AnalyzedDataset::analyze(dataset));
    let mut app = ParamExApp::from_session(Session::new());
    app.set_active_workspace(Workspace::Tlm);
    app.set_tlm_state(tlm);
    app
}

#[test]
fn vg_recompute_between_frames_renders_the_rebuilt_result_rows() {
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1280.0, 800.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            seed_tlm_app()
        });
    harness.run();
    harness.run();

    // Sanity: the initial analysis' R_c cell is on screen (Results tab default).
    let before_rc_cell = harness.state().tlm().rows().results()[0][1].clone();
    assert!(
        harness.query_all_by_label(&before_rc_cell).next().is_some(),
        "initial R_c cell {before_rc_cell:?} should render"
    );

    // Re-analyze at a DIFFERENT measured V_G between frames (same window size,
    // so only the rows generation invalidates the render-side caches).
    let other = {
        let picker = harness
            .state()
            .tlm()
            .vg_picker()
            .expect("loaded V_G picker");
        let current = picker.selected_vg;
        picker
            .vg_values
            .iter()
            .copied()
            .find(|v| (v - current).abs() > 1e-12)
            .expect("corpus has at least two measured V_G values")
    };
    harness.state_mut().tlm_mut().recompute_at_vg(other);
    harness.run();
    harness.run();

    // The numeric fit cells of the FIRST result row (skipping the group-name
    // cell, which legitimately also appears in GROUPS / the plot pill)
    // now reflect the new analysis.
    let after_row = harness.state().tlm().rows().results()[0].clone();
    for cell in after_row.iter().skip(1).take(4).filter(|c| !c.is_empty()) {
        assert!(
            harness.query_all_by_label(cell).next().is_some(),
            "rebuilt result cell {cell:?} should render after the V_G recompute"
        );
    }
    // And the superseded fit value is gone from the table (the SELECTED
    // card formats its values with a unit suffix, so no false collision).
    if after_row[1] != before_rc_cell {
        assert!(
            harness.query_all_by_label(&before_rc_cell).next().is_none(),
            "stale R_c cell {before_rc_cell:?} still rendered after the recompute"
        );
    }
}

#[test]
fn files_status_column_stays_static_between_empty_and_loaded() {
    let empty = common::app_harness(common::empty_workspace_app(Workspace::Tlm));
    let loaded = common::app_harness(common::loaded_tlm_app());

    common::assert_same_raster_rect(
        "TLM FILES status header",
        empty.get_by_label("status").rect(),
        loaded.get_by_label("status").rect(),
        empty.ctx.pixels_per_point(),
    );
}

#[test]
fn clicking_a_files_remove_x_drops_that_workbook_via_a_real_pointer_click() {
    // The FILES-card ✕ is a hand-painted close-button inside an egui_extras table
    // cell, so only a pointer guard proves the click reaches the reducer.
    // This drives the WHOLE app, so the page-level removal + its success toast both
    // run. The toast keeps egui-notify requesting repaint, so the post-click frames
    // use `run_steps` (a fixed frame count) — `run()` would hit max_steps and panic,
    // which is exactly the case egui_kittest's own error text points here for.
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1280.0, 800.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            seed_tlm_app()
        });
    harness.run();

    let before = harness.state().tlm().rows().status().len();
    assert!(before >= 2, "corpus loaded multiple files");
    assert!(
        harness.get_all_by_label("Remove file").count() >= 2,
        "one ✕ per FILES row"
    );
    harness
        .get_all_by_label("Remove file")
        .next()
        .unwrap()
        .click();
    harness.run_steps(3);

    assert_eq!(
        harness.state().tlm().rows().status().len(),
        before - 1,
        "the clicked workbook was removed and the dataset re-analyzed"
    );
}

#[test]
fn removing_the_final_valid_file_reports_every_status_row_cleared() {
    let mut direct = seed_tlm_app_with_one_valid_and_one_failed_file();
    let valid_file = direct.tlm().rows().status()[0][0].clone();
    assert_eq!(
        direct.tlm_mut().remove_file(&valid_file),
        2,
        "the reducer reports the valid row plus the residual failed row it clears"
    );

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1280.0, 800.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            seed_tlm_app_with_one_valid_and_one_failed_file()
        });
    harness.run();

    let before = harness.state().tlm().rows().status().len();
    assert_eq!(before, 2);
    harness
        .get_all_by_label("Remove file")
        .next()
        .expect("valid FILES row has a remove action")
        .click();
    harness.run_steps(3);

    assert!(!harness.state().tlm().has_dataset());
    let after = harness.state().tlm().rows().status().len();
    assert_eq!(after, 0);
}
