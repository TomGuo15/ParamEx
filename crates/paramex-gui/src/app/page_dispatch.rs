//! Workspace page dispatch for the app shell.

use eframe::egui;

use super::ParamExApp;
use crate::layout::ShellRects;
use crate::state::Workspace;
use crate::workspaces;

pub(super) fn show_active_workspace(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    shell: &ShellRects,
    app: &mut ParamExApp,
) {
    match app.active_workspace {
        Workspace::Transfer => {
            workspaces::transfer::show(
                ui,
                ctx,
                shell,
                &mut app.transfer,
                &mut app.edits,
                &mut app.toasts,
            );
        }
        Workspace::Tlm => {
            workspaces::tlm::show(
                ui,
                ctx,
                shell,
                &mut app.tlm,
                &mut app.edits,
                &mut app.toasts,
            );
        }
    }
}
