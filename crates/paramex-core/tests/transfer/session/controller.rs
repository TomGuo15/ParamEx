use crate::{expect_attached, transfer_curve as curve};
use paramex_core::transfer::{ExpertWindow, OutputCurve, OutputDataset, Session};
use std::path::PathBuf;

fn output(name: &str, scale: f64) -> OutputDataset {
    OutputDataset {
        name: name.to_string(),
        curves: vec![OutputCurve {
            vg: 5.0,
            vd: vec![0.0, 1.0, 2.0, 3.0],
            id: vec![0.0, scale * 1.0e-6, scale * 1.7e-6, scale * 2.5e-6],
        }],
        source_path: None,
    }
}

#[test]
fn output_commands_transfer_ownership_atomically() {
    let mut session = Session::new();
    let id = session.add_curve(curve("device_a.csv", 1.0)).unwrap();

    assert!(session
        .replace_output(&id, output("first.csv", 1.0))
        .unwrap()
        .is_none());
    let displaced = session
        .replace_output(&id, output("second.csv", 2.0))
        .unwrap()
        .expect("first output displaced");
    assert_eq!(displaced.name, "first.csv");

    let generation = session.generation();
    let rejected = session
        .replace_output("missing", output("rejected.csv", 3.0))
        .expect_err("missing file returns the unconsumed dataset");
    assert_eq!(rejected.name, "rejected.csv");
    assert_eq!(session.generation(), generation);

    let taken = session.take_output(&id).expect("second output attached");
    assert_eq!(taken.name, "second.csv");
    let generation = session.generation();
    assert!(session.take_output(&id).is_none());
    assert_eq!(session.generation(), generation);
}

#[test]
fn add_curve_appends_selects_and_extracts() {
    let mut s = Session::new();
    let id = s.add_curve(curve("a.csv", 1.0)).expect("added");
    assert_eq!(s.active_file_id(), Some(id.as_str()));
    assert_eq!(
        s.selected_file_metrics_projection()
            .expect("selected metrics")
            .result
            .filename,
        "a.csv"
    );
    assert!(s.has_selected_file());
    assert!(s.has_file(&id));
    assert!(!s.has_file("missing"));
}

#[test]
fn add_curve_dedups_identical() {
    let mut s = Session::new();
    s.add_curve(curve("a.csv", 1.0)).unwrap();
    assert!(
        s.add_curve(curve("a.csv", 1.0)).is_none(),
        "identical curve is a duplicate"
    );
    assert_eq!(s.file_count(), 1);
}

#[test]
fn add_curve_rejects_parser_invalid_data_atomically() {
    let mut s = Session::new();
    let loaded_id = s.add_curve(curve("loaded.csv", 1.0)).unwrap();
    let initial_generation = s.generation();

    let mut invalid = curve("invalid.csv", 1.5);
    invalid.id_abs[0] = f64::NAN;

    assert!(s.add_curve(invalid).is_none());
    assert_eq!(s.file_ids().collect::<Vec<_>>(), vec![loaded_id.as_str()]);
    assert_eq!(s.active_file_id(), Some(loaded_id.as_str()));
    assert_eq!(s.generation(), initial_generation);
}

#[test]
fn source_path_loaded_queries_loaded_curve_sources() {
    let mut s = Session::new();
    let loaded_path = PathBuf::from("tests/fixtures/source-path-loaded.csv");
    let other_path = PathBuf::from("tests/fixtures/other-source.csv");
    let mut loaded = curve("loaded.csv", 1.0);
    loaded.source_path = Some(loaded_path.clone());

    s.add_curve(loaded).unwrap();

    assert!(s.source_path_loaded(&loaded_path));
    assert!(!s.source_path_loaded(&other_path));
}

#[test]
fn file_list_row_snapshots_loaded_file_display_state() {
    let mut s = Session::new();
    let id = s.add_curve(curve("a.csv", 1.0)).unwrap();

    assert!(s.set_file_checked(&id, true));
    assert!(s.set_expert_window(&id, ExpertWindow::FwdVt, Some((0.0, 1.0))));
    let row = s.file_list_row(&id).expect("file-list row");

    assert_eq!(row.file_id, id);
    assert_eq!(row.name, "a.csv");
    assert_eq!(row.point_count, 160);
    assert!(row.is_checked);
    assert!(row.is_selected);
    assert!(row.manual_ranges);
    assert_eq!(s.file_list_row("missing"), None);
}

#[test]
fn selected_file_metrics_projects_the_total_result() {
    let mut s = Session::new();

    assert_eq!(s.selected_file_metrics_projection(), None);

    let id = s.add_curve(curve("a.csv", 1.0)).unwrap();
    let selected = s
        .selected_file_metrics_projection()
        .expect("selected metrics");
    assert_eq!(selected.filename, "a.csv");
    assert_eq!(selected.result.filename, "a.csv");
    assert_eq!(s.active_file_id(), Some(id.as_str()));
}

#[test]
fn selected_fit_window_file_snapshots_selector_inputs() {
    let mut s = Session::new();
    assert_eq!(s.selected_fit_window_file(), None);

    let id = s.add_curve(curve("a.csv", 1.0)).unwrap();
    assert!(s.set_expert_window(&id, ExpertWindow::FwdVt, Some((0.0, 1.0))));

    let selected = s
        .selected_fit_window_file()
        .expect("selected fit-window file");

    assert_eq!(selected.file_id, id.as_str());
    assert_eq!(selected.expert_ranges.vt_range, Some((0.0, 1.0)));
    assert!(!selected.has_backward_sweep);
    assert_eq!(selected.vg.len(), 160);
    assert_eq!(selected.id_abs.len(), 160);
    assert!(selected.vt_window.is_some());
    assert!(selected.ss_window.is_some());
}

#[test]
fn insertion_order_is_preserved() {
    let mut s = Session::new();
    let a = s.add_curve(curve("a.csv", 0.5)).unwrap();
    let b = s.add_curve(curve("b.csv", 1.0)).unwrap();
    let c = s.add_curve(curve("c.csv", 1.5)).unwrap();
    let order: Vec<&str> = s.file_ids().collect();
    assert_eq!(order, vec![a.as_str(), b.as_str(), c.as_str()]);
}

#[test]
fn remove_keeps_order_and_reselects() {
    let mut s = Session::new();
    let a = s.add_curve(curve("a.csv", 0.5)).unwrap();
    let b = s.add_curve(curve("b.csv", 1.0)).unwrap();
    let c = s.add_curve(curve("c.csv", 1.5)).unwrap();
    assert!(s.select_file(&b));
    assert_eq!(s.remove_selected_or_checked(), 1);
    assert_eq!(
        s.file_ids().collect::<Vec<_>>(),
        vec![a.as_str(), c.as_str()]
    );
    // selected was removed -> falls back to first remaining
    assert_eq!(s.active_file_id(), Some(a.as_str()));
}

#[test]
fn remove_selected_or_checked_prefers_checked_then_selected() {
    let mut s = Session::new();
    let a = s.add_curve(curve("a.csv", 0.5)).unwrap();
    let b = s.add_curve(curve("b.csv", 1.0)).unwrap();
    let c = s.add_curve(curve("c.csv", 1.5)).unwrap();
    assert!(s.select_file(&c));
    assert!(s.set_file_checked(&b, true));

    assert_eq!(s.remove_selected_or_checked(), 1);
    assert!(s.has_file(&a));
    assert!(!s.has_file(&b));
    assert!(s.has_file(&c));

    assert_eq!(s.remove_selected_or_checked(), 1);
    assert!(s.has_file(&a));
    assert!(!s.has_file(&c));
}

#[test]
fn keep_checked_and_clear_files_apply_bulk_file_policy() {
    let mut s = Session::new();
    let a = s.add_curve(curve("a.csv", 0.5)).unwrap();
    let b = s.add_curve(curve("b.csv", 1.0)).unwrap();
    let c = s.add_curve(curve("c.csv", 1.5)).unwrap();

    assert!(!s.has_checked_files());
    assert!(s.has_unchecked_files());
    assert_eq!(s.keep_checked_files(), None);

    assert!(s.set_file_checked(&b, true));
    assert_eq!(s.keep_checked_files(), Some(2));
    assert!(!s.has_file(&a));
    assert!(s.has_file(&b));
    assert!(!s.has_file(&c));
    assert!(!s.has_unchecked_files());

    assert_eq!(s.clear_files(), 1);
    assert!(!s.has_files());
}

#[test]
fn set_cox_updates_settings_and_recomputes() {
    let mut s = Session::new();
    s.add_curve(curve("a.csv", 1.0)).unwrap();
    s.set_cox(25.0).expect("positive finite Cox commits");
    assert_eq!(s.cox_nf_per_cm2(), 25.0);
    assert_eq!(
        s.selected_file_metrics_projection()
            .expect("selected metrics")
            .result
            .filename,
        "a.csv"
    );
}

#[test]
fn set_cox_rejects_invalid_values_atomically() {
    let mut s = Session::new();
    s.add_curve(curve("a.csv", 1.0)).unwrap();
    let initial_cox = s.cox_nf_per_cm2();
    let initial_generation = s.generation();

    for invalid in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(s.set_cox(invalid).is_err());
        assert_eq!(s.cox_nf_per_cm2(), initial_cox);
        assert_eq!(s.generation(), initial_generation);
        assert_eq!(
            s.selected_file_metrics_projection()
                .expect("selected metrics")
                .result
                .filename,
            "a.csv"
        );
    }
}

#[test]
fn output_fit_range_updates_results_and_different_source_replacement_resets_range() {
    let mut s = Session::new();
    let id = s.add_curve(curve("device_a.csv", 1.0)).unwrap();
    assert!(expect_attached(s.attach_output(output("device_a_output.csv", 1.0)), &id,).is_none());

    let default = s.output_report_rows()[0].clone();
    assert_eq!(default.fit_range, Some((2.0, 3.0)));

    assert!(s.set_output_fit_range(&id, Some((0.0, 1.0))));
    assert_eq!(
        s.selected_output_file()
            .expect("selected output")
            .selected_fit_range,
        Some((0.0, 1.0))
    );
    let ranged = s.output_report_rows()[0].clone();
    assert_eq!(ranged.fit_range, Some((0.0, 1.0)));
    assert_ne!(ranged.gds, default.gds);
    assert_ne!(ranged.idsat, default.idsat);

    let displaced = s
        .replace_output(&id, output("device_a_id-vd.csv", 2.0))
        .expect("loaded transfer exists")
        .expect("different source returns the prior output");
    assert_eq!(displaced.name, "device_a_output.csv");
    assert_eq!(
        s.selected_output_file()
            .expect("selected output")
            .selected_fit_range,
        None
    );
    assert_eq!(s.output_report_rows()[0].fit_range, Some((2.0, 3.0)));
}

#[test]
fn same_source_output_reattach_keeps_manual_fit_range() {
    // A folder re-scan re-parses every output file on disk; re-attaching the
    // same source must not silently reset a hand-tuned V_D fit range.
    let mut s = Session::new();
    let id = s.add_curve(curve("device_a.csv", 1.0)).unwrap();
    let crate_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut first = output("device_a_output.csv", 1.0);
    first.source_path = Some(crate_root.join("Cargo.toml"));
    let mut reimport = first.clone();
    reimport.source_path = Some(crate_root.join("src").join("..").join("Cargo.toml"));
    assert!(expect_attached(s.attach_output(first), &id).is_none());
    assert!(s.set_output_fit_range(&id, Some((0.0, 1.0))));

    assert!(
        expect_attached(s.attach_output(reimport), &id).is_none(),
        "a canonical alias reload must not report a displaced dataset"
    );
    assert_eq!(
        s.selected_output_file()
            .expect("selected output")
            .selected_fit_range,
        Some((0.0, 1.0))
    );

    // A replacement from a different source still resets the range.
    let mut other = output("device_a_output.csv", 2.0);
    other.source_path = Some(crate_root.join("src").join("lib.rs"));
    assert!(s.replace_output(&id, other).is_ok());
    assert_eq!(
        s.selected_output_file()
            .expect("selected output")
            .selected_fit_range,
        None
    );
}

#[test]
fn pathless_same_name_automatic_reattach_keeps_range_without_displacement() {
    let mut session = Session::new();
    let id = session.add_curve(curve("device_a.csv", 1.0)).unwrap();
    assert!(expect_attached(
        session.attach_output(output("device_a_output.csv", 1.0)),
        &id,
    )
    .is_none());
    assert!(session.set_output_fit_range(&id, Some((0.0, 1.0))));

    assert!(
        expect_attached(
            session.attach_output(output("device_a_output.csv", 2.0)),
            &id,
        )
        .is_none(),
        "an equal-name in-memory reload is the same source"
    );
    let selected = session
        .selected_output_file()
        .expect("transfer remains selected");
    assert_eq!(selected.selected_fit_range, Some((0.0, 1.0)));
    assert_eq!(
        selected.output.expect("reloaded output").curves[0].id[1],
        2.0e-6
    );
}
