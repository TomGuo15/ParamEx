//! DATA card helper and deferred-command policy.

use eframe::egui;
use egui_notify::Toasts;
use paramex_core::tlm::{valid_vd, TlmParseError};

use crate::format_ui::{cleared_error_rows, removed_items};
use crate::workspaces::tlm::ingest::start_load_tlm_folder;
use crate::workspaces::tlm::TlmWorkspace;

const FALLBACK_VD_REJECTION: &str = "Fallback VD must be a finite, nonzero number.";

fn fallback_vd_rejection() -> TlmParseError {
    TlmParseError(FALLBACK_VD_REJECTION.to_owned())
}

/// Parse and validate a fallback-V_D field. The DATA card is the only
/// production caller; `apply_commands` then commits the value to the session.
pub fn commit_fallback_vd(text: &str) -> Result<f64, TlmParseError> {
    let value = text
        .trim()
        .parse::<f64>()
        .map_err(|_| fallback_vd_rejection())?;
    valid_vd(value, "Fallback V_D").map_err(|_| fallback_vd_rejection())
}

/// `(folder basename, "N workbooks · M groups")` for the TLM Data card.
pub fn folder_summary(root: &str, workbooks: usize, groups: usize) -> (String, String) {
    let base = std::path::Path::new(root)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(root)
        .to_string();
    (
        base,
        format!("{workbooks} workbooks \u{00B7} {groups} groups"),
    )
}

/// Deferred data-card actions (the Transfer deferred-commit pattern).
pub(super) enum Cmd {
    Load,
    Clear,
    DismissLoadError,
    Fallback(f64),
    FallbackInvalid,
}

pub(super) fn apply_commands(
    ctx: &egui::Context,
    workspace: &mut TlmWorkspace,
    toasts: &mut Toasts,
    cmds: Vec<Cmd>,
) {
    for cmd in cmds {
        match cmd {
            Cmd::Load => {
                start_load_tlm_folder(ctx, &mut workspace.io, Some(workspace.state.fallback_vd()))
            }
            Cmd::Clear => {
                let data = workspace.state.data_card();
                let count = data.folder.map_or(0, |folder| folder.workbooks);
                let had_load_error = data.load_error.is_some();
                workspace.state.clear();
                if count > 0 {
                    toasts.info(removed_items(count, "file"));
                } else if had_load_error {
                    toasts.info(cleared_error_rows());
                }
            }
            Cmd::DismissLoadError => workspace.state.dismiss_load_error(),
            // `commit_fallback_vd` already validated the value and the session
            // re-checks the same contract, so a rejection here cannot happen;
            // it is surfaced anyway rather than swallowed.
            Cmd::Fallback(value) => {
                if workspace.state.set_fallback_vd(value).is_err() {
                    toasts.warning(FALLBACK_VD_REJECTION);
                }
            }
            Cmd::FallbackInvalid => {
                toasts.warning(FALLBACK_VD_REJECTION);
            }
        }
    }
}
