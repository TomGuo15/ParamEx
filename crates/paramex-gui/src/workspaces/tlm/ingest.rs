//! TLM workbook load/analyze and CSV export workers plus their UI-thread
//! effects.

use std::path::{Path, PathBuf};

use eframe::egui;
use egui_notify::Toasts;
use paramex_core::tlm;

use crate::format_ui::{exported_to, loaded_files};
use crate::io_tasks::{saved_file_name, IoEvent, IoQueue};
use crate::workspaces::tlm::state::TlmState;
use crate::workspaces::tlm::TlmWorkspace;
use paramex_core::tlm::AnalyzedDataset;

/// One terminal result from a TLM worker.
pub(crate) enum Msg {
    DatasetLoaded {
        result: Result<Box<AnalyzedDataset>, String>,
    },
    CsvExported {
        result: Result<PathBuf, String>,
    },
}

/// Load and analyze every TLM workbook under `root` on a worker thread.
pub(crate) fn load_tlm_analyzed(
    root: &Path,
    fallback_vd: Option<f64>,
) -> Result<Box<AnalyzedDataset>, String> {
    let dataset = tlm::load_dataset(root, fallback_vd).map_err(|e| e.0)?;
    Ok(Box::new(AnalyzedDataset::analyze(dataset)))
}

/// "Load Folder": open the folder picker (blocking) on a worker thread, load
/// and analyze the TLM dataset, and post one terminal message.
pub(crate) fn start_load_tlm_folder(
    ctx: &egui::Context,
    queue: &mut IoQueue<Msg>,
    fallback_vd: Option<f64>,
) {
    crate::io_tasks::spawn_io(ctx, queue, "TLM folder load", move || {
        rfd::FileDialog::new()
            .set_title("Load a folder of TLM workbooks")
            .pick_folder()
            .map(|dir| Msg::DatasetLoaded {
                result: load_tlm_analyzed(&dir, fallback_vd),
            })
    });
}

/// Save TLM CSV bytes (gathered on the UI thread via `result_csv`/`sweep_csv`)
/// to an rfd-chosen `.csv` path.
pub(crate) fn start_export_tlm_csv(
    ctx: &egui::Context,
    queue: &mut IoQueue<Msg>,
    bytes: Vec<u8>,
    default_name: &'static str,
) {
    crate::io_tasks::start_save_csv(
        ctx,
        queue,
        bytes,
        default_name,
        "Export TLM CSV",
        |result| Msg::CsvExported { result },
    );
}

pub(crate) fn drain(workspace: &mut TlmWorkspace, toasts: &mut Toasts) {
    for event in workspace.io.drain_events() {
        match event {
            IoEvent::Message(Msg::DatasetLoaded { result }) => {
                apply_dataset_loaded(result, &mut workspace.state, toasts)
            }
            IoEvent::Message(Msg::CsvExported { result }) => {
                apply_csv_exported(result, &mut workspace.state, toasts)
            }
            IoEvent::WorkerFailed(failure) => {
                workspace.state.push_load_error(failure.notice());
                toasts.error(failure.notice());
            }
        }
    }
}

fn apply_dataset_loaded(
    result: Result<Box<AnalyzedDataset>, String>,
    tlm: &mut TlmState,
    toasts: &mut Toasts,
) {
    match result {
        Ok(analyzed) => {
            let n = analyzed.workbook_count();
            tlm.install_analyzed(*analyzed);
            toasts.success(loaded_files(n));
        }
        Err(message) => {
            tlm.push_load_error(message.clone());
            toasts.error(message);
        }
    }
}

fn apply_csv_exported(result: Result<PathBuf, String>, state: &mut TlmState, toasts: &mut Toasts) {
    match result {
        Ok(path) => {
            toasts.success(exported_to(&saved_file_name(&path)));
        }
        Err(message) => {
            tracing::error!("TLM CSV export failed: {message}");
            state.push_load_error(format!("CSV export: {message}"));
            toasts.error(message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_failure_persists_after_toast_sink_is_dropped() {
        let mut state = TlmState::default();
        apply_csv_exported(
            Err("Access denied".to_owned()),
            &mut state,
            &mut Toasts::default(),
        );
        assert_eq!(
            state.data_card().load_error,
            Some("CSV export: Access denied")
        );
    }

    #[test]
    fn analyzed_load_keeps_every_discovered_workbook_status() {
        let corpus = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../paramex-core/tests/reference/tlm/corpus");
        let analyzed = load_tlm_analyzed(&corpus, None).expect("corpus loads");

        assert_eq!(
            analyzed.workbook_count(),
            33,
            "loaded count includes 32 valid curves and one failed workbook"
        );
        assert!(analyzed.group_count() > 0);
    }

    #[test]
    fn analyzed_load_rejects_an_empty_directory() {
        let empty =
            std::env::temp_dir().join(format!("paramex_tlm_empty_test_dir_{}", std::process::id()));
        std::fs::create_dir_all(&empty).expect("empty test directory");
        let result = load_tlm_analyzed(&empty, None);
        std::fs::remove_dir(&empty).expect("remove empty test directory");

        assert!(result.is_err(), "no workbooks => error");
    }
}
