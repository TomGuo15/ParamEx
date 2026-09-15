//! Bottom results table via `egui_extras::TableBuilder`, plus byte-parity Export CSV.

mod columns;
mod measure;
mod model;
mod output;
mod overall;
mod render;
mod span;

use eframe::egui;

use crate::table_kit::horizontal_table_scroll;
use crate::ui_kit::{self, Variant};
use crate::workspaces::transfer::ingest::{start_export_output_report, start_export_report};
use crate::workspaces::transfer::state::TransferResultsView;
use crate::workspaces::transfer::TransferWorkspace;

pub use columns::{
    col_min_width, col_right_aligned, gui_column_specs, gui_header_label_html, table_min_width,
    GuiColumnSpec,
};
pub(crate) use model::ResultsTableCache;
use output::render_output_results_table_body;
pub(crate) use output::OutputResultsTableCache;
use render::render_results_table_body;

/// Render the results table + Export button.
pub fn show(ui: &mut egui::Ui, ctx: &egui::Context, workspace: &mut TransferWorkspace) {
    let TransferWorkspace {
        session,
        ui: workspace_ui,
        results_cache: cache,
        output_results_cache: output_cache,
        io,
        ..
    } = workspace;
    let mut active_view = workspace_ui.results_view();
    ui_kit::card_slot(ui, |ui| {
        cache.ensure(ui, session);
        output_cache.ensure(ui, session);
        let has_transfer_rows = !cache.is_empty();
        let has_output_rows = !output_cache.is_empty();
        let has_rows = match active_view {
            TransferResultsView::Transfer => has_transfer_rows,
            TransferResultsView::Output => has_output_rows,
        };
        let (clicked, export_clicked) = ui_kit::header_nav_action_row(
            ui,
            "RESULTS",
            |ui| {
                ui.add_enabled_ui(has_transfer_rows || has_output_rows, |ui| {
                    ui_kit::segmented(
                        ui,
                        &["Transfer Fit", "Output Fit"],
                        active_view.index(),
                        ui_kit::SegStyle::Card,
                        None,
                    )
                })
                .inner
            },
            |ui| {
                ui.add_enabled_ui(has_rows && io.is_idle(), |ui| {
                    ui_kit::header_action(ui, "Export CSV", Variant::Primary).clicked()
                })
                .inner
            },
        );
        if let Some(idx) = clicked {
            active_view = TransferResultsView::from_index(idx);
            workspace_ui.set_results_view(active_view);
            ctx.request_repaint();
        }

        if export_clicked {
            match active_view {
                TransferResultsView::Transfer => {
                    let bytes = session.report_bytes();
                    start_export_report(ctx, io, bytes);
                }
                TransferResultsView::Output => {
                    let bytes = session.output_report_bytes();
                    start_export_output_report(ctx, io, bytes);
                }
            }
        }

        match active_view {
            TransferResultsView::Transfer => {
                horizontal_table_scroll(ui, "results_table_scroll", |ui, card_w| {
                    render_results_table_body(ui, cache.rows(), cache.widths(), card_w);
                });
            }
            TransferResultsView::Output => {
                horizontal_table_scroll(ui, "output_results_table_scroll", |ui, card_w| {
                    render_output_results_table_body(
                        ui,
                        output_cache.rows(),
                        output_cache.widths(),
                        card_w,
                    );
                });
            }
        }
    });
}

fn results_empty_notice(ui: &mut egui::Ui, viewport: egui::Rect, message: &str) {
    let row = egui::Rect::from_min_size(
        viewport.min,
        egui::vec2(viewport.width(), ui.spacing().interact_size.y),
    );
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(row)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.set_width(row.width());
            ui.set_height(row.height());
            ui_kit::status_badge_line(
                ui,
                "empty",
                ui_kit::BadgeTone::Warning,
                ui_kit::StatusLineText::Inline(message),
                |_| {},
            );
        },
    );
}

#[cfg(test)]
mod tests {
    use egui_kittest::{
        kittest::{NodeT, Queryable},
        Harness,
    };
    use paramex_core::transfer::{ParsedCurve, Session};

    use super::*;
    use crate::io_tasks::spawn_io;

    struct BusyResultsApp {
        workspace: TransferWorkspace,
    }

    impl eframe::App for BusyResultsApp {
        fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
            let ctx = ui.ctx().clone();
            ui.allocate_ui(egui::vec2(564.0, 220.0), |ui| {
                show(ui, &ctx, &mut self.workspace);
            });
        }
    }

    #[test]
    fn export_is_disabled_while_transfer_io_is_in_flight() {
        let mut session = Session::new();
        session.add_curve(ParsedCurve {
            name: "alpha.csv".to_string(),
            vg: (0..12)
                .map(|index| -1.0 + 5.0 * index as f64 / 11.0)
                .collect(),
            id_abs: (0..12)
                .map(|index| 1e-12 * 10f64.powf(9.0 * index as f64 / 11.0))
                .collect(),
            source_path: None,
        });
        let mut workspace = TransferWorkspace::from_session(session);
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        spawn_io(
            &egui::Context::default(),
            &mut workspace.io,
            "blocked result export test",
            move || {
                let _ = release_rx.recv();
                None
            },
        );

        let mut harness = Harness::builder()
            .with_size(egui::vec2(590.0, 250.0))
            .build_eframe(|cc| {
                crate::theme::install(&cc.egui_ctx);
                BusyResultsApp { workspace }
            });
        harness.run();

        assert!(harness
            .get_by_label("Export CSV")
            .accesskit_node()
            .is_disabled());
        release_tx.send(()).unwrap();
    }
}
