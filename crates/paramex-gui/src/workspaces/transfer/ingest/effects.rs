//! Transfer ingest-message effects.

use std::path::PathBuf;

use egui_notify::Toasts;
use paramex_core::transfer::{AttachOutputOutcome, OutputDataset, ParsedCurve};

use super::output_stats::OutputIngestStats;
use crate::format_ui::{already_loaded, exported_to, loaded_files};
use crate::io_tasks::saved_file_name;
use crate::workspaces::transfer::state::PendingOutputReason;
use crate::workspaces::transfer::TransferWorkspace;

pub(super) fn apply_files_parsed(
    outcomes: Vec<(String, Result<ParsedCurve, String>)>,
    workspace: &mut TransferWorkspace,
    toasts: &mut Toasts,
) {
    let mut loaded = 0usize;
    for (name, result) in outcomes {
        match result {
            Ok(curve) => match workspace.add_curve(curve) {
                Some(_) => {
                    loaded += 1;
                }
                None => {
                    toasts.warning(already_loaded(&name));
                }
            },
            Err(message) => {
                // Add-Files failures stay visible as a red error row and a toast.
                workspace.record_ingest_error(name.clone(), message.clone());
                toasts.error(message);
            }
        }
    }
    if loaded > 0 {
        toasts.success(loaded_files(loaded));
    }
}

pub(super) fn apply_folder_parsed(
    outcomes: Vec<(PathBuf, String, Result<ParsedCurve, String>)>,
    output_outcomes: Vec<(String, Result<OutputDataset, String>)>,
    workspace: &mut TransferWorkspace,
    toasts: &mut Toasts,
) {
    let (mut loaded, mut rejected, mut skipped) = (0usize, 0usize, 0usize);
    for (path, name, result) in outcomes {
        // A path already loaded is `skipped` regardless of parse outcome, so a
        // re-scan never turns a loaded file into a rejection.
        if workspace.session.source_path_loaded(&path) {
            skipped += 1;
            continue;
        }
        match result {
            Err(message) => {
                tracing::warn!("Folder import rejected {}: {message}", path.display());
                workspace.record_ingest_error(name, message);
                rejected += 1;
            }
            Ok(curve) => match workspace.add_curve(curve) {
                Some(_) => {
                    loaded += 1;
                }
                None => skipped += 1,
            },
        }
    }
    let output = apply_output_outcomes(output_outcomes, workspace);
    let pending = output.unmatched + output.ambiguous + output.displaced;
    let summary = format!(
        "Folder import: {loaded} transfer loaded, {attached} output attached, \
         {pending} output pending, {rejected} rejected, {skipped} already loaded, \
         {errors} output error(s).",
        attached = output.attached,
        errors = output.errors,
    );
    if loaded > 0 || output.attached > 0 {
        toasts.success(summary);
    } else {
        toasts.warning(summary);
    }
}

pub(super) fn apply_output_parsed(
    outcomes: Vec<(String, Result<OutputDataset, String>)>,
    workspace: &mut TransferWorkspace,
    toasts: &mut Toasts,
) {
    let stats = apply_output_outcomes(outcomes, workspace);
    let summary = stats.transfer_summary();
    if stats.attached > 0 {
        toasts.success(summary);
    } else {
        toasts.warning(summary);
    }
}

fn apply_output_outcomes(
    outcomes: Vec<(String, Result<OutputDataset, String>)>,
    workspace: &mut TransferWorkspace,
) -> OutputIngestStats {
    let mut stats = OutputIngestStats::default();
    for (name, result) in outcomes {
        match result {
            Ok(dataset) => {
                // Remove any earlier classification before transferring
                // ownership. Unattached outcomes return the dataset and are
                // recorded again below; attached outcomes leave no stale row.
                workspace.clear_pending_output_for(&dataset);
                match workspace.attach_output(dataset) {
                    AttachOutputOutcome::Attached { displaced, .. } => {
                        if let Some(displaced) = displaced {
                            workspace.retain_detached_output(displaced);
                            stats.displaced += 1;
                        }
                        stats.attached += 1;
                    }
                    AttachOutputOutcome::NoMatch { output } => {
                        workspace.record_pending_output(output, PendingOutputReason::NoMatch);
                        stats.unmatched += 1;
                        tracing::warn!("Transfer output: no loaded transfer file matches {name}");
                    }
                    AttachOutputOutcome::Ambiguous { output } => {
                        workspace.record_pending_output(output, PendingOutputReason::Ambiguous);
                        stats.ambiguous += 1;
                        tracing::warn!(
                            "Transfer output: output file {name} matches multiple files"
                        );
                    }
                }
            }
            Err(message) => {
                tracing::warn!("Transfer output parse failed for {name}: {message}");
                workspace.record_ingest_error(name, message.clone());
                stats.record_error(message);
            }
        }
    }
    stats
}

pub(super) fn apply_report_exported(
    result: Result<PathBuf, String>,
    workspace: &mut TransferWorkspace,
    toasts: &mut Toasts,
) {
    match result {
        Ok(path) => {
            toasts.success(exported_to(&saved_file_name(&path)));
        }
        Err(message) => {
            tracing::error!("Report export failed: {message}");
            workspace.record_ingest_error("Report export", message.clone());
            toasts.error(message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paramex_core::transfer::{OutputCurve, OutputDataset, OutputFitKind, Session};

    #[test]
    fn export_failure_persists_after_toast_sink_is_dropped() {
        let mut workspace = TransferWorkspace::from_session(Session::new());
        apply_report_exported(
            Err("Access denied".to_owned()),
            &mut workspace,
            &mut Toasts::default(),
        );
        assert!(matches!(
            workspace.file_rows.rows().next(),
            Some(crate::workspaces::transfer::state::FileRow::Error {
                name: "Report export",
                message: "Access denied",
                ..
            })
        ));
    }

    fn curve(name: &str, offset: f64) -> ParsedCurve {
        ParsedCurve {
            name: name.to_string(),
            vg: (0..12)
                .map(|index| -1.0 + 5.0 * index as f64 / 11.0)
                .collect(),
            id_abs: (0..12)
                .map(|index| 1e-12 * 10f64.powf(9.0 * index as f64 / 11.0) + offset)
                .collect(),
            source_path: None,
        }
    }

    fn output(name: &str) -> OutputDataset {
        OutputDataset {
            name: name.to_string(),
            curves: vec![OutputCurve {
                vg: 4.0,
                vd: vec![0.0, 1.0, 2.0],
                id: vec![1e-6, 2e-6, 3e-6],
            }],
            source_path: None,
        }
    }

    #[test]
    fn output_summary_surfaces_first_error_only_when_nothing_attached() {
        let mut stats = OutputIngestStats::default();
        stats.record_error("bad output header".to_string());
        let total_fail = stats.transfer_summary();
        assert!(
            total_fail.contains("First error: bad output header"),
            "{total_fail}"
        );

        let mut stats = OutputIngestStats::default();
        stats.attached = 1;
        stats.record_error("bad output header".to_string());
        let mixed = stats.transfer_summary();
        assert!(!mixed.contains("First error"), "{mixed}");
    }

    #[test]
    fn apply_output_parsed_attaches_unique_match_without_adding_transfer_files() {
        let mut session = Session::new();
        let file_id = session.add_curve(curve("device_a.csv", 0.0)).unwrap();
        let mut workspace = TransferWorkspace::from_session(session);
        let mut toasts = Toasts::default();

        apply_output_parsed(
            vec![(
                "device_a_output.csv".to_string(),
                Ok(output("device_a_output.csv")),
            )],
            &mut workspace,
            &mut toasts,
        );

        assert_eq!(workspace.session.file_count(), 1);
        assert_eq!(workspace.session.active_file_id(), Some(file_id.as_str()));
        assert!(workspace
            .session
            .selected_output_file()
            .expect("selected transfer")
            .output
            .is_some());
        assert_eq!(
            workspace
                .session
                .output_report_rows()
                .iter()
                .filter(|row| row.fit == OutputFitKind::Family)
                .count(),
            1
        );
        assert!(workspace.pending_outputs.is_empty());
    }

    #[test]
    fn automatic_replacement_keeps_the_displaced_output_as_pending() {
        let mut session = Session::new();
        session.add_curve(curve("device_a.csv", 0.0)).unwrap();
        let mut workspace = TransferWorkspace::from_session(session);

        let stats = apply_output_outcomes(
            vec![
                (
                    "device_a_output.csv".to_string(),
                    Ok(output("device_a_output.csv")),
                ),
                (
                    "device_a_id-vd.csv".to_string(),
                    Ok(output("device_a_id-vd.csv")),
                ),
            ],
            &mut workspace,
        );

        assert_eq!(stats.attached, 2);
        assert_eq!(stats.displaced, 1);
        assert!(stats.transfer_summary().contains("1 displaced"));
        assert_eq!(
            workspace
                .session
                .selected_output_file()
                .and_then(|selected| selected.output)
                .map(|dataset| dataset.name.as_str()),
            Some("device_a_id-vd.csv")
        );
        assert_eq!(workspace.pending_outputs.len(), 1);
        assert_eq!(workspace.pending_outputs[0].name(), "device_a_output.csv");
        assert_eq!(
            workspace.pending_outputs[0].reason(),
            PendingOutputReason::Detached
        );
    }

    #[test]
    fn automatic_replacement_preserves_a_newer_same_source_pending_payload() {
        let mut session = Session::new();
        let first_id = session.add_curve(curve("device_a.csv", 0.0)).unwrap();
        let mut workspace = TransferWorkspace::from_session(session);

        let mut old = output("device_a_output.csv");
        old.source_path = Some(PathBuf::from("lot-a/device_a_output.csv"));
        old.curves[0].vg = 1.0;
        let attached = apply_output_outcomes(
            vec![("device_a_output.csv".to_owned(), Ok(old.clone()))],
            &mut workspace,
        );
        assert_eq!(attached.attached, 1);

        let duplicate_id = workspace.add_curve(curve("device_a.txt", 1.0e-12)).unwrap();
        let mut newer = old.clone();
        newer.curves[0].vg = 9.0;
        let ambiguous = apply_output_outcomes(
            vec![("device_a_output.csv".to_owned(), Ok(newer))],
            &mut workspace,
        );
        assert_eq!(ambiguous.ambiguous, 1);
        assert_eq!(workspace.pending_outputs[0].dataset().curves[0].vg, 9.0);

        assert!(workspace.select_file(&duplicate_id));
        assert_eq!(workspace.remove_selected_or_checked(), 1);
        assert!(workspace.session.has_file(&first_id));

        let mut replacement = output("device_a_output.csv");
        replacement.source_path = Some(PathBuf::from("lot-b/device_a_output.csv"));
        replacement.curves[0].vg = 2.0;
        let replaced = apply_output_outcomes(
            vec![("device_a_output.csv".to_owned(), Ok(replacement))],
            &mut workspace,
        );

        assert_eq!(replaced.attached, 1);
        assert_eq!(replaced.displaced, 1);
        assert_eq!(
            workspace
                .session
                .selected_output_file()
                .and_then(|selected| selected.output)
                .expect("replacement attached")
                .curves[0]
                .vg,
            2.0
        );
        assert_eq!(workspace.pending_outputs.len(), 1);
        assert_eq!(
            workspace.pending_outputs[0].dataset().curves[0].vg,
            9.0,
            "the older displaced attachment must not replace newer pending data"
        );
    }

    #[test]
    fn automatic_same_source_alias_reload_clears_stale_pending_without_displacement() {
        let mut session = Session::new();
        session.add_curve(curve("device_a.csv", 0.0)).unwrap();
        let mut workspace = TransferWorkspace::from_session(session);
        let mut toasts = Toasts::default();
        let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut first = output("device_a_output.csv");
        first.source_path = Some(crate_root.join("Cargo.toml"));

        apply_output_parsed(
            vec![("device_a_output.csv".to_string(), Ok(first.clone()))],
            &mut workspace,
            &mut toasts,
        );
        workspace.record_pending_output(first, PendingOutputReason::NoMatch);

        let mut alias = output("device_a_id-vd.csv");
        alias.source_path = Some(crate_root.join("src").join("..").join("Cargo.toml"));
        apply_output_parsed(
            vec![("device_a_id-vd.csv".to_string(), Ok(alias))],
            &mut workspace,
            &mut toasts,
        );

        assert_eq!(
            workspace
                .session
                .selected_output_file()
                .and_then(|selected| selected.output)
                .map(|dataset| dataset.name.as_str()),
            Some("device_a_id-vd.csv")
        );
        assert!(workspace.pending_outputs.is_empty());
    }

    #[test]
    fn automatic_pathless_same_name_reload_preserves_range_without_pending_data() {
        let mut session = Session::new();
        let file_id = session.add_curve(curve("device_a.csv", 0.0)).unwrap();
        let mut workspace = TransferWorkspace::from_session(session);
        let mut toasts = Toasts::default();

        apply_output_parsed(
            vec![(
                "device_a_output.csv".to_string(),
                Ok(output("device_a_output.csv")),
            )],
            &mut workspace,
            &mut toasts,
        );
        assert!(workspace
            .session
            .set_output_fit_range(&file_id, Some((0.0, 1.0))));

        let mut reload = output("device_a_output.csv");
        reload.curves[0].id[1] = 9.0e-6;
        apply_output_parsed(
            vec![("device_a_output.csv".to_string(), Ok(reload))],
            &mut workspace,
            &mut toasts,
        );

        let selected = workspace
            .session
            .selected_output_file()
            .expect("transfer remains selected");
        assert_eq!(selected.selected_fit_range, Some((0.0, 1.0)));
        assert_eq!(
            selected.output.expect("reloaded output").curves[0].id[1],
            9.0e-6
        );
        assert!(workspace.pending_outputs.is_empty());
    }

    #[test]
    fn apply_folder_parsed_attaches_same_folder_output_after_transfer_load() {
        let path = PathBuf::from("2-6.xlsx");
        let mut parsed = curve("2-6.xlsx", 0.0);
        parsed.source_path = Some(path.clone());
        let mut workspace = TransferWorkspace::from_session(Session::new());
        let mut toasts = Toasts::default();

        apply_folder_parsed(
            vec![(path, "2-6.xlsx".to_string(), Ok(parsed))],
            vec![("2-6o.xlsx".to_string(), Ok(output("2-6o.xlsx")))],
            &mut workspace,
            &mut toasts,
        );

        let file_id = workspace.session.file_ids().next().unwrap().to_string();
        assert_eq!(workspace.session.file_count(), 1);
        assert_eq!(workspace.session.active_file_id(), Some(file_id.as_str()));
        assert_eq!(
            workspace
                .session
                .selected_output_file()
                .expect("selected transfer")
                .output
                .expect("attached output")
                .name,
            "2-6o.xlsx"
        );
        assert!(workspace.pending_outputs.is_empty());
    }

    #[test]
    fn apply_output_parsed_leaves_session_output_unchanged_for_unmatched_output() {
        let mut session = Session::new();
        let file_id = session.add_curve(curve("device_a.csv", 0.0)).unwrap();
        let generation = session.generation();
        let mut workspace = TransferWorkspace::from_session(session);
        let mut toasts = Toasts::default();

        apply_output_parsed(
            vec![(
                "device_b_output.csv".to_string(),
                Ok(output("device_b_output.csv")),
            )],
            &mut workspace,
            &mut toasts,
        );

        assert_eq!(workspace.session.file_count(), 1);
        assert_eq!(workspace.session.generation(), generation);
        assert_eq!(workspace.session.active_file_id(), Some(file_id.as_str()));
        assert!(workspace
            .session
            .selected_output_file()
            .expect("selected transfer")
            .output
            .is_none());
        assert!(workspace.session.output_report_rows().is_empty());
        assert_eq!(workspace.pending_outputs.len(), 1);
        assert_eq!(
            workspace.pending_outputs[0].reason(),
            PendingOutputReason::NoMatch
        );
    }

    #[test]
    fn repeated_unmatched_output_load_replaces_the_pending_row_instead_of_duplicating() {
        let mut workspace = TransferWorkspace::from_session(Session::new());
        let mut toasts = Toasts::default();
        for _ in 0..2 {
            apply_output_parsed(
                vec![(
                    "device_b_output.csv".to_string(),
                    Ok(output("device_b_output.csv")),
                )],
                &mut workspace,
                &mut toasts,
            );
        }
        assert_eq!(workspace.pending_outputs.len(), 1);
    }

    #[test]
    fn later_scan_attach_clears_the_stale_pending_row() {
        let mut workspace = TransferWorkspace::from_session(Session::new());
        let mut toasts = Toasts::default();
        // Scan 1: no transfer file loaded yet -> pending "No match".
        apply_output_parsed(
            vec![(
                "device_a_output.csv".to_string(),
                Ok(output("device_a_output.csv")),
            )],
            &mut workspace,
            &mut toasts,
        );
        assert_eq!(workspace.pending_outputs.len(), 1);

        // The transfer file arrives, then a re-scan attaches the output for
        // real — the stale pending row must not linger.
        workspace
            .session
            .add_curve(curve("device_a.csv", 0.0))
            .unwrap();
        apply_output_parsed(
            vec![(
                "device_a_output.csv".to_string(),
                Ok(output("device_a_output.csv")),
            )],
            &mut workspace,
            &mut toasts,
        );
        assert!(workspace.pending_outputs.is_empty());
    }

    #[test]
    fn apply_output_parsed_leaves_session_output_unchanged_for_errors_and_ambiguous_matches() {
        let mut session = Session::new();
        let first_id = session.add_curve(curve("device_a.csv", 0.0)).unwrap();
        let second_id = session.add_curve(curve("device_a.txt", 1e-12)).unwrap();
        let mut workspace = TransferWorkspace::from_session(session);
        let mut toasts = Toasts::default();

        apply_output_parsed(
            vec![
                (
                    "device_a_output.csv".to_string(),
                    Ok(output("device_a_output.csv")),
                ),
                (
                    "bad_output.csv".to_string(),
                    Err("bad output header".to_string()),
                ),
            ],
            &mut workspace,
            &mut toasts,
        );

        for file_id in [first_id, second_id] {
            assert!(workspace.select_file(&file_id));
            assert!(workspace
                .session
                .selected_output_file()
                .expect("selected transfer")
                .output
                .is_none());
        }
        assert!(workspace.session.output_report_rows().is_empty());
        assert_eq!(workspace.pending_outputs.len(), 1);
        assert_eq!(
            workspace.pending_outputs[0].reason(),
            PendingOutputReason::Ambiguous
        );
        assert!(workspace.file_rows.rows().any(|row| matches!(
            row,
            crate::workspaces::transfer::state::FileRow::Error {
                name: "bad_output.csv",
                message: "bad output header",
                ..
            }
        )));
    }

    #[test]
    fn folder_transfer_rejections_persist_as_error_rows() {
        let mut workspace = TransferWorkspace::from_session(Session::new());
        let mut toasts = Toasts::default();

        apply_folder_parsed(
            vec![(
                PathBuf::from("bad_transfer.csv"),
                "bad_transfer.csv".to_owned(),
                Err("missing gate-voltage column".to_owned()),
            )],
            Vec::new(),
            &mut workspace,
            &mut toasts,
        );

        assert!(matches!(
            workspace.file_rows.rows().next(),
            Some(crate::workspaces::transfer::state::FileRow::Error {
                name: "bad_transfer.csv",
                message: "missing gate-voltage column",
                ..
            })
        ));
    }
}
