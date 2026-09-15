//! Tests for the TLM data card helpers and load-error rendering.

use crate::common;
use egui_kittest::{kittest::Queryable, Harness};
use egui_notify::Toasts;
use paramex_gui::state::EditBuffers;
use paramex_gui::workspaces::tlm::panels::data::{commit_fallback_vd, folder_summary};
use paramex_gui::workspaces::tlm::state::TlmState;
use paramex_gui::workspaces::tlm::TlmWorkspace;

#[test]
fn fallback_commit_validates_finite_nonzero() {
    assert_eq!(commit_fallback_vd("-1.5"), Ok(-1.5));
    assert_eq!(commit_fallback_vd(" 2 "), Ok(2.0));
    assert!(commit_fallback_vd("0").is_err());
    assert!(commit_fallback_vd("abc").is_err());
    assert!(commit_fallback_vd("inf").is_err());
    assert!(commit_fallback_vd("").is_err());
}

#[test]
fn fallback_rejections_share_one_typed_ui_message() {
    for input in ["0", "abc", "inf", ""] {
        assert_eq!(
            commit_fallback_vd(input).unwrap_err().to_string(),
            "Fallback VD must be a finite, nonzero number."
        );
    }
}

#[test]
fn folder_summary_shows_basename_and_counts() {
    assert_eq!(
        folder_summary(r"D:\data\RC_run7", 12, 3),
        (
            "RC_run7".to_string(),
            "12 workbooks \u{00B7} 3 groups".to_string()
        )
    );
    assert_eq!(folder_summary("run", 1, 1).0, "run");
}

struct TlmDataHarnessApp {
    workspace: TlmWorkspace,
    edits: EditBuffers,
    toasts: Toasts,
    size: egui::Vec2,
}

impl eframe::App for TlmDataHarnessApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.allocate_ui(self.size, |ui| {
            let ctx = ui.ctx().clone();
            paramex_gui::workspaces::tlm::panels::data::show(
                ui,
                &ctx,
                &mut self.workspace,
                &mut self.edits,
                &mut self.toasts,
            );
        });
    }
}

#[test]
fn empty_tlm_data_card_keeps_clear_slot_disabled() {
    let state = TlmDataHarnessApp {
        workspace: TlmWorkspace::default(),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
        size: egui::Vec2::new(320.0, 300.0),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(360.0, 340.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    harness.get_by_label("Load Folder");
    assert!(harness.get_by_label("Clear All").rect().is_positive());
}

#[test]
fn tlm_data_actions_share_one_compact_row() {
    let state = TlmDataHarnessApp {
        workspace: TlmWorkspace::default(),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
        size: egui::Vec2::new(320.0, 300.0),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(360.0, 340.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    let load = harness.get_by_label("Load Folder").rect();
    let clear = harness.get_by_label("Clear All").rect();
    let pixels_per_point = harness.ctx.pixels_per_point();

    common::assert_same_raster_edge(
        "TLM action-row top",
        load.top(),
        clear.top(),
        pixels_per_point,
    );
    common::assert_same_raster_edge(
        "TLM action-row bottom",
        load.bottom(),
        clear.bottom(),
        pixels_per_point,
    );
    assert!(
        load.right() <= clear.left() - 1.0,
        "TLM actions should split the row left-to-right: load={load:?}, clear={clear:?}"
    );
}

#[test]
fn tlm_data_error_state_keeps_controls_in_empty_slots() {
    let empty = TlmDataHarnessApp {
        workspace: TlmWorkspace::default(),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
        size: egui::Vec2::new(320.0, 300.0),
    };
    let mut error_tlm = TlmState::default();
    error_tlm.push_load_error("No valid TLM workbooks were found.".to_string());
    let error = TlmDataHarnessApp {
        workspace: TlmWorkspace::from_state(error_tlm),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
        size: egui::Vec2::new(320.0, 300.0),
    };

    let mut empty_harness = Harness::builder()
        .with_size(egui::Vec2::new(360.0, 340.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            empty
        });
    empty_harness.run();

    let mut error_harness = Harness::builder()
        .with_size(egui::Vec2::new(360.0, 340.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            error
        });
    error_harness.run();

    for label in ["Fallback VD (V)", "Load Folder", "Clear All"] {
        let empty_rect = empty_harness.get_by_label(label).rect();
        let error_rect = error_harness.get_by_label(label).rect();
        common::assert_same_raster_rect(
            &format!("TLM DATA {label} empty/error stability"),
            empty_rect,
            error_rect,
            empty_harness.ctx.pixels_per_point(),
        );
    }
}

#[test]
fn empty_tlm_data_summary_is_blank_not_dash_text() {
    let state = TlmDataHarnessApp {
        workspace: TlmWorkspace::default(),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
        size: egui::Vec2::new(320.0, 300.0),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(360.0, 340.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    let dash_count = harness.query_all_by_label("\u{2014}").count();
    assert_eq!(
        dash_count, 0,
        "empty DATA summary should reserve the loaded summary slot without painting dash values"
    );
}

#[test]
fn empty_tlm_data_summary_uses_complete_zero_state_copy() {
    let state = TlmDataHarnessApp {
        workspace: TlmWorkspace::default(),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
        size: egui::Vec2::new(320.0, 300.0),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(360.0, 340.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    let folder = harness.get_by_label("No folder loaded").rect();
    let counts = harness.get_by_label("0 workbooks \u{00B7} 0 groups").rect();
    let fallback = harness.get_by_label("Fallback VD (V)").rect();

    assert!(
        crate::common::raster_pixel(folder.bottom())
            <= crate::common::raster_pixel(counts.top())
            && crate::common::raster_pixel(counts.bottom()) + 2
                <= crate::common::raster_pixel(fallback.top()),
        "empty DATA should reserve the loaded summary slot with complete zero-state copy: folder={folder:?}, counts={counts:?}, fallback={fallback:?}"
    );
}

#[test]
fn tlm_load_error_uses_compact_notice() {
    let msg = "No valid TLM workbooks were found.";
    let mut tlm = TlmState::default();
    tlm.push_load_error(msg.to_string());
    let state = TlmDataHarnessApp {
        workspace: TlmWorkspace::from_state(tlm),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
        size: egui::Vec2::new(320.0, 300.0),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(360.0, 340.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    assert!(harness.get_by_label("ERROR").rect().is_positive());
    assert!(
        harness.query_by_label("Folder layout:").is_none(),
        "load-error state should not stack the empty folder instructions above the error notice"
    );

    let message = harness.get_by_label("No valid TLM workbooks").rect();
    assert!(
        message.right() <= 360.0,
        "TLM load-error summary should stay inside the card: {message:?}"
    );
    assert!(
        message.height() <= 18.0,
        "TLM load-error summary should stay one compact line: {message:?}"
    );

    let dismiss = harness.get_by_label("Dismiss").rect();
    assert!(
        dismiss.is_positive() && dismiss.right() <= 360.0,
        "dismiss button should stay reachable inside the TLM data card: {dismiss:?}"
    );
    assert!(harness.get_by_label("Clear All").rect().is_positive());

    harness.get_by_label("Dismiss").click();
    harness.run();
    assert!(!harness.state().workspace.state().has_load_error());
}

#[test]
fn failed_reload_notice_precedes_retained_folder_until_dismissed() {
    let msg = "No valid TLM workbooks were found.";
    let mut tlm = common::loaded_tlm_state();
    let folder = tlm.data_card().folder.expect("loaded folder");
    let folder_name = folder_summary(folder.root, folder.workbooks, folder.groups).0;
    tlm.push_load_error(msg.to_string());
    let state = TlmDataHarnessApp {
        workspace: TlmWorkspace::from_state(tlm),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
        size: egui::Vec2::new(320.0, 300.0),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(360.0, 340.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    harness.get_by_label("ERROR");
    assert!(
        harness.query_by_label(&folder_name).is_none(),
        "a failed reload must take the fixed summary slot until dismissed"
    );
    assert!(harness.state().workspace.state().has_dataset());

    harness.get_by_label("Dismiss").click();
    harness.run();

    assert!(!harness.state().workspace.state().has_load_error());
    harness.get_by_label(&folder_name);
    assert!(harness.state().workspace.state().has_dataset());
}

#[test]
fn tlm_load_error_actions_do_not_overlap_fallback_field() {
    let msg = "Could not load the selected folder. Expected folder › group › length-µm › *.xlsx with List(*) sheets containing vg, abs_id, and abs_is.";
    let mut tlm = TlmState::default();
    tlm.push_load_error(msg.to_string());
    let data_height = paramex_gui::workspaces::tlm::layout::TLM_DATA_CARD_HEIGHT;
    let state = TlmDataHarnessApp {
        workspace: TlmWorkspace::from_state(tlm),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
        size: egui::Vec2::new(320.0, data_height),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(360.0, data_height + 40.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    let fallback = harness.get_by_label("Fallback VD (V)").rect();
    let load = harness.get_by_label("Load Folder").rect();
    let clear = harness.get_by_label("Clear All").rect();
    let pixels_per_point = harness.ctx.pixels_per_point();

    assert!(
        fallback.bottom() <= load.top() - 1.0,
        "fallback field should stay above the load action: fallback={fallback:?}, load={load:?}"
    );
    common::assert_same_raster_edge(
        "TLM load-error action-row top",
        load.top(),
        clear.top(),
        pixels_per_point,
    );
    common::assert_same_raster_edge(
        "TLM load-error action-row bottom",
        load.bottom(),
        clear.bottom(),
        pixels_per_point,
    );
}
