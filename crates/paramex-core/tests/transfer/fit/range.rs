use super::transform_of;
use crate::common::{assert_close, load_reference_in, parse_f64};
use paramex_core::transfer::{SweepData, WindowedFitter};

#[test]
fn windowed_fitter_range_matches_reference() {
    let reference = load_reference_in("fit", "windowed_fitter_range");
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

        for r in case["ranges"].as_array().expect("ranges") {
            let range = if r["range"].is_null() {
                None
            } else {
                let a = r["range"].as_array().expect("range pair");
                Some((parse_f64(&a[0]), parse_f64(&a[1])))
            };
            let res = fitter.fit(range);

            assert_eq!(
                res.points,
                r["points"].as_u64().expect("points") as usize,
                "case {ci} range {range:?}: points"
            );
            assert_close(res.slope, parse_f64(&r["slope"]), 1e-12, 1e-12);
            assert_close(res.intercept, parse_f64(&r["intercept"]), 1e-12, 1e-12);
            assert_close(res.r2, parse_f64(&r["r2"]), 1e-12, 1e-12);
        }
    }
}
