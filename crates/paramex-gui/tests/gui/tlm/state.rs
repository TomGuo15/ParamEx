use crate::common;
use paramex_core::tlm::AnalyzedDataset;
use paramex_core::tlm::{FileStatus, Status, TlmCurve, TlmDataset, TlmSample, VdSource};
use paramex_gui::workspaces::tlm::state::TlmState;
use std::path::PathBuf;

#[test]
fn install_analyzed_sets_defaults() {
    let tlm = common::loaded_tlm_state();

    let data = tlm.data_card();
    let groups = tlm.group_list().expect("groups computed");
    let picker = tlm.vg_picker().expect("V_G picker computed");
    let results = tlm.results_card();
    assert!(data.has_dataset);
    assert!(results.has_result, "result computed");
    assert!(results.has_sweep, "sweep computed");
    // default V_G is the selected V_G exposed to the picker.
    assert_eq!(tlm.selected_vg(), Some(picker.selected_vg));
    // first group is selected.
    assert_eq!(
        tlm.selected_group_name(),
        groups.groups.first().map(|group| group.group.as_str())
    );
    assert!(tlm.selected_group_analysis().is_some());
}

#[test]
fn data_workbook_count_includes_failed_status_rows() {
    let tlm = common::loaded_tlm_state();
    let workbooks = tlm
        .data_card()
        .folder
        .expect("loaded TLM folder summary")
        .workbooks;

    assert_eq!(workbooks, tlm.rows().status().len());
    assert_eq!(
        workbooks, 33,
        "the corpus contains 32 OK + 1 failed workbook"
    );
}

#[test]
fn remove_file_drops_a_workbook_and_reanalyzes() {
    let mut tlm = common::loaded_tlm_state();
    let before = tlm.rows().status().len();
    assert!(before >= 2, "corpus has multiple files");
    // Row 0 of a status row is the file path the reducer matches on.
    let file = tlm.rows().status()[0][0].clone();

    assert_eq!(tlm.remove_file(&file), 1, "removes one known file");
    assert_eq!(
        tlm.rows().status().len(),
        before - 1,
        "exactly one file row is gone"
    );
    assert!(
        tlm.results_card().has_result,
        "the remainder is re-analyzed, not left stale"
    );
    // An unknown file is a no-op.
    assert_eq!(tlm.remove_file("no/such/file.xlsx"), 0);
}

#[test]
fn remove_file_keeps_suffix_neighbor_and_refreshes_gate_voltages() {
    let root = PathBuf::from("root");
    let removed_relative = PathBuf::from("proc").join("50").join("device.xlsx");
    let retained_relative = PathBuf::from("xproc").join("50").join("device.xlsx");
    let removed_file = removed_relative.display().to_string();
    let retained_file = retained_relative.display().to_string();
    let removed_full = root.join(&removed_relative).display().to_string();
    let retained_full = root.join(&retained_relative).display().to_string();
    assert!(retained_full.ends_with(&removed_file));

    let curve = |file_path: String, group: &str, vg: f64| {
        TlmCurve::try_new(
            file_path,
            group.to_string(),
            50.0,
            vec![TlmSample::try_new(vg, 1e-6, 1e-6).unwrap()],
            -0.5,
            VdSource::Setup,
        )
        .unwrap()
    };
    let status = |file: String, group: &str| FileStatus {
        file,
        group: group.to_string(),
        length_um: Some(50.0),
        status: Status::Ok,
        message: "Loaded".to_string(),
        vd_source: VdSource::Setup,
    };
    let dataset = TlmDataset::try_new(
        root.display().to_string(),
        vec![
            curve(removed_full, "removed", 1.0),
            curve(retained_full, "retained", 2.0),
        ],
        vec![
            status(removed_file.clone(), "removed"),
            status(retained_file.clone(), "retained"),
        ],
    )
    .unwrap();
    let mut tlm = TlmState::default();
    tlm.install_analyzed(AnalyzedDataset::analyze(dataset));

    assert_eq!(tlm.remove_file(&removed_file), 1);
    assert!(tlm.has_dataset(), "the suffix neighbor must remain loaded");
    assert_eq!(
        tlm.rows().status()[0][0],
        retained_file,
        "only the exact workbook row should remain"
    );
    assert_eq!(
        tlm.vg_picker().expect("remaining analysis").vg_values,
        &[2.0],
        "the picker must expose only voltages from remaining curves"
    );
}

#[test]
fn remove_file_clears_the_terminal_dataset_and_residual_failures() {
    let root = PathBuf::from("root");
    let valid_file = PathBuf::from("process").join("50").join("valid.xlsx");
    let dataset = TlmDataset::try_new(
        root.display().to_string(),
        vec![TlmCurve::try_new(
            root.join(&valid_file).display().to_string(),
            "process".to_string(),
            50.0,
            vec![TlmSample::try_new(1.0, 1e-6, 1e-6).unwrap()],
            -0.5,
            VdSource::Setup,
        )
        .unwrap()],
        vec![
            FileStatus {
                file: valid_file.display().to_string(),
                group: "process".to_string(),
                length_um: Some(50.0),
                status: Status::Ok,
                message: "Loaded".to_string(),
                vd_source: VdSource::Setup,
            },
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
    let mut tlm = TlmState::default();
    tlm.install_analyzed(AnalyzedDataset::analyze(dataset));

    assert_eq!(tlm.remove_file(&valid_file.display().to_string()), 2);
    assert!(!tlm.has_dataset());
    assert!(!tlm.results_card().has_result);
    assert!(tlm.vg_picker().is_none());
    assert!(tlm.rows().status().is_empty());
}

#[test]
fn recompute_at_vg_snaps_to_measured() {
    let mut tlm = common::loaded_tlm_state();
    let measured = tlm
        .vg_picker()
        .expect("loaded V_G picker")
        .vg_values
        .to_vec();
    assert!(measured.len() >= 2);

    // Ask for a value between two measured points; the engine snaps to the nearest.
    let target = (measured[0] + measured[1]) / 2.0 + (measured[1] - measured[0]) * 0.01;
    tlm.recompute_at_vg(target);
    let snapped = tlm.selected_vg().unwrap();
    assert!(
        measured.iter().any(|&v| (v - snapped).abs() < 1e-12),
        "snapped to a measured V_G"
    );
}

#[test]
fn select_group_accepts_only_analyzed_groups() {
    let mut tlm = common::loaded_tlm_state();
    let groups = tlm.group_list().expect("loaded TLM groups");
    let target = groups
        .groups
        .iter()
        .map(|group| group.group.clone())
        .find(|name| Some(name.as_str()) != groups.selected)
        .expect("at least two process groups in corpus");

    assert!(tlm.select_group(&target));
    assert_eq!(tlm.selected_group_name(), Some(target.as_str()));

    assert!(!tlm.select_group("missing-process-group"));
    assert_eq!(
        tlm.selected_group_name(),
        Some(target.as_str()),
        "invalid selections should not overwrite the current group"
    );
}

#[test]
fn clear_resets_analysis_but_keeps_fallback() {
    let ds = common::load_tlm_corpus();
    // Mutate-after-default (not struct-update): TlmState carries a private
    // rows_generation field, so `..TlmState::default()` no longer compiles here.
    let mut tlm = TlmState::default();
    tlm.set_fallback_vd(-1.5).expect("valid fallback");
    tlm.install_analyzed(AnalyzedDataset::analyze(ds));
    // Confirm something was loaded.
    assert!(tlm.has_dataset());
    tlm.clear();
    assert!(!tlm.has_dataset());
    assert!(!tlm.results_card().has_result);
    assert!(!tlm.results_card().has_sweep);
    assert!(tlm.vg_picker().is_none());
    assert!(tlm.files_card().is_none());
    assert!(tlm.rows().results().is_empty());
    assert!(tlm.rows().sweep().is_empty());
    assert!(tlm.selected_group_name().is_none());
    assert!(tlm.selected_vg().is_none());
    assert!(!tlm.has_load_error());
    assert_eq!(tlm.fallback_vd(), -1.5);
}

#[test]
fn install_analyzed_self_heals_prior_load_error() {
    let ds = common::load_tlm_corpus();
    // Seed a prior failure before the successful load arrives.
    let mut tlm = TlmState::default();
    tlm.push_load_error("boom".into());
    tlm.install_analyzed(AnalyzedDataset::analyze(ds));
    // A successful install must clear the error row.
    assert!(
        !tlm.has_load_error(),
        "install_analyzed should self-heal a prior load_error"
    );
}

#[test]
fn later_error_queues_behind_the_oldest_load_error() {
    let mut tlm = TlmState::default();
    tlm.push_load_error("load failed".into());
    tlm.push_load_error("CSV export: Access denied".into());
    assert_eq!(tlm.data_card().load_error, Some("load failed"));
    assert_eq!(tlm.data_card().load_error_count, 2);
    tlm.dismiss_load_error();
    assert_eq!(
        tlm.data_card().load_error,
        Some("CSV export: Access denied")
    );
    assert_eq!(tlm.data_card().load_error_count, 1);
}

#[test]
fn default_fallback_is_minus_half() {
    assert_eq!(TlmState::default().fallback_vd(), -0.5);
}

#[test]
fn fallback_rejects_invalid_values_without_mutating_state() {
    let mut tlm = TlmState::default();
    let initial = tlm.fallback_vd();

    for invalid in [0.0, f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
        assert!(tlm.set_fallback_vd(invalid).is_err());
        assert_eq!(tlm.fallback_vd(), initial);
    }
}
