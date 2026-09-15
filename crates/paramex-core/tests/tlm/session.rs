//! `tlm::Session`: install, selection, recompute, workbook removal, and the
//! display-cache generation contract.

use crate::common::tlm_corpus_dir;
use paramex_core::tlm::{
    load_dataset, AnalyzedDataset, FileStatus, Session, Status, TlmCurve, TlmDataset, TlmSample,
    VdSource, DEFAULT_FALLBACK_VD,
};
use std::path::PathBuf;

fn loaded_session() -> Session {
    let dataset = load_dataset(&tlm_corpus_dir(), None).expect("TLM corpus loads");
    let mut session = Session::new();
    session.install(AnalyzedDataset::analyze(dataset));
    session
}

fn curve(file_path: String, group: &str, vg: f64) -> TlmCurve {
    TlmCurve::try_new(
        file_path,
        group.to_string(),
        50.0,
        vec![TlmSample::try_new(vg, 1e-6, 1e-6).unwrap()],
        -0.5,
        VdSource::Setup,
    )
    .unwrap()
}

fn ok_status(file: String, group: &str) -> FileStatus {
    FileStatus {
        file,
        group: group.to_string(),
        length_um: Some(50.0),
        status: Status::Ok,
        message: "Loaded".to_string(),
        vd_source: VdSource::Setup,
    }
}

#[test]
fn new_session_is_empty_with_the_default_fallback() {
    let session = Session::new();
    assert!(!session.has_dataset());
    assert!(session.dataset().is_none());
    assert!(session.result().is_none());
    assert!(session.sweep().is_none());
    assert!(session.selected_group_name().is_none());
    assert!(session.selected_vg().is_none());
    assert!(session.selected_group_analysis().is_none());
    assert!(session.result_csv_bytes().is_none());
    assert!(session.sweep_csv_bytes().is_none());
    assert_eq!(session.fallback_vd(), DEFAULT_FALLBACK_VD);
    assert_eq!(session.generation(), 0);
}

#[test]
fn install_selects_the_default_vg_and_first_group() {
    let session = loaded_session();
    let result = session.result().expect("result computed");
    assert!(session.sweep().is_some(), "sweep computed");
    assert_eq!(session.selected_vg(), Some(result.selected_vg));
    assert_eq!(session.selected_group_name(), result.first_group_name());
    assert!(session.selected_group_analysis().is_some());
    assert_eq!(session.generation(), 1);
    assert!(session.result_csv_bytes().is_some_and(|b| !b.is_empty()));
    assert!(session.sweep_csv_bytes().is_some_and(|b| !b.is_empty()));
}

#[test]
fn analyzed_dataset_counts_every_discovered_workbook() {
    let dataset = load_dataset(&tlm_corpus_dir(), None).expect("TLM corpus loads");
    let analyzed = AnalyzedDataset::analyze(dataset);
    assert_eq!(
        analyzed.workbook_count(),
        33,
        "32 parsed curves plus one failed workbook"
    );
    assert!(analyzed.group_count() > 1);
}

#[test]
fn recompute_at_vg_snaps_to_a_measured_voltage_and_bumps_generation() {
    let mut session = loaded_session();
    let measured = session.result().expect("result").vg_values.clone();
    assert!(measured.len() >= 2);
    let before = session.generation();

    let target = (measured[0] + measured[1]) / 2.0 + (measured[1] - measured[0]) * 0.01;
    session.recompute_at_vg(target);

    let snapped = session.selected_vg().expect("selected V_G");
    assert!(measured.iter().any(|&v| (v - snapped).abs() < 1e-12));
    assert_eq!(session.result().expect("result").selected_vg, snapped);
    assert_eq!(session.generation(), before + 1);
}

#[test]
fn recompute_keeps_the_selected_group_when_it_survives() {
    let mut session = loaded_session();
    let names: Vec<String> = session
        .result()
        .expect("result")
        .groups
        .iter()
        .map(|g| g.group.clone())
        .collect();
    let target = names.last().expect("corpus has groups").clone();
    assert!(session.select_group(&target));

    let vg = session.result().expect("result").vg_values[0];
    session.recompute_at_vg(vg);
    assert_eq!(session.selected_group_name(), Some(target.as_str()));
}

#[test]
fn select_group_accepts_only_analyzed_groups_without_bumping_generation() {
    let mut session = loaded_session();
    let generation = session.generation();
    let names: Vec<String> = session
        .result()
        .expect("result")
        .groups
        .iter()
        .map(|g| g.group.clone())
        .collect();
    let target = names
        .iter()
        .find(|name| Some(name.as_str()) != session.selected_group_name())
        .expect("at least two groups")
        .clone();

    assert!(session.select_group(&target));
    assert_eq!(session.selected_group_name(), Some(target.as_str()));
    assert!(!session.select_group("missing-process-group"));
    assert_eq!(session.selected_group_name(), Some(target.as_str()));
    assert_eq!(session.generation(), generation);

    assert!(!Session::new().select_group(&target));
}

#[test]
fn set_fallback_vd_rejects_invalid_values_without_mutating() {
    let mut session = Session::new();
    for invalid in [0.0, f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
        assert!(session.set_fallback_vd(invalid).is_err());
        assert_eq!(session.fallback_vd(), DEFAULT_FALLBACK_VD);
    }
    assert!(session.set_fallback_vd(-1.5).is_ok());
    assert_eq!(session.fallback_vd(), -1.5);
    assert_eq!(session.generation(), 0, "settings do not invalidate rows");
}

#[test]
fn clear_drops_analyses_but_keeps_the_fallback() {
    let mut session = loaded_session();
    session.set_fallback_vd(-1.5).expect("valid fallback");
    let before = session.generation();

    session.clear();
    assert!(!session.has_dataset());
    assert!(session.result().is_none());
    assert!(session.sweep().is_none());
    assert!(session.selected_group_name().is_none());
    assert!(session.selected_vg().is_none());
    assert_eq!(session.fallback_vd(), -1.5);
    assert_eq!(session.generation(), before + 1);

    session.clear();
    assert_eq!(
        session.generation(),
        before + 1,
        "clearing an empty session changes nothing"
    );
}

#[test]
fn remove_workbook_reanalyzes_the_remainder() {
    let mut session = loaded_session();
    let statuses = session.result().expect("result").statuses.clone();
    assert!(statuses.len() >= 2);
    let file = statuses[0].file.clone();
    let before = session.generation();

    assert_eq!(session.remove_workbook(&file), 1);
    let result = session.result().expect("remainder re-analyzed");
    assert_eq!(result.statuses.len(), statuses.len() - 1);
    assert!(result.statuses.iter().all(|status| status.file != file));
    assert_eq!(session.generation(), before + 1);

    assert_eq!(session.remove_workbook("no/such/file.xlsx"), 0);
    assert_eq!(session.generation(), before + 1, "a miss changes nothing");
    assert_eq!(Session::new().remove_workbook(&file), 0);
}

#[test]
fn remove_workbook_matches_the_exact_relative_path_and_refreshes_gate_voltages() {
    let root = PathBuf::from("root");
    let removed_relative = PathBuf::from("proc").join("50").join("device.xlsx");
    let retained_relative = PathBuf::from("xproc").join("50").join("device.xlsx");
    let removed_file = removed_relative.display().to_string();
    let retained_file = retained_relative.display().to_string();
    let dataset = TlmDataset::try_new(
        root.display().to_string(),
        vec![
            curve(
                root.join(&removed_relative).display().to_string(),
                "removed",
                1.0,
            ),
            curve(
                root.join(&retained_relative).display().to_string(),
                "retained",
                2.0,
            ),
        ],
        vec![
            ok_status(removed_file.clone(), "removed"),
            ok_status(retained_file.clone(), "retained"),
        ],
    )
    .unwrap();
    let mut session = Session::new();
    session.install(AnalyzedDataset::analyze(dataset));
    assert_eq!(session.selected_group_name(), Some("removed"));

    assert_eq!(session.remove_workbook(&removed_file), 1);
    assert!(
        session.has_dataset(),
        "the suffix neighbor must remain loaded"
    );
    let result = session.result().expect("result");
    assert_eq!(result.statuses.len(), 1);
    assert_eq!(result.statuses[0].file, retained_file);
    assert_eq!(result.vg_values, vec![2.0]);
    assert_eq!(
        session.selected_vg(),
        Some(2.0),
        "V_G snaps to the survivor"
    );
    assert_eq!(
        session.selected_group_name(),
        Some("retained"),
        "a removed group falls back to the first survivor"
    );
}

#[test]
fn removing_the_last_curve_clears_the_session_and_counts_residual_failures() {
    let root = PathBuf::from("root");
    let valid_file = PathBuf::from("process").join("50").join("valid.xlsx");
    let dataset = TlmDataset::try_new(
        root.display().to_string(),
        vec![curve(
            root.join(&valid_file).display().to_string(),
            "process",
            1.0,
        )],
        vec![
            ok_status(valid_file.display().to_string(), "process"),
            FileStatus {
                file: "process/50/failed.xlsx".to_string(),
                group: "process".to_string(),
                length_um: Some(50.0),
                status: Status::Error,
                message: "failed".to_string(),
                vd_source: VdSource::Unread,
            },
        ],
    )
    .unwrap();
    let mut session = Session::new();
    session.set_fallback_vd(-2.0).expect("valid fallback");
    session.install(AnalyzedDataset::analyze(dataset));
    let before = session.generation();

    assert_eq!(
        session.remove_workbook(&valid_file.display().to_string()),
        2
    );
    assert!(!session.has_dataset());
    assert!(session.result().is_none());
    assert!(session.sweep().is_none());
    assert!(session.selected_vg().is_none());
    assert_eq!(session.fallback_vd(), -2.0);
    assert_eq!(session.generation(), before + 1);
}
