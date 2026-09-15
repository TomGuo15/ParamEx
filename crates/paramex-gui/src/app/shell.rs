//! The per-frame app shell: paints the page background and ink banner, renders
//! the brand bar, hands the body to `page_dispatch`, then layers the Technical
//! Guide modal and toasts on top.

use eframe::egui;

use super::ParamExApp;
use super::{brand_bar, help_guide};
use crate::layout::{self, ShellRects};

impl ParamExApp {
    /// Render the whole window layout into `ui`. Shared by the eframe entry
    /// point (`App::ui`) and the headless snapshot test, so the test exercises
    /// the real panel layout rather than a reconstruction.
    pub fn render(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.drain_ingest();

        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, crate::theme::tokens().bg);
        let shell = ShellRects::from_content(ui.max_rect());
        // Paint the ink banner across the whole top rect first: the brand-bar
        // content alone does not fill the width, so a content-sized Frame would
        // leave the right side unpainted. Then render the content on top.
        ui.painter()
            .rect_filled(shell.top, 0.0, crate::theme::tokens().ink);
        let help_was_open = self.show_help;
        layout::show_in_rect(ui, "brand_bar_rect", shell.top, |ui| {
            brand_bar::show(
                ui,
                &mut self.egg,
                &mut self.active_workspace,
                &mut self.show_help,
            );
        });
        let help_just_opened = self.show_help && !help_was_open;
        if help_just_opened {
            self.help_workspace = self.active_workspace;
        }

        super::page_dispatch::show_active_workspace(ui, &ctx, &shell, self);
        help_guide::show_help_window(
            &ctx,
            &mut self.show_help,
            &mut self.help_workspace,
            help_just_opened,
        );

        self.toasts.show(&ctx);
    }
}
