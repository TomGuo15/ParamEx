//! Transfer-curve file/folder loading and report export workers.

use std::path::{Path, PathBuf};

use eframe::egui;
use paramex_core::transfer::{
    is_supported_measurement_path, output_name_hint, parse_output_file, parse_transfer_file,
    OutputDataset, ParsedCurve, SUPPORTED_EXTENSIONS,
};

use crate::io_tasks::{IoEvent, IoQueue};
use crate::workspaces::transfer::TransferWorkspace;

mod effects;
mod output_stats;

/// One terminal result from a Transfer worker.
pub(crate) enum Msg {
    FilesParsed {
        outcomes: Vec<(String, Result<ParsedCurve, String>)>,
    },
    FolderParsed {
        outcomes: Vec<(PathBuf, String, Result<ParsedCurve, String>)>,
        output_outcomes: Vec<(String, Result<OutputDataset, String>)>,
    },
    OutputParsed {
        outcomes: Vec<(String, Result<OutputDataset, String>)>,
    },
    ReportExported {
        result: Result<PathBuf, String>,
    },
}

pub(crate) fn drain(workspace: &mut TransferWorkspace, toasts: &mut egui_notify::Toasts) {
    for event in workspace.io.drain_events() {
        match event {
            IoEvent::Message(Msg::FilesParsed { outcomes }) => {
                effects::apply_files_parsed(outcomes, workspace, toasts)
            }
            IoEvent::Message(Msg::FolderParsed {
                outcomes,
                output_outcomes,
            }) => effects::apply_folder_parsed(outcomes, output_outcomes, workspace, toasts),
            IoEvent::Message(Msg::OutputParsed { outcomes }) => {
                effects::apply_output_parsed(outcomes, workspace, toasts)
            }
            IoEvent::Message(Msg::ReportExported { result }) => {
                effects::apply_report_exported(result, workspace, toasts)
            }
            IoEvent::WorkerFailed(failure) => {
                workspace.record_ingest_error(
                    failure.task_name().to_owned(),
                    failure.message().to_owned(),
                );
                toasts.error(failure.notice());
            }
        }
    }
}

/// Supported transfer-curve files reachable from `path` (file or dir),
/// recursively, sorted by lowercased path components so load order is stable
/// across file systems and independent of filename case.
fn supported_files_in_dir(path: &Path) -> Vec<PathBuf> {
    if path.is_file() {
        return if is_supported_measurement_path(path) {
            vec![path.to_path_buf()]
        } else {
            Vec::new()
        };
    }
    if !path.is_dir() {
        return Vec::new();
    }
    let mut out = Vec::new();
    collect(path, &mut out);
    out.sort_by_cached_key(|a| {
        a.components()
            .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
            .collect::<Vec<_>>()
    });
    out
}

/// Recursive directory walk (std only; no `walkdir`, size gate). Unreadable
/// sub-directories are skipped so one bad folder does not abort the scan.
fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            collect(&p, out);
        } else if p.is_file() && is_supported_measurement_path(&p) {
            out.push(p);
        }
    }
}

/// Map a `core::ParseError`/`io` failure to the message string the UI shows.
fn parse_one(path: &Path) -> Result<ParsedCurve, String> {
    parse_transfer_file(path).map_err(|e| e.0)
}

fn parse_output_one(path: &Path) -> Result<OutputDataset, String> {
    parse_output_file(path).map_err(|e| e.0)
}

/// "Add Files": open the multi picker (blocking) on a worker thread, parse all
/// picks, and send one batched `FilesParsed`.
pub(crate) fn start_add_files(ctx: &egui::Context, queue: &mut IoQueue<Msg>) {
    crate::io_tasks::spawn_io(ctx, queue, "Transfer file load", || {
        let exts: Vec<&str> = SUPPORTED_EXTENSIONS
            .iter()
            .map(|e| e.trim_start_matches('.'))
            .collect();
        let picked = rfd::FileDialog::new()
            .add_filter("data", &exts)
            .set_title("Add ParamEx data files")
            .pick_files();
        picked.map(|paths| {
            let outcomes = paths
                .into_iter()
                .map(|path| {
                    let name = crate::io_tasks::file_name_lossy(&path);
                    let result = parse_one(&path);
                    (name, result)
                })
                .collect();
            Msg::FilesParsed { outcomes }
        })
    });
}

/// "Add Folder": open the folder picker (blocking), enumerate supported files,
/// parse transfer files and same-folder output curves, then send one batch.
pub(crate) fn start_add_folder(ctx: &egui::Context, queue: &mut IoQueue<Msg>) {
    crate::io_tasks::spawn_io(ctx, queue, "Transfer folder load", || {
        let folder = rfd::FileDialog::new()
            .set_title("Add a folder of ParamEx data")
            .pick_folder();
        folder.map(|dir| {
            let mut outcomes = Vec::new();
            let mut output_outcomes = Vec::new();
            for path in supported_files_in_dir(&dir) {
                let name = crate::io_tasks::file_name_lossy(&path);
                if output_name_hint(&name) {
                    // Symmetric to the un-hinted fallback below: a name that
                    // merely looks like an output convention can still be a
                    // transfer sweep, so a failed output parse retries as one.
                    match parse_output_one(&path) {
                        Ok(dataset) => output_outcomes.push((name, Ok(dataset))),
                        Err(output_err) => match parse_one(&path) {
                            Ok(curve) => outcomes.push((path, name, Ok(curve))),
                            Err(_) => output_outcomes.push((name, Err(output_err))),
                        },
                    }
                    continue;
                }
                match parse_one(&path) {
                    Ok(curve) => outcomes.push((path, name, Ok(curve))),
                    Err(transfer_err) => match parse_output_one(&path) {
                        Ok(dataset) => output_outcomes.push((name, Ok(dataset))),
                        Err(_) => outcomes.push((path, name, Err(transfer_err))),
                    },
                }
            }
            Msg::FolderParsed {
                outcomes,
                output_outcomes,
            }
        })
    });
}

/// "Load Output": open the multi picker, parse output Id-Vd curves, and send one
/// batched `OutputParsed`.
pub(crate) fn start_add_output_files(ctx: &egui::Context, queue: &mut IoQueue<Msg>) {
    crate::io_tasks::spawn_io(ctx, queue, "Transfer output load", || {
        let exts: Vec<&str> = SUPPORTED_EXTENSIONS
            .iter()
            .map(|e| e.trim_start_matches('.'))
            .collect();
        let picked = rfd::FileDialog::new()
            .add_filter("data", &exts)
            .set_title("Load transfer output curves (Id-Vd)")
            .pick_files();
        picked.map(|paths| {
            let outcomes = paths
                .into_iter()
                .map(|path| {
                    let name = crate::io_tasks::file_name_lossy(&path);
                    let result = parse_output_one(&path);
                    (name, result)
                })
                .collect();
            Msg::OutputParsed { outcomes }
        })
    });
}

/// Save report bytes (already gathered on the UI thread via `Session::report_bytes`)
/// to an rfd-chosen `.csv` path. Posts `ReportExported`; a cancelled picker posts
/// nothing.
pub(crate) fn start_export_report(ctx: &egui::Context, queue: &mut IoQueue<Msg>, bytes: Vec<u8>) {
    crate::io_tasks::start_save_csv(
        ctx,
        queue,
        bytes,
        "paramex_report.csv",
        "Export report CSV",
        |result| Msg::ReportExported { result },
    );
}

pub(crate) fn start_export_output_report(
    ctx: &egui::Context,
    queue: &mut IoQueue<Msg>,
    bytes: Vec<u8>,
) {
    crate::io_tasks::start_save_csv(
        ctx,
        queue,
        bytes,
        "paramex_output_report.csv",
        "Export output report CSV",
        |result| Msg::ReportExported { result },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("paramex_{name}_{}", std::process::id()))
    }

    #[test]
    fn folder_scan_is_recursive_filtered_and_sorted() {
        let root = temp_dir("scan");
        let _ = std::fs::remove_dir_all(&root);
        let sub = root.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        for name in ["B.CSV", "a.tsv", "ignore.png", "note.txt"] {
            std::fs::write(root.join(name), b"x").unwrap();
        }
        std::fs::write(sub.join("deep.xlsx"), b"x").unwrap();
        std::fs::write(sub.join("skip.dat"), b"x").unwrap();

        let got: Vec<String> = supported_files_in_dir(&root)
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(got, ["a.tsv", "B.CSV", "note.txt", "deep.xlsx"]);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn folder_scan_handles_single_files_and_component_order() {
        let root = temp_dir("scan_edges");
        let _ = std::fs::remove_dir_all(&root);
        let sub = root.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        let csv = root.join("one.csv");
        let png = root.join("one.png");
        std::fs::write(&csv, b"x").unwrap();
        std::fs::write(&png, b"x").unwrap();
        assert_eq!(supported_files_in_dir(&csv), [csv]);
        assert!(supported_files_in_dir(&png).is_empty());

        let sibling = root.join("sub.csv");
        let nested = sub.join("a.csv");
        std::fs::write(&sibling, b"x").unwrap();
        std::fs::write(&nested, b"x").unwrap();
        let got = supported_files_in_dir(&root);
        assert_eq!(got, [root.join("one.csv"), nested, sibling]);
        std::fs::remove_dir_all(root).unwrap();
    }
}
