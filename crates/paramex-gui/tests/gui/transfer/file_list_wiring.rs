//! Headless wiring tests: a tiny panel that exercises the real Session removal
//! path the file_list buttons use, asserting state changes after a click.

use egui_kittest::{
    kittest::{NodeT, Queryable},
    Harness,
}; // Queryable trait brings get_by_label into scope
use egui_notify::Toasts;
use paramex_core::transfer::{OutputCurve, OutputDataset, Session};
use paramex_gui::workspaces::transfer::state::PendingOutputReason;
use paramex_gui::workspaces::transfer::TransferWorkspace;

use crate::transfer_curve as curve;

fn output_dataset(name: &str) -> OutputDataset {
    output_dataset_at(name, std::path::PathBuf::from(name))
}

fn output_dataset_at(name: &str, source_path: std::path::PathBuf) -> OutputDataset {
    OutputDataset {
        name: name.to_string(),
        curves: vec![OutputCurve {
            vg: 5.0,
            vd: vec![0.0, 1.0, 2.0, 3.0],
            id: vec![0.0, 1.0e-6, 1.7e-6, 2.5e-6],
        }],
        source_path: Some(source_path),
    }
}

#[test]
fn clear_all_button_empties_the_session() {
    let mut session = Session::new();
    session.add_curve(curve("a.csv"));
    session.add_curve(curve("b.csv"));
    assert_eq!(session.file_count(), 2);
    let state = FileListHarnessApp::new(session);

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(300.0, 460.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    harness.get_by_label("Clear All").click();
    harness.run();

    assert_eq!(harness.state().workspace.session().file_count(), 0);
}

struct FileListHarnessApp {
    workspace: TransferWorkspace,
    toasts: Toasts,
}

impl FileListHarnessApp {
    fn new(session: Session) -> Self {
        Self {
            workspace: TransferWorkspace::from_session(session),
            toasts: Toasts::default(),
        }
    }

    fn with_pending_output(mut self, output: OutputDataset, reason: PendingOutputReason) -> Self {
        self.workspace.record_pending_output(output, reason);
        self
    }

    fn with_ingest_error(mut self, name: &str, message: &str) -> Self {
        self.workspace.record_ingest_error(name, message);
        self
    }
}

impl eframe::App for FileListHarnessApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.allocate_ui(egui::Vec2::new(280.0, 440.0), |ui| {
            let ctx = ui.ctx().clone();
            paramex_gui::workspaces::transfer::panels::file_list::show(
                ui,
                &ctx,
                &mut self.workspace,
                &mut self.toasts,
            );
        });
    }
}

/// Detects the 3px primary-blue selection bar painted on the left edge of a
/// selected file row (primary = #003CFF: r≈0, g≈60, b≈255).
fn is_primary_bar_pixel<P>(pixel: &P) -> bool
where
    P: std::ops::Index<usize, Output = u8>,
{
    pixel[3] > 200 && pixel[0] < 20 && pixel[1] < 100 && pixel[2] > 200
}

#[test]
fn selected_file_row_paints_primary_accent_bar() {
    let mut session = Session::new();
    let id = session.add_curve(curve("a.csv")).unwrap();
    session.select_file(&id);
    let state = FileListHarnessApp::new(session);

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(300.0, 460.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    let filename = harness.get_by_label("a.csv").rect();
    let image = crate::common::render(&mut harness);
    let y = filename.center().y.round() as u32;
    // The 3px primary bar is at the outer-frame left edge, well to the left of
    // the text label (which is inset by the card + frame margins). Search the
    // full row width up to the filename label's left edge.
    let search_right = filename.left().ceil().min(image.width() as f32) as u32;

    (0..search_right)
        .find(|x| is_primary_bar_pixel(image.get_pixel(*x, y)))
        .expect("selected file row should draw a primary-blue 3px accent bar on its left edge");
}

#[test]
fn file_rows_expose_distinct_active_and_bulk_selection_controls() {
    let mut session = Session::new();
    let id_a = session.add_curve(curve("a.csv")).unwrap();
    let id_b = session.add_curve(curve("b.csv")).unwrap();
    assert!(session.select_file(&id_a));
    let state = FileListHarnessApp::new(session);
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(300.0, 460.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    {
        let selected =
            harness.get_by_role_and_label(egui::accesskit::Role::RadioButton, "Select a.csv");
        let target =
            harness.get_by_role_and_label(egui::accesskit::Role::RadioButton, "Select b.csv");
        assert_eq!(
            selected.accesskit_node().toggled(),
            Some(egui::accesskit::Toggled::True)
        );
        assert_eq!(
            target.accesskit_node().toggled(),
            Some(egui::accesskit::Toggled::False)
        );
        for row in [&selected, &target] {
            let node = row.accesskit_node();
            assert!(node.data().supports_action(egui::accesskit::Action::Focus));
            assert!(node.data().supports_action(egui::accesskit::Action::Click));
        }
    }

    {
        let bulk_a = harness.get_by_role_and_label(
            egui::accesskit::Role::CheckBox,
            "Mark a.csv for bulk actions",
        );
        harness.get_by_role_and_label(
            egui::accesskit::Role::CheckBox,
            "Mark b.csv for bulk actions",
        );
        assert!(bulk_a
            .accesskit_node()
            .data()
            .supports_action(egui::accesskit::Action::Click));
        bulk_a.click_accesskit();
    }
    harness.run();
    assert!(
        harness
            .state()
            .workspace
            .session()
            .file_list_row(&id_a)
            .expect("first file row")
            .is_checked
    );

    let before = crate::common::render(&mut harness);
    let target_rect = {
        let target =
            harness.get_by_role_and_label(egui::accesskit::Role::RadioButton, "Select b.csv");
        let rect = target.rect().expand(2.0);
        target.focus();
        rect
    };
    harness.run();
    let focused = crate::common::render(&mut harness);
    let changed = (target_rect.top().max(0.0) as u32
        ..target_rect.bottom().min(focused.height() as f32) as u32)
        .flat_map(|y| {
            (target_rect.left().max(0.0) as u32
                ..target_rect.right().min(focused.width() as f32) as u32)
                .map(move |x| (x, y))
        })
        .filter(|(x, y)| before.get_pixel(*x, *y) != focused.get_pixel(*x, *y))
        .count();
    assert!(changed > 0, "focused file row should paint a visible ring");

    harness.key_press(egui::Key::Enter);
    harness.run();
    harness.run();
    assert_eq!(
        harness.state().workspace.session().active_file_id(),
        Some(id_b.as_str())
    );
}

#[test]
fn empty_file_management_actions_keep_slots_disabled() {
    let state = FileListHarnessApp::new(Session::new());

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(300.0, 460.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    for label in ["Remove Selected", "Clear All", "Keep Checked"] {
        assert!(
            harness.get_by_label(label).accesskit_node().is_disabled(),
            "{label} should render disabled when the file list is empty"
        );
    }
}

#[test]
fn transfer_data_load_actions_use_compact_rows() {
    let state = FileListHarnessApp::new(Session::new());

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(300.0, 460.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    assert!(
        harness.query_by_label("Load Files").is_none(),
        "folder loading should use the explicit Load Folder action label"
    );

    let load_transfer = harness.get_by_label("Load Transfer").rect();
    let load_output = harness.get_by_label("Load Output").rect();
    let load_folder = harness.get_by_label("Load Folder").rect();
    let remove = harness.get_by_label("Remove Selected").rect();
    let clear_all = harness.get_by_label("Clear All").rect();
    let keep_checked = harness.get_by_label("Keep Checked").rect();
    let pixels_per_point = harness.ctx.pixels_per_point();

    crate::common::assert_same_raster_edge(
        "Load Transfer/Output top edge",
        load_transfer.top(),
        load_output.top(),
        pixels_per_point,
    );
    crate::common::assert_same_raster_edge(
        "Load Transfer/Output bottom edge",
        load_transfer.bottom(),
        load_output.bottom(),
        pixels_per_point,
    );
    assert!(
        load_folder.top() > load_transfer.bottom(),
        "Load Folder should sit below the primary load row"
    );
    crate::common::assert_same_raster_edge(
        "Load Folder/Transfer left edge",
        load_folder.left(),
        load_transfer.left(),
        pixels_per_point,
    );
    crate::common::assert_same_raster_edge(
        "Load Folder/Output right edge",
        load_folder.right(),
        load_output.right(),
        pixels_per_point,
    );
    crate::common::assert_same_raster_edge(
        "Remove/Clear All top edge",
        remove.top(),
        clear_all.top(),
        pixels_per_point,
    );
    crate::common::assert_same_raster_edge(
        "Remove/Clear All bottom edge",
        remove.bottom(),
        clear_all.bottom(),
        pixels_per_point,
    );
    assert!(
        keep_checked.top() > remove.bottom(),
        "Keep Checked should stay below destructive actions: keep={keep_checked:?}, remove={remove:?}"
    );
    crate::common::assert_same_raster_edge(
        "Keep Checked/Remove left edge",
        keep_checked.left(),
        remove.left(),
        pixels_per_point,
    );
    crate::common::assert_same_raster_edge(
        "Keep Checked/Clear All right edge",
        keep_checked.right(),
        clear_all.right(),
        pixels_per_point,
    );
}

#[test]
fn single_file_keeps_bulk_clear_all_slot_disabled() {
    let mut session = Session::new();
    session.add_curve(curve("a.csv")).unwrap();
    let state = FileListHarnessApp::new(session);

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(300.0, 460.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    assert!(
        !harness
            .get_by_label("Remove Selected")
            .accesskit_node()
            .is_disabled(),
        "a loaded idle file should keep Remove Selected enabled"
    );
    assert!(
        harness
            .get_by_label("Clear All")
            .accesskit_node()
            .is_disabled(),
        "a single loaded file keeps Clear All reserved but disabled"
    );
}

#[test]
fn clear_all_removes_error_rows_when_no_files_loaded() {
    let state = FileListHarnessApp::new(Session::new())
        .with_ingest_error("bad_device.csv", "No transfer columns found");

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(300.0, 460.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    assert!(
        !harness
            .get_by_label("Clear All")
            .accesskit_node()
            .is_disabled(),
        "Clear All should be available when the visible file list contains error rows"
    );

    harness.get_by_label("Clear All").click();
    harness.run();

    assert!(!harness.state().workspace.has_ingest_errors());
}

/// File-list errors carry verbose parser diagnostics, but the row itself should
/// stay as compact as an OK file row. The full diagnostic is available on hover;
/// the visible card text stays concise.
#[test]
fn long_error_message_is_summarized_and_dismiss_stays_reachable() {
    const LONG_MSG: &str = "No usable transfer curve found in output_curve.xlsx. Check that the file contains Vg and Id columns with at least 12 valid positive-current rows.";
    let state =
        FileListHarnessApp::new(Session::new()).with_ingest_error("output_curve.xlsx", LONG_MSG);
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(300.0, 460.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    let msg = harness.get_by_label("No usable transfer curve").rect();
    assert!(
        msg.right() <= 300.0,
        "the error summary must stay inside the panel, got right edge {:.0}",
        msg.right()
    );
    assert!(
        msg.height() <= 18.0,
        "the file-list error summary should stay one compact line, got height {:.1}",
        msg.height()
    );

    let x = harness.get_by_label("Dismiss").rect();
    assert!(
        x.right() <= 300.0 && x.is_positive(),
        "the dismiss button must stay reachable inside the card"
    );
    harness.get_by_label("Dismiss").click();
    harness.run();
    assert!(
        !harness.state().workspace.has_ingest_errors(),
        "clicking the dismiss button removes the error row"
    );
}

#[test]
fn error_row_content_aligns_with_ok_file_row_content() {
    let mut session = Session::new();
    session.add_curve(curve("ok_device.csv")).unwrap();
    let state = FileListHarnessApp::new(session).with_ingest_error(
        "bad_device.csv",
        "No usable transfer curve found in bad_device.csv.",
    );

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(300.0, 460.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    let ok_title = harness.get_by_label("ok_device.csv").rect();
    let error_title = harness.get_by_label("bad_device.csv").rect();
    crate::common::assert_same_raster_edge(
        "OK and ERROR row-title gutter",
        ok_title.left(),
        error_title.left(),
        harness.ctx.pixels_per_point(),
    );

    let ok_badge = harness.get_by_label("OK").rect();
    let error_badge = harness.get_by_label("ERROR").rect();
    crate::common::assert_same_raster_edge(
        "OK and ERROR badge gutter",
        ok_badge.left(),
        error_badge.left(),
        harness.ctx.pixels_per_point(),
    );
}

#[test]
fn bulk_action_slots_stay_fixed_when_a_file_is_checked() {
    let mut session = Session::new();
    let id_a = session.add_curve(curve("a.csv")).unwrap();
    let id_b = session.add_curve(curve("b.csv")).unwrap();
    assert!(
        !session.has_checked_files(),
        "newly loaded files should start unchecked"
    );

    let state = FileListHarnessApp::new(session);

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(300.0, 460.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    let keep_before = harness.get_by_label("Keep Checked").rect();
    let first_row_before = harness.get_by_label("a.csv").rect();
    assert!(
        harness
            .get_by_label("Keep Checked")
            .accesskit_node()
            .is_disabled(),
        "Keep Checked should reserve its slot while no files are checked"
    );
    harness
        .get_by_role_and_label(
            egui::accesskit::Role::CheckBox,
            "Mark a.csv for bulk actions",
        )
        .click_accesskit();
    harness.run();

    let keep_after = harness.get_by_label("Keep Checked").rect();
    let first_row_after = harness.get_by_label("a.csv").rect();
    assert_eq!(keep_before, keep_after, "Keep Checked must not move");
    assert_eq!(
        first_row_before, first_row_after,
        "checking a file must not shift the scroll body"
    );
    assert!(
        !harness
            .get_by_label("Keep Checked")
            .accesskit_node()
            .is_disabled(),
        "Keep Checked should enable for a checked subset"
    );
    assert!(harness.query_by_label("Remove Selected").is_none());
    harness.get_by_label("Remove Checked").click();
    harness.run();
    assert!(!harness.state().workspace.session().has_file(&id_a));
    assert!(harness.state().workspace.session().has_file(&id_b));
}

#[test]
fn keep_checked_renders_for_a_mixed_checked_set() {
    let mut session = Session::new();
    let id_a = session.add_curve(curve("a.csv")).unwrap();
    session.add_curve(curve("b.csv")).unwrap();
    assert!(session.set_file_checked(&id_a, true));
    assert!(session.has_unchecked_files());

    let state = FileListHarnessApp::new(session);

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(300.0, 460.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    assert!(
        !harness
            .get_by_label("Keep Checked")
            .accesskit_node()
            .is_disabled(),
        "Keep Checked should render once there is a checked subset to keep"
    );
}

#[test]
fn pending_output_row_attaches_to_selected_transfer_file() {
    let mut session = Session::new();
    let id = session.add_curve(curve("device_a.csv")).unwrap();
    session.select_file(&id);
    let state = FileListHarnessApp::new(session).with_pending_output(
        output_dataset("orphan_output.csv"),
        PendingOutputReason::NoMatch,
    );

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(300.0, 460.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    harness.get_by_label("orphan_output.csv");
    harness.get_by_label("No match");
    harness.get_by_label("Attach to Selected").click();
    harness.run();

    assert!(harness.state().workspace.pending_outputs().is_empty());
    assert!(harness
        .state()
        .workspace
        .session()
        .selected_output_file()
        .and_then(|selected| selected.output)
        .is_some());
    assert!(harness
        .state()
        .workspace
        .session()
        .output_report_rows()
        .iter()
        .any(|row| row.fit == paramex_core::transfer::OutputFitKind::Family));
}

#[test]
fn attached_output_row_can_be_detached_from_file_list() {
    let mut session = Session::new();
    let id = session.add_curve(curve("device_a.csv")).unwrap();
    assert!(session
        .replace_output(&id, output_dataset("device_a_output.csv"))
        .is_ok());
    let state = FileListHarnessApp::new(session);

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(300.0, 460.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    harness.get_by_label("device_a_output.csv");
    let detach = harness.get_by_label("Detach output").rect();
    let remove = harness.get_by_label("Remove attached output").rect();
    assert!(
        detach.width() <= 22.0,
        "Detach should render as an icon-only affordance, got {detach:?}"
    );
    assert!(
        remove.left() > detach.right(),
        "attached output X should sit to the right of the detach icon: detach={detach:?}, remove={remove:?}"
    );
    harness.get_by_label("Detach output").click();
    harness.run();

    assert!(harness
        .state()
        .workspace
        .session()
        .selected_output_file()
        .and_then(|selected| selected.output)
        .is_none());
    assert!(harness
        .state()
        .workspace
        .session()
        .output_report_rows()
        .is_empty());
    assert!(harness.query_by_label("Detach output").is_none());
    assert_eq!(harness.state().workspace.pending_outputs().len(), 1);
    assert_eq!(
        harness.state().workspace.pending_outputs()[0].name(),
        "device_a_output.csv"
    );
    assert_eq!(
        harness.state().workspace.pending_outputs()[0].reason(),
        PendingOutputReason::Detached
    );
    harness.get_by_label("Detached");
    harness.get_by_label("Attach to Selected").click();
    harness.run();

    assert!(harness.state().workspace.pending_outputs().is_empty());
    assert!(harness
        .state()
        .workspace
        .session()
        .selected_output_file()
        .and_then(|selected| selected.output)
        .is_some());
}

#[test]
fn explicit_detach_preserves_a_newer_same_source_pending_payload() {
    let source = std::path::PathBuf::from("lot-a/device_a_output.csv");
    let mut session = Session::new();
    let id = session.add_curve(curve("device_a.csv")).unwrap();
    assert!(session
        .replace_output(
            &id,
            output_dataset_at("device_a_output.csv", source.clone())
        )
        .is_ok());
    let state = FileListHarnessApp::new(session).with_pending_output(
        output_dataset_at("device_a_output.csv", source),
        PendingOutputReason::Ambiguous,
    );
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(300.0, 460.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    harness.get_by_label("Detach output").click();
    harness.run();

    assert!(harness
        .state()
        .workspace
        .session()
        .selected_output_file()
        .and_then(|selected| selected.output)
        .is_none());
    assert_eq!(harness.state().workspace.pending_outputs().len(), 1);
    assert_eq!(
        harness.state().workspace.pending_outputs()[0].reason(),
        PendingOutputReason::Ambiguous,
        "the older detached attachment must not replace newer pending data"
    );
}

#[test]
fn attached_output_x_removes_without_pending_row() {
    let mut session = Session::new();
    let id = session.add_curve(curve("device_a.csv")).unwrap();
    assert!(session
        .replace_output(&id, output_dataset("device_a_output.csv"))
        .is_ok());
    let state = FileListHarnessApp::new(session);

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(300.0, 460.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    harness.get_by_label("Remove attached output").click();
    harness.run();

    assert!(harness
        .state()
        .workspace
        .session()
        .selected_output_file()
        .and_then(|selected| selected.output)
        .is_none());
    assert!(harness.state().workspace.pending_outputs().is_empty());
}

#[test]
fn attaching_pending_output_moves_replaced_output_to_pending() {
    let mut session = Session::new();
    let id = session.add_curve(curve("device_a.csv")).unwrap();
    session.select_file(&id);
    assert!(session
        .replace_output(&id, output_dataset("wrong_output.csv"))
        .is_ok());
    let state = FileListHarnessApp::new(session).with_pending_output(
        output_dataset("right_output.csv"),
        PendingOutputReason::Detached,
    );

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(300.0, 460.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    harness.get_by_label("Attach to Selected").click();
    harness.run();

    let output = harness
        .state()
        .workspace
        .session()
        .selected_output_file()
        .and_then(|selected| selected.output)
        .expect("replacement output");
    assert_eq!(output.name, "right_output.csv");
    assert_eq!(harness.state().workspace.pending_outputs().len(), 1);
    assert_eq!(
        harness.state().workspace.pending_outputs()[0].name(),
        "wrong_output.csv"
    );
    assert_eq!(
        harness.state().workspace.pending_outputs()[0].reason(),
        PendingOutputReason::Detached
    );
}

#[test]
fn attaching_pending_output_preserves_a_newer_displaced_source_payload() {
    let source = std::path::PathBuf::from("lot-a/device_a_output.csv");
    let mut session = Session::new();
    let id = session.add_curve(curve("device_a.csv")).unwrap();
    session.select_file(&id);
    assert!(session
        .replace_output(
            &id,
            output_dataset_at("device_a_output.csv", source.clone())
        )
        .is_ok());
    let state = FileListHarnessApp::new(session)
        .with_pending_output(
            output_dataset_at("device_a_output.csv", source),
            PendingOutputReason::Ambiguous,
        )
        .with_pending_output(
            output_dataset_at(
                "replacement_output.csv",
                std::path::PathBuf::from("lot-b/replacement_output.csv"),
            ),
            PendingOutputReason::NoMatch,
        );
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(300.0, 460.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    harness
        .get_all_by_label("Attach to Selected")
        .nth(1)
        .expect("replacement pending row")
        .click();
    harness.run();

    assert_eq!(
        harness
            .state()
            .workspace
            .session()
            .selected_output_file()
            .and_then(|selected| selected.output)
            .expect("replacement attached")
            .name,
        "replacement_output.csv"
    );
    assert_eq!(harness.state().workspace.pending_outputs().len(), 1);
    assert_eq!(
        harness.state().workspace.pending_outputs()[0].reason(),
        PendingOutputReason::Ambiguous,
        "the older displaced attachment must not replace newer pending data"
    );
}

#[test]
fn attaching_an_alias_of_the_same_output_does_not_create_a_detached_row() {
    let crate_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let direct_path = crate_root.join("Cargo.toml");
    let alias_path = crate_root.join("src").join("..").join("Cargo.toml");
    let mut session = Session::new();
    let id = session.add_curve(curve("device_a.csv")).unwrap();
    session.select_file(&id);
    assert!(session
        .replace_output(&id, output_dataset_at("device_a_output.csv", direct_path))
        .is_ok());
    let state = FileListHarnessApp::new(session).with_pending_output(
        output_dataset_at("device_a_output_alias.csv", alias_path),
        PendingOutputReason::Detached,
    );

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(300.0, 460.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    harness.get_by_label("Attach to Selected").click();
    harness.run();

    assert!(harness.state().workspace.pending_outputs().is_empty());
    assert_eq!(
        harness
            .state()
            .workspace
            .session()
            .selected_output_file()
            .and_then(|selected| selected.output)
            .expect("alias reload remains attached")
            .name,
        "device_a_output_alias.csv"
    );
}

#[test]
fn canonical_aliases_share_one_pending_output_row() {
    let crate_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let direct_path = crate_root.join("Cargo.toml");
    let alias_path = crate_root.join("src").join("..").join("Cargo.toml");
    let state = FileListHarnessApp::new(Session::new())
        .with_pending_output(
            output_dataset_at("first-name.csv", direct_path),
            PendingOutputReason::NoMatch,
        )
        .with_pending_output(
            output_dataset_at("rescanned-name.csv", alias_path),
            PendingOutputReason::Ambiguous,
        );

    assert_eq!(state.workspace.pending_outputs().len(), 1);
    assert_eq!(
        state.workspace.pending_outputs()[0].name(),
        "rescanned-name.csv"
    );
    assert_eq!(
        state.workspace.pending_outputs()[0].reason(),
        PendingOutputReason::Ambiguous
    );
}

#[test]
fn pending_output_row_can_be_removed_without_clear_all() {
    let state = FileListHarnessApp::new(Session::new()).with_pending_output(
        output_dataset("orphan_output.csv"),
        PendingOutputReason::NoMatch,
    );

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(300.0, 460.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    harness.get_by_label("orphan_output.csv");
    let attach = harness.get_by_label("Attach to Selected").rect();
    let remove = harness.get_by_label("Remove pending output").rect();
    assert!(
        attach.width() <= 22.0,
        "Attach to Selected should render as an icon-only affordance, got {attach:?}"
    );
    assert!(
        remove.left() > attach.right(),
        "pending output X should sit to the right of Attach to Selected: attach={attach:?}, remove={remove:?}"
    );
    harness.get_by_label("Remove pending output").click();
    harness.run();

    assert!(harness.state().workspace.pending_outputs().is_empty());
    assert!(harness.query_by_label("orphan_output.csv").is_none());
}
