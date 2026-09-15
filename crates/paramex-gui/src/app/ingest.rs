//! App-shell polling for workspace-owned worker queues.

use egui_notify::Toasts;

use super::ParamExApp;
use crate::state::Workspace;

impl ParamExApp {
    /// Poll every workspace at the top of the frame. Hidden results still
    /// commit, but their transient notices go to a discarded sink.
    pub(super) fn drain_ingest(&mut self) {
        let active = self.active_workspace;
        let mut offscreen = Toasts::default();

        let toasts = if active == Workspace::Transfer {
            &mut self.toasts
        } else {
            &mut offscreen
        };
        self.transfer.drain_ingest(toasts);

        let toasts = if active == Workspace::Tlm {
            &mut self.toasts
        } else {
            &mut offscreen
        };
        self.tlm.drain_ingest(toasts);
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use paramex_core::transfer::Session;

    use super::*;
    use crate::workspaces::transfer::state::FileRow;

    #[test]
    fn hidden_worker_panic_releases_busy_gate_and_persists_in_its_workspace() {
        let ctx = egui::Context::default();
        let mut app = ParamExApp::from_session(Session::new());
        app.active_workspace = Workspace::Tlm;
        crate::io_tasks::spawn_io(
            &ctx,
            &mut app.transfer.io,
            "Transfer synthetic load",
            || -> Option<crate::workspaces::transfer::ingest::Msg> {
                panic!("synthetic hidden worker panic")
            },
        );

        let deadline = Instant::now() + Duration::from_secs(2);
        while app.transfer.is_busy() && Instant::now() < deadline {
            app.drain_ingest();
            std::thread::sleep(Duration::from_millis(5));
        }

        assert!(app.transfer.is_idle());
        assert!(matches!(
            app.transfer.file_rows.rows().next(),
            Some(FileRow::Error {
                name: "Transfer synthetic load",
                message: "Background operation failed unexpectedly. Please try again.",
                ..
            })
        ));
        assert!(
            !app.tlm.state.has_load_error(),
            "hidden Transfer failure must not leak into the visible TLM surface"
        );
    }
}
