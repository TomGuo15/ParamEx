use super::transform_of;
use crate::common::{assert_close, load_reference_in, parse_f64};
use paramex_core::transfer::{SweepData, Transform, WindowedFitter};

#[test]
fn windowed_fitter_indices_matches_reference() {
    let reference = load_reference_in("fit", "windowed_fitter_indices");
    let cases = reference["cases"].as_array().expect("cases array");
    assert!(!cases.is_empty(), "reference has no cases");

    for (ci, case) in cases.iter().enumerate() {
        let vg: Vec<f64> = case["vg"]
            .as_array()
            .expect("vg")
            .iter()
            .map(parse_f64)
            .collect();
        let id_abs: Vec<f64> = case["id_abs"]
            .as_array()
            .expect("id")
            .iter()
            .map(parse_f64)
            .collect();
        let transform = transform_of(case["transform"].as_str().expect("transform"));
        let fitter = WindowedFitter::new(&SweepData { vg, id_abs }, transform);

        for w in case["windows"].as_array().expect("windows") {
            let start = w["start"].as_u64().expect("start") as usize;
            let end = w["end"].as_u64().expect("end") as usize;
            let res = fitter.fit_indices(start, end);

            assert_eq!(
                res.points,
                w["points"].as_u64().expect("points") as usize,
                "case {ci} window [{start},{end}): points"
            );
            // One-pass closed-form engine: should match the reference to ~1e-12.
            assert_close(res.slope, parse_f64(&w["slope"]), 1e-12, 1e-12);
            assert_close(res.intercept, parse_f64(&w["intercept"]), 1e-12, 1e-12);
            assert_close(res.r2, parse_f64(&w["r2"]), 1e-12, 1e-12);
            if res.r2.is_finite() {
                assert!(
                    res.r2 <= 1.0,
                    "case {ci} window [{start},{end}): r2 must be clamped <= 1.0"
                );
            }
        }
    }
}

#[test]
fn fit_indices_clamps_out_of_range_indices_instead_of_panicking() {
    let sweep = SweepData {
        vg: vec![0.0, 1.0, 2.0, 3.0, 4.0],
        id_abs: vec![1e-6, 2e-6, 3e-6, 4e-6, 5e-6],
    };
    let fitter = WindowedFitter::new(&sweep, Transform::Sqrt);

    // end far past the data: clamps to the available samples, identical to a
    // full-window fit (stale GUI indices must not abort the app).
    let clamped = fitter.fit_indices(0, 1_000);
    let full = fitter.fit_indices(0, 5);
    assert_eq!(clamped.slope.to_bits(), full.slope.to_bits(), "slope");
    assert_eq!(clamped.r2.to_bits(), full.r2.to_bits(), "r2");
    assert_eq!(clamped.points, full.points, "points");

    // window entirely past the data: too few samples -> NaN fit, no panic.
    let past = fitter.fit_indices(50, 100);
    assert!(past.slope.is_nan(), "slope NaN for empty clamped window");
    assert_eq!(past.points, 0, "no points in an empty clamped window");
}
