use paramex_core::transfer::{GeometryError, GeometrySource, ParsedCurve, Session};

fn curve(name: &str) -> ParsedCurve {
    ParsedCurve {
        name: name.to_string(),
        vg: (0..12)
            .map(|index| -1.0 + 5.0 * index as f64 / 11.0)
            .collect(),
        id_abs: (1..=12).map(|index| index as f64 * 1e-9).collect(),
        source_path: Some(std::path::PathBuf::from(name)),
    }
}

#[test]
fn set_global_wl_applies_and_recomputes() {
    let mut s = Session::new();
    s.add_curve(curve("a.csv"));
    s.add_curve(curve("b.csv"));

    let n = s
        .set_global_wl(220.0, 11.0)
        .expect("positive W/L is accepted");
    assert_eq!(n, 2);

    let ids: Vec<String> = s.file_ids().map(str::to_string).collect();
    for id in ids {
        let row = s
            .file_geometry_rows()
            .into_iter()
            .find(|row| row.file_id == id)
            .expect("geometry row");
        assert_eq!(row.width_um, 220.0);
        assert_eq!(row.length_um, 11.0);
        assert_eq!(row.source, GeometrySource::Global);
        // recompute_all ran: the stored result reflects the new geometry.
        assert!(s.select_file(&id));
        let selected = s
            .selected_file_metrics_projection()
            .expect("selected metrics");
        assert_eq!(selected.result.width_um, 220.0);
        assert_eq!(selected.result.length_um, 11.0);
        assert_eq!(selected.result.aspect_ratio, 20.0);
    }
}

#[test]
fn set_global_wl_rejects_nonpositive_without_mutating() {
    let mut s = Session::new();
    s.add_curve(curve("a.csv"));
    let id = s.file_ids().next().unwrap().to_string();
    let before = s
        .file_geometry_rows()
        .into_iter()
        .find(|row| row.file_id == id)
        .expect("geometry row");

    let err = s.set_global_wl(0.0, 10.0).unwrap_err();
    assert_eq!(err, GeometryError);

    // Validation happens before the session mutates any loaded geometry.
    let after = s
        .file_geometry_rows()
        .into_iter()
        .find(|row| row.file_id == id)
        .expect("geometry row");
    assert_eq!(before, after);
}
