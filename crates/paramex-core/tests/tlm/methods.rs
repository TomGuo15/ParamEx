use paramex_core::tlm::{Status, TlmCurve, TlmSample, VdSource};

fn curve(vg: &[f64], id: &[f64], is: &[f64]) -> TlmCurve {
    assert_eq!(vg.len(), id.len());
    assert_eq!(vg.len(), is.len());
    let samples = vg
        .iter()
        .zip(id)
        .zip(is)
        .map(|((&vg, &id), &is)| TlmSample::try_new(vg, id, is).unwrap())
        .collect();
    TlmCurve::try_new(
        "x.xlsx".into(),
        "g".into(),
        50.0,
        samples,
        -0.5,
        VdSource::Setup,
    )
    .unwrap()
}

#[test]
fn current_at_picks_nearest_vg_and_min_of_id_is() {
    // vg ascending; at -40 the exact index is used; current = min(abs_id, abs_is).
    let c = curve(
        &[-40.0, -39.0, -38.0],
        &[-2e-6, 3e-6, 4e-6],
        &[1e-6, 5e-6, 6e-6],
    );
    let (current, actual) = c.current_at(-40.0);
    assert_eq!(actual, -40.0);
    assert_eq!(current, 1e-6); // min(|-2e-6|, 1e-6)
}

#[test]
fn sample_constructor_normalizes_signed_current_columns() {
    let sample = TlmSample::try_new(1.0, -2e-6, -1e-6).expect("finite signed currents load");
    assert_eq!(sample.abs_id(), 2e-6);
    assert_eq!(sample.abs_is(), 1e-6);
}

#[test]
fn status_and_vd_source_display_their_csv_spelling() {
    assert_eq!(Status::Ok.to_string(), "ok");
    assert_eq!(Status::Error.to_string(), "error");
    assert_eq!(VdSource::Setup.to_string(), "setup");
    assert_eq!(VdSource::Fallback.to_string(), "fallback");
    assert_eq!(VdSource::Unread.to_string(), "unread");
}

#[test]
fn current_at_ties_to_closer_neighbor_and_clamps_ends() {
    let c = curve(&[-40.0, -38.0], &[2e-6, 4e-6], &[2e-6, 4e-6]);
    assert_eq!(c.current_at(-50.0).1, -40.0); // below range -> first
    assert_eq!(c.current_at(0.0).1, -38.0); // above range -> last
    assert_eq!(c.current_at(-38.4).1, -38.0); // closer to -38 than -40
}

#[test]
fn sample_constructor_rejects_each_non_finite_component() {
    assert!(TlmSample::try_new(1.0, 0.0, -0.0).is_ok());
    for values in [
        (f64::NAN, 1.0, 1.0),
        (1.0, f64::INFINITY, 1.0),
        (1.0, 1.0, f64::NEG_INFINITY),
    ] {
        assert!(TlmSample::try_new(values.0, values.1, values.2).is_err());
    }
}

#[test]
fn curve_constructor_sorts_whole_samples_and_preserves_equal_vg_order() {
    let c = curve(
        &[2.0, 1.0, 1.0],
        &[20.0, 10.0, 11.0],
        &[200.0, 100.0, 110.0],
    );

    assert_eq!(
        c.samples()
            .iter()
            .map(|sample| (sample.vg(), sample.abs_id()))
            .collect::<Vec<_>>(),
        vec![(1.0, 10.0), (1.0, 11.0), (2.0, 20.0)]
    );
    assert_eq!(c.current_at(1.0), (10.0, 1.0));
    assert_eq!(c.device_id(), "x");
}

#[test]
fn current_at_keeps_first_duplicate_for_nearest_and_clamped_measurements() {
    let c = curve(
        &[1.0, 1.0, 2.0, 2.0],
        &[10.0, 11.0, 20.0, 21.0],
        &[100.0, 110.0, 200.0, 210.0],
    );

    for (requested, expected) in [
        (0.0, (10.0, 1.0)),
        (1.0, (10.0, 1.0)),
        (1.2, (10.0, 1.0)),
        (1.5, (10.0, 1.0)),
        (1.8, (20.0, 2.0)),
        (2.0, (20.0, 2.0)),
        (3.0, (20.0, 2.0)),
    ] {
        assert_eq!(c.current_at(requested), expected, "V_G={requested}");
    }
}

#[test]
fn signed_zero_gate_samples_keep_source_order() {
    let c = curve(&[1.0, 0.0, -0.0], &[1.0, 20.0, 30.0], &[1.0, 200.0, 300.0]);

    assert!(c.samples()[0].vg().is_sign_positive());
    assert!(c.samples()[1].vg().is_sign_negative());
    assert_eq!(c.current_at(0.0), (20.0, 0.0));
}

#[test]
fn normalized_unsorted_curve_uses_the_exact_measurement() {
    let c = curve(&[0.0, 2.0, 1.0], &[1.0, 20.0, 100.0], &[1.0, 20.0, 100.0]);

    assert_eq!(c.current_at(1.0), (100.0, 1.0));
}

#[test]
fn curve_constructor_rejects_empty_or_invalid_metadata() {
    let sample = TlmSample::try_new(1.0, 1.0, 1.0).unwrap();
    let build = |file: &str, group: &str, length: f64, samples: Vec<TlmSample>, vd, source| {
        TlmCurve::try_new(
            file.to_string(),
            group.to_string(),
            length,
            samples,
            vd,
            source,
        )
    };

    assert!(build("x.xlsx", "g", 50.0, Vec::new(), -0.5, VdSource::Setup).is_err());
    assert!(build("", "g", 50.0, vec![sample], -0.5, VdSource::Setup).is_err());
    assert!(build("folder/   ", "g", 50.0, vec![sample], -0.5, VdSource::Setup).is_err());
    assert!(build("x.xlsx", "", 50.0, vec![sample], -0.5, VdSource::Setup).is_err());
    assert!(build("x.xlsx", "g", f64::NAN, vec![sample], -0.5, VdSource::Setup).is_err());
    assert!(build("x.xlsx", "g", 50.0, vec![sample], 0.0, VdSource::Setup).is_err());
    assert!(build("x.xlsx", "g", 50.0, vec![sample], -0.5, VdSource::Unread).is_err());
}
