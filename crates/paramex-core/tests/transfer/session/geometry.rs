use paramex_core::transfer::{
    DeviceGeometry, ExpertWindow, ExtractionSettings, GeometryError, GeometrySource, ParsedCurve,
    Session,
};

fn curve(name: &str) -> ParsedCurve {
    ParsedCurve {
        name: name.to_string(),
        vg: (0..12).map(|index| index as f64 / 11.0).collect(),
        id_abs: (1..=12).map(|index| index as f64 * 1e-9).collect(),
        source_path: None,
    }
}

#[test]
fn transfer_geometry_settings_defaults_stay_public() {
    let default_geometry = DeviceGeometry::default();
    assert_eq!(default_geometry.width_um, 1500.0);
    assert_eq!(default_geometry.length_um, 50.0);
    assert_eq!(default_geometry.source, GeometrySource::Default);
    assert_eq!(default_geometry.aspect_ratio(), 30.0);

    let invalid_geometry = DeviceGeometry {
        width_um: 1500.0,
        length_um: 0.0,
        source: GeometrySource::Manual,
    };
    assert!(invalid_geometry.aspect_ratio().is_nan());

    let default_settings = ExtractionSettings::default();
    assert_eq!(default_settings.cox_nf_per_cm2, 10.0);
    assert_eq!(default_settings.cox_f_per_cm2(), 10.0e-9);
}

#[test]
fn session_manual_geometry_validates_recomputes_and_ignores_missing_ids() {
    let mut session = Session::new();
    let id = session.add_curve(curve("a.csv")).unwrap();
    let initial_generation = session.generation();

    assert_eq!(
        session.set_manual_geometry(&id, Some(250.0), None),
        Ok(true)
    );
    assert_eq!(session.generation(), initial_generation + 1);
    let row = session
        .file_geometry_rows()
        .into_iter()
        .find(|row| row.file_id == id)
        .expect("geometry row");
    assert_eq!((row.width_um, row.length_um), (250.0, 50.0));
    assert_eq!(row.source, GeometrySource::Manual);
    let selected = session
        .selected_file_metrics_projection()
        .expect("selected metrics");
    assert_eq!(selected.result.width_um, 250.0);

    let generation = session.generation();
    assert_eq!(
        session.set_manual_geometry(&id, Some(0.0), None),
        Err(GeometryError)
    );
    assert_eq!(
        session.set_manual_geometry("missing", Some(300.0), None),
        Ok(false)
    );
    assert_eq!(session.generation(), generation);
}

#[test]
fn session_file_geometry_rows_snapshot_loaded_geometry_in_order() {
    let mut session = Session::new();
    let a = session.add_curve(curve("a.csv")).unwrap();
    let b = session.add_curve(curve("b.csv")).unwrap();
    session
        .set_manual_geometry(&b, Some(240.0), Some(12.0))
        .expect("manual geometry applies");

    let rows = session.file_geometry_rows();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].file_id, a);
    assert_eq!(rows[0].name, "a.csv");
    assert_eq!(
        (rows[0].width_um, rows[0].length_um, rows[0].source.as_str()),
        (1500.0, 50.0, "default")
    );
    assert_eq!(rows[1].file_id, b);
    assert_eq!(rows[1].name, "b.csv");
    assert_eq!(
        (rows[1].width_um, rows[1].length_um, rows[1].source.as_str()),
        (240.0, 12.0, "manual")
    );
}

#[test]
fn session_expert_window_commands_project_clamped_values() {
    let mut session = Session::new();
    let id = session.add_curve(curve("a.csv")).unwrap();
    let initial_generation = session.generation();

    assert!(session.set_expert_window(&id, ExpertWindow::FwdVt, Some((2.0, 0.0))));
    assert_eq!(session.generation(), initial_generation + 1);
    assert_eq!(
        session
            .selected_fit_window_file()
            .expect("selected fit-window file")
            .expert_ranges
            .vt_range,
        Some((0.0, 1.0))
    );

    assert!(session.set_expert_window(&id, ExpertWindow::BwdSs, Some((0.25, 0.75))));
    assert_eq!(
        session
            .selected_fit_window_file()
            .expect("selected fit-window file")
            .expert_ranges
            .ss_range_bwd,
        Some((0.25, 0.75))
    );

    let generation = session.generation();
    assert!(!session.set_expert_window("missing", ExpertWindow::FwdSs, Some((0.2, 0.8))));
    assert!(!session.clear_expert_windows("missing"));
    assert_eq!(session.generation(), generation);

    assert!(session.clear_expert_windows(&id));
    assert_eq!(
        session
            .selected_fit_window_file()
            .expect("selected fit-window file")
            .expert_ranges,
        Default::default()
    );
    assert_eq!(session.generation(), generation + 1);
}

#[test]
fn session_expert_window_rejects_non_finite_endpoints_atomically() {
    let mut session = Session::new();
    let id = session.add_curve(curve("a.csv")).unwrap();
    assert!(session.set_expert_window(&id, ExpertWindow::FwdVt, Some((0.25, 0.75))));
    let generation = session.generation();

    for window in [
        Some((f64::NAN, 0.5)),
        Some((0.5, f64::INFINITY)),
        Some((f64::NEG_INFINITY, 0.5)),
    ] {
        assert!(!session.set_expert_window(&id, ExpertWindow::FwdVt, window));
    }

    assert_eq!(
        session
            .selected_fit_window_file()
            .expect("selected fit-window file")
            .expert_ranges
            .vt_range,
        Some((0.25, 0.75))
    );
    assert_eq!(session.generation(), generation);
}
