//! Left-column DATA card: file list + management. Renders rows from `Session` +
//! the ingestion-error side-channel in arrival order; buttons and row actions
//! are collected as deferred commands and applied through the workspace command
//! seam or the ingestion seam (add files/folder) after render.

use eframe::egui;
use egui_notify::Toasts;

use crate::ui_kit::{self, Variant};
use crate::workspaces::transfer::TransferWorkspace;

mod model;
mod rows;

use model::{apply_commands, Cmd};

/// Gap between stacked action rows.
const ACTION_ROW_GAP: f32 = 4.0;
/// Gap between the last action row and the scrolling file rows.
const ROWS_TOP_GAP: f32 = 8.0;

pub fn show(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    workspace: &mut TransferWorkspace,
    toasts: &mut Toasts,
) {
    let mut cmds: Vec<Cmd> = Vec::new();
    ui_kit::card_slot(ui, |ui| {
        ui_kit::section_header(ui, "DATA", None);
        let ingest_idle = workspace.is_idle();
        render_actions(ui, workspace, ingest_idle, &mut cmds);
        ui.add_space(ROWS_TOP_GAP);

        let has_files = workspace.session.has_files();
        let has_error_rows = workspace.file_rows.has_errors();
        let has_pending_outputs = !workspace.pending_outputs.is_empty();
        let rows_h = ui.available_height().max(0.0);
        ui_kit::scroll_body(ui, "file_rows", rows_h, |ui| {
            if has_files || has_error_rows || has_pending_outputs {
                rows::render_rows(
                    ui,
                    &workspace.session,
                    &workspace.file_rows,
                    &workspace.pending_outputs,
                    ingest_idle,
                    &mut cmds,
                );
            }
        });
    });
    apply_commands(ctx, workspace, toasts, cmds);
}

/// The load / remove / keep action rows above the file list.
fn render_actions(
    ui: &mut egui::Ui,
    workspace: &TransferWorkspace,
    ingest_idle: bool,
    cmds: &mut Vec<Cmd>,
) {
    let mut load_transfer = false;
    let mut load_output = false;
    ui.columns(2, |cols| {
        load_transfer = cols[0]
            .add_enabled_ui(ingest_idle, |ui| {
                ui_kit::button_full(ui, "Load Transfer", Variant::Primary)
            })
            .inner
            .clicked();
        load_output = cols[1]
            .add_enabled_ui(ingest_idle, |ui| {
                ui_kit::button_full(ui, "Load Output", Variant::Primary)
            })
            .inner
            .clicked();
    });
    if load_transfer {
        cmds.push(Cmd::LoadTransfer);
    }
    ui.add_space(ACTION_ROW_GAP);
    if ui
        .add_enabled_ui(ingest_idle, |ui| {
            ui_kit::button_full(ui, "Load Folder", Variant::Secondary)
        })
        .inner
        .clicked()
    {
        cmds.push(Cmd::LoadFolder);
    }
    if load_output {
        cmds.push(Cmd::LoadOutput);
    }
    let has_files = workspace.session.has_files();
    let has_error_rows = workspace.file_rows.has_errors();
    let has_pending_outputs = !workspace.pending_outputs.is_empty();
    let show_clear_all =
        workspace.session.file_count() > 1 || has_error_rows || has_pending_outputs;
    let has_checked_files = workspace.session.has_checked_files();
    let has_unchecked_files = workspace.session.has_unchecked_files();
    let can_keep_checked = has_checked_files && has_unchecked_files;
    let remove_label = if has_checked_files {
        "Remove Checked"
    } else {
        "Remove Selected"
    };
    ui.add_space(ACTION_ROW_GAP);
    let mut remove = false;
    let mut clear_all = false;
    ui.columns(2, |cols| {
        remove = cols[0]
            .add_enabled_ui(ingest_idle && has_files, |ui| {
                ui_kit::button_full(ui, remove_label, Variant::Danger)
            })
            .inner
            .clicked();
        clear_all = cols[1]
            .add_enabled_ui(ingest_idle && show_clear_all, |ui| {
                ui_kit::button_full(ui, "Clear All", Variant::Danger)
            })
            .inner
            .clicked();
    });
    if remove {
        cmds.push(Cmd::RemoveSelectedOrChecked);
    }
    if clear_all {
        cmds.push(Cmd::ClearAll);
    }
    ui.add_space(ACTION_ROW_GAP);
    if ui
        .add_enabled_ui(ingest_idle && can_keep_checked, |ui| {
            ui_kit::button_full(ui, "Keep Checked", Variant::Secondary)
        })
        .inner
        .clicked()
    {
        cmds.push(Cmd::KeepChecked);
    }
}

#[cfg(test)]
mod tests {
    use egui_kittest::{
        kittest::{NodeT, Queryable},
        Harness,
    };
    use paramex_core::transfer::{
        AttachOutputOutcome, OutputCurve, OutputDataset, ParsedCurve, Session,
    };

    use super::*;
    use crate::io_tasks::{spawn_io, IoQueue};
    use crate::workspaces::transfer::state::{FileRows, PendingOutputReason};

    struct BusyFileListApp {
        workspace: TransferWorkspace,
        toasts: Toasts,
    }

    impl eframe::App for BusyFileListApp {
        fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
            let ctx = ui.ctx().clone();
            show(ui, &ctx, &mut self.workspace, &mut self.toasts);
        }
    }

    fn curve(name: &str) -> ParsedCurve {
        ParsedCurve {
            name: name.to_owned(),
            vg: (0..12)
                .map(|index| -1.0 + 5.0 * index as f64 / 11.0)
                .collect(),
            id_abs: (0..12)
                .map(|index| 1e-12 * 10f64.powf(9.0 * index as f64 / 11.0))
                .collect(),
            source_path: None,
        }
    }

    fn output(name: &str) -> OutputDataset {
        OutputDataset {
            name: name.to_owned(),
            curves: vec![OutputCurve {
                vg: 4.0,
                vd: vec![0.0, 1.0, 2.0],
                id: vec![1e-6, 2e-6, 3e-6],
            }],
            source_path: None,
        }
    }

    #[test]
    fn load_and_destructive_actions_are_disabled_while_io_is_in_flight() {
        let mut session = Session::new();
        let first = session.add_curve(curve("a.csv")).unwrap();
        let second = session.add_curve(curve("b.csv")).unwrap();
        assert!(matches!(
            session.attach_output(output("a_output.csv")),
            AttachOutputOutcome::Attached {
                displaced: None,
                ..
            }
        ));
        let mut file_rows = FileRows::default();
        file_rows.record_file(first.clone());
        file_rows.record_file(second);

        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let mut io = IoQueue::default();
        spawn_io(
            &egui::Context::default(),
            &mut io,
            "blocked test worker",
            move || {
                let _ = release_rx.recv();
                None
            },
        );

        let mut workspace = TransferWorkspace::from_session(session);
        workspace.io = io;
        workspace.file_rows = file_rows;
        workspace.record_pending_output(output("orphan_output.csv"), PendingOutputReason::NoMatch);
        let state = BusyFileListApp {
            workspace,
            toasts: Toasts::default(),
        };
        let mut harness = Harness::builder()
            .with_size(egui::vec2(300.0, 460.0))
            .build_eframe(|cc| {
                crate::theme::install(&cc.egui_ctx);
                state
            });
        harness.run();

        for label in [
            "Load Transfer",
            "Load Output",
            "Load Folder",
            "Remove Selected",
            "Clear All",
            "Keep Checked",
        ] {
            assert!(
                harness.get_by_label(label).accesskit_node().is_disabled(),
                "{label} should be disabled while another ingest operation is running"
            );
        }
        assert!(harness.query_by_label("Operation in progress...").is_none());

        harness.get_by_label("a.csv").click();
        harness.run();
        assert_eq!(
            harness.state().workspace.session.active_file_id(),
            Some(first.as_str()),
            "busy-state gating keeps harmless row selection available"
        );
        harness
            .get_by_role_and_label(
                egui::accesskit::Role::CheckBox,
                "Mark a.csv for bulk actions",
            )
            .click_accesskit();
        harness.run();
        assert!(
            harness.state().workspace.session.has_checked_files(),
            "busy-state gating keeps harmless bulk selection available"
        );

        for label in [
            "Remove attached output",
            "Detach output",
            "Remove pending output",
            "Attach to Selected",
        ] {
            assert!(
                harness.get_by_label(label).accesskit_node().is_disabled(),
                "{label} should be disabled while another ingest operation is running"
            );
            harness.get_by_label(label).click();
            harness.run();
        }
        assert_eq!(
            harness
                .state()
                .workspace
                .session
                .selected_output_file()
                .and_then(|selected| selected.output)
                .map(|dataset| dataset.name.as_str()),
            Some("a_output.csv"),
            "disabled row actions must not detach or remove the attached output"
        );
        assert_eq!(
            harness.state().workspace.pending_outputs.len(),
            1,
            "disabled row actions must not attach or remove pending output"
        );

        release_tx.send(()).unwrap();
    }
}
