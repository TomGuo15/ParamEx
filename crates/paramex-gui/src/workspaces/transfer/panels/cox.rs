//! Right-column COX card: the extraction Cox value (commit only on `>0 && finite`)
//! and a dielectric-stack estimator. Committed Cox lives in
//! `Session::cox_nf_per_cm2`; the layers + estimate label are transient.

use eframe::egui;
use egui_notify::Toasts;
use paramex_core::transfer::Session;

use crate::state::EditBuffers;
use crate::ui_kit;
use crate::workspaces::transfer::state::CoxUi;

mod stack;

/// Commit a typed Cox value, letting the core enforce the positive-finite invariant.
pub fn commit_cox(session: &mut Session, text: &str) -> bool {
    match text.trim().parse::<f64>() {
        Ok(value) => session.set_cox(value).is_ok(),
        _ => false,
    }
}

pub use show as show_setup;

pub fn show(
    ui: &mut egui::Ui,
    session: &mut Session,
    cox: &mut CoxUi,
    edits: &mut EditBuffers,
    toasts: &mut Toasts,
) {
    ui_kit::card_slot(ui, |ui| {
        ui_kit::section_header(ui, "COX", None);
        render_measured_cox_input(ui, session, edits);
        ui_kit::field_label(ui, "Stack estimator");
        stack::render_stack_estimator(ui, session, cox, edits, toasts);
    });
}

fn render_measured_cox_input(ui: &mut egui::Ui, session: &mut Session, edits: &mut EditBuffers) {
    let committed = format!("{}", session.cox_nf_per_cm2());
    // Settings row: label left, fixed-width field at the right edge.
    if let Some(text) = ui_kit::inline_settings_row_commit(
        ui,
        edits,
        "cox:value",
        "Measured C<sub>ox</sub> (nF/cm<sup>2</sup>)",
        &committed,
    ) {
        // An invalid value is skipped silently: the field re-syncs to the committed Cox.
        commit_cox(session, &text);
    }
}
