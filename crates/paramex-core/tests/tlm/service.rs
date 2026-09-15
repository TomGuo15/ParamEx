use crate::common::tlm_fixture_dir;
use paramex_core::tlm::{
    analyze_dataset, analyze_sweep, load_dataset, FileStatus, Status, TlmCurve, TlmDataset,
    TlmSample, VdSource,
};
use std::path::PathBuf;

fn curve(file_path: String, group: &str, length_um: f64, vg: &[f64]) -> TlmCurve {
    let samples = vg
        .iter()
        .enumerate()
        .map(|(index, &vg)| {
            let current = (index + 1) as f64 * 1e-6;
            TlmSample::try_new(vg, current, current).unwrap()
        })
        .collect();
    TlmCurve::try_new(
        file_path,
        group.to_string(),
        length_um,
        samples,
        -0.5,
        VdSource::Setup,
    )
    .unwrap()
}

fn ok_status(file: String, group: &str, length_um: f64) -> FileStatus {
    FileStatus {
        file,
        group: group.to_string(),
        length_um: Some(length_um),
        status: Status::Ok,
        message: "Loaded".to_string(),
        vd_source: VdSource::Setup,
    }
}

#[test]
fn load_and_analyze_smoke() {
    // The single-workbook fixture: one group "grp", one length -> < 2 lengths -> NaN fit,
    // but the dataset/status/vg plumbing must still hold.
    let root = tlm_fixture_dir();
    let ds = load_dataset(&root, None).expect("loads");
    assert_eq!(ds.statuses().len(), 1);
    assert_eq!(ds.curves().len(), 1);
    let res = analyze_dataset(&ds, None);
    assert_eq!(
        res.groups
            .iter()
            .map(|group| group.group.as_str())
            .collect::<Vec<_>>(),
        vec!["grp"]
    );
    assert_eq!(res.first_group_name(), Some("grp"));
    assert!(res.has_group("grp"));
    assert!(res.group("grp").is_some());
    assert!(!res.has_group("missing"));
    assert!(res.group("missing").is_none());
    assert!(res.groups[0].fit.r_squared.is_nan()); // one length
    let swp = analyze_sweep(&ds);
    assert_eq!(swp.points.len(), ds.vg_values().len()); // 1 group × N vg
}

#[test]
fn remove_workbook_uses_exact_identity_and_rebuilds_gate_voltages() {
    let root = PathBuf::from("root");
    let removed_relative = PathBuf::from("proc").join("50").join("device.xlsx");
    let retained_relative = PathBuf::from("xproc").join("50").join("device.xlsx");
    let removed_file = removed_relative.display().to_string();
    let retained_file = retained_relative.display().to_string();
    let removed_full = root.join(&removed_relative).display().to_string();
    let retained_full = root.join(&retained_relative).display().to_string();
    assert!(
        retained_full.ends_with(&removed_file),
        "fixture must reproduce the old suffix collision"
    );

    let dataset = TlmDataset::try_new(
        root.display().to_string(),
        vec![
            curve(removed_full, "removed", 50.0, &[1.0]),
            curve(retained_full.clone(), "retained", 50.0, &[2.0]),
        ],
        vec![
            ok_status(removed_file.clone(), "removed", 50.0),
            ok_status(retained_file.clone(), "retained", 50.0),
        ],
    )
    .unwrap();

    let removal = dataset.remove_workbook(&removed_file);
    assert_eq!(removal.removed_statuses, 1);
    let dataset = removal.dataset.expect("one curve remains");
    assert_eq!(
        dataset
            .statuses()
            .iter()
            .map(|status| status.file.as_str())
            .collect::<Vec<_>>(),
        vec![retained_file.as_str()]
    );
    assert_eq!(dataset.curves().len(), 1);
    assert_eq!(dataset.curves()[0].file_path(), retained_full);
    assert_eq!(dataset.vg_values(), [2.0]);

    let unchanged = dataset.clone();
    let removal = dataset.remove_workbook("missing.xlsx");
    assert_eq!(removal.removed_statuses, 0);
    assert_eq!(removal.dataset, Some(unchanged));
}

#[test]
fn dataset_constructor_rejects_empty_or_error_only_aggregates() {
    let root = PathBuf::from("root");
    assert_eq!(
        TlmDataset::try_new(root.display().to_string(), Vec::new(), Vec::new()),
        Err(paramex_core::tlm::TlmParseError(
            "No valid TLM workbooks were found.".to_string()
        ))
    );

    let failed = FileStatus {
        file: "process/50/failed.xlsx".to_string(),
        group: "process".to_string(),
        length_um: Some(50.0),
        status: Status::Error,
        message: "failed".to_string(),
        vd_source: VdSource::Unread,
    };
    assert!(TlmDataset::try_new(root.display().to_string(), Vec::new(), vec![failed]).is_err());
}

#[test]
fn removing_the_final_curve_is_terminal_and_counts_residual_failures() {
    let root = PathBuf::from("root");
    let relative = PathBuf::from("process").join("50").join("valid.xlsx");
    let dataset = TlmDataset::try_new(
        root.display().to_string(),
        vec![curve(
            root.join(&relative).display().to_string(),
            "process",
            50.0,
            &[1.0],
        )],
        vec![
            ok_status(relative.display().to_string(), "process", 50.0),
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

    let removal = dataset.remove_workbook(&relative.display().to_string());
    assert_eq!(removal.removed_statuses, 2);
    assert_eq!(removal.dataset, None);
}

#[test]
fn dataset_constructor_derives_read_only_gate_voltages() {
    let root = PathBuf::from("root");
    let first_file = PathBuf::from("process").join("50").join("first.xlsx");
    let second_file = PathBuf::from("process").join("50").join("second.xlsx");

    let dataset = TlmDataset::try_new(
        root.display().to_string(),
        vec![
            curve(
                root.join(&first_file).display().to_string(),
                "process",
                50.0,
                &[2.0, 1.0],
            ),
            curve(
                root.join(&second_file).display().to_string(),
                "process",
                50.0,
                &[1.0, 3.0],
            ),
        ],
        vec![
            ok_status(first_file.display().to_string(), "process", 50.0),
            ok_status(second_file.display().to_string(), "process", 50.0),
        ],
    )
    .unwrap();

    assert_eq!(dataset.vg_values(), [1.0, 2.0, 3.0]);
}

#[test]
fn dataset_voltage_rounding_preserves_large_finite_measurements() {
    let root = PathBuf::from("root");
    let relative = PathBuf::from("process").join("50").join("large.xlsx");
    let dataset = TlmDataset::try_new(
        root.display().to_string(),
        vec![curve(
            root.join(&relative).display().to_string(),
            "process",
            50.0,
            &[f64::MAX],
        )],
        vec![ok_status(relative.display().to_string(), "process", 50.0)],
    )
    .unwrap();

    assert_eq!(dataset.vg_values(), [f64::MAX]);
    assert!(dataset.vg_values()[0].is_finite());
}

#[test]
fn dataset_constructor_rejects_outside_root_or_status_curve_disagreement() {
    let root = PathBuf::from("root");
    let relative = PathBuf::from("process").join("50").join("device.xlsx");
    let status = ok_status(relative.display().to_string(), "process", 50.0);

    assert!(TlmDataset::try_new(
        root.display().to_string(),
        vec![curve(
            PathBuf::from("outside")
                .join(&relative)
                .display()
                .to_string(),
            "process",
            50.0,
            &[1.0],
        )],
        vec![status.clone()],
    )
    .is_err());

    let valid_curve = curve(
        root.join(&relative).display().to_string(),
        "process",
        50.0,
        &[1.0],
    );
    assert!(TlmDataset::try_new(
        root.display().to_string(),
        vec![valid_curve.clone()],
        vec![ok_status(
            PathBuf::from("other.xlsx").display().to_string(),
            "process",
            50.0,
        )],
    )
    .is_err());

    let mut wrong_metadata = status;
    wrong_metadata.length_um = Some(80.0);
    assert!(TlmDataset::try_new(
        root.display().to_string(),
        vec![valid_curve.clone()],
        vec![wrong_metadata],
    )
    .is_err());

    let failed_status = FileStatus {
        file: relative.display().to_string(),
        group: "process".to_string(),
        length_um: Some(50.0),
        status: Status::Error,
        message: "failed".to_string(),
        vd_source: VdSource::Unread,
    };
    assert!(TlmDataset::try_new(
        root.display().to_string(),
        vec![valid_curve],
        vec![failed_status],
    )
    .is_err());
}
