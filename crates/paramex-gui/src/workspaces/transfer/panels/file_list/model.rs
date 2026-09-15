//! Transfer file-list deferred commands and bulk action policy.

use eframe::egui;
use egui_notify::Toasts;

use crate::format_ui::{
    cleared_error_rows, removed_items, ATTACHED_PENDING_OUTPUT_MESSAGE,
    OUTPUT_MOVED_TO_PENDING_MESSAGE, REMOVED_OUTPUT_MESSAGE, REMOVED_PENDING_OUTPUT_MESSAGE,
};
use crate::workspaces::transfer::ingest;
use crate::workspaces::transfer::state::PendingOutput;
use crate::workspaces::transfer::TransferWorkspace;

/// Deferred DATA card actions, collected during render and applied after.
pub(super) enum Cmd {
    LoadTransfer,
    LoadOutput,
    LoadFolder,
    RemoveSelectedOrChecked,
    ClearAll,
    KeepChecked,
    Select(String),
    SetChecked(String, bool),
    DismissError(String),
    AttachPendingOutput(String),
    RemovePendingOutput(String),
    DetachOutput(String),
    RemoveAttachedOutput(String),
}

pub(super) fn apply_commands(
    ctx: &egui::Context,
    workspace: &mut TransferWorkspace,
    toasts: &mut Toasts,
    cmds: Vec<Cmd>,
) {
    for cmd in cmds {
        match cmd {
            Cmd::LoadTransfer => ingest::start_add_files(ctx, &mut workspace.io),
            Cmd::LoadOutput => ingest::start_add_output_files(ctx, &mut workspace.io),
            Cmd::LoadFolder => ingest::start_add_folder(ctx, &mut workspace.io),
            Cmd::RemoveSelectedOrChecked => remove_selected_or_checked(workspace, toasts),
            Cmd::ClearAll => clear_all(workspace, toasts),
            Cmd::KeepChecked => keep_checked(workspace, toasts),
            Cmd::Select(id) => {
                workspace.select_file(&id);
            }
            Cmd::SetChecked(id, checked) => {
                workspace.set_file_checked(&id, checked);
            }
            Cmd::DismissError(id) => {
                workspace.file_rows.dismiss_error(&id);
            }
            Cmd::AttachPendingOutput(id) => {
                if attach_pending_output(workspace, &id) {
                    toasts.success(ATTACHED_PENDING_OUTPUT_MESSAGE);
                } else {
                    toasts.warning("Select a transfer file before attaching output.");
                }
            }
            Cmd::RemovePendingOutput(id) => {
                if remove_pending_output(workspace, &id) {
                    toasts.info(REMOVED_PENDING_OUTPUT_MESSAGE);
                }
            }
            Cmd::DetachOutput(id) => {
                if detach_output_to_pending(workspace, &id) {
                    toasts.info(OUTPUT_MOVED_TO_PENDING_MESSAGE);
                }
            }
            Cmd::RemoveAttachedOutput(id) => {
                if remove_attached_output(workspace, &id) {
                    toasts.info(REMOVED_OUTPUT_MESSAGE);
                }
            }
        }
    }
}

/// "Remove Checked", or "Remove Selected" when none are checked. The dynamic
/// GUI label makes this policy explicit.
fn remove_selected_or_checked(workspace: &mut TransferWorkspace, toasts: &mut Toasts) {
    let removed = workspace.remove_selected_or_checked();
    if removed == 0 {
        toasts.warning("No files selected to remove.");
    } else {
        toasts.info(removed_items(removed, "file"));
    }
}

/// "Keep Checked": remove the unchecked set.
fn keep_checked(workspace: &mut TransferWorkspace, toasts: &mut Toasts) {
    match workspace.keep_checked_files() {
        None => {
            toasts.warning("Check the files you want to keep first.");
        }
        Some(removed) if removed > 0 => {
            toasts.info(removed_items(removed, "file"));
        }
        Some(_) => {}
    }
}

/// "Clear All": remove every file, pending output row, and error row.
fn clear_all(workspace: &mut TransferWorkspace, toasts: &mut Toasts) {
    let had_errors = workspace.file_rows.has_errors();
    let removed = workspace.clear_files();
    let pending = workspace.pending_outputs.len();
    workspace.pending_outputs.clear();
    if had_errors {
        workspace.file_rows.clear_errors();
    }
    if removed > 0 || had_errors || pending > 0 {
        if removed > 0 {
            toasts.info(removed_items(removed, "file"));
        } else if pending > 0 {
            toasts.info("Cleared pending output row(s).");
        } else {
            toasts.info(cleared_error_rows());
        }
    }
}

fn attach_pending_output(workspace: &mut TransferWorkspace, pending_id: &str) -> bool {
    let Some(file_id) = workspace.session.active_file_id() else {
        return false;
    };
    let file_id = file_id.to_string();
    let Some(pos) = workspace
        .pending_outputs
        .iter()
        .position(|row| row.id() == pending_id)
    else {
        return false;
    };
    let pending = workspace.pending_outputs.remove(pos);
    let reason = pending.reason();
    let output = pending.into_dataset();
    match workspace.session.replace_output(&file_id, output) {
        Ok(displaced) => {
            if let Some(displaced) = displaced {
                workspace.retain_detached_output(displaced);
            }
            true
        }
        Err(output) => {
            workspace
                .pending_outputs
                .insert(pos, PendingOutput::new(output, reason));
            false
        }
    }
}

fn detach_output_to_pending(workspace: &mut TransferWorkspace, file_id: &str) -> bool {
    if let Some(output) = workspace.session.take_output(file_id) {
        workspace.retain_detached_output(output);
        true
    } else {
        false
    }
}

fn remove_attached_output(workspace: &mut TransferWorkspace, file_id: &str) -> bool {
    workspace.session.take_output(file_id).is_some()
}

fn remove_pending_output(workspace: &mut TransferWorkspace, pending_id: &str) -> bool {
    let before = workspace.pending_outputs.len();
    workspace
        .pending_outputs
        .retain(|row| row.id() != pending_id);
    workspace.pending_outputs.len() != before
}
