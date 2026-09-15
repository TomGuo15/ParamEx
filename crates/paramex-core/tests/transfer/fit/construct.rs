use super::transform_of;
use crate::common::{load_reference_in, parse_f64};
use paramex_core::transfer::{SweepData, WindowedFitter};

#[test]
fn windowed_fitter_construction_matches_reference() {
    let reference = load_reference_in("fit", "windowed_fitter_construct");
    let cases = reference["cases"].as_array().expect("cases array");
    assert!(!cases.is_empty(), "reference has no cases");

    for (i, case) in cases.iter().enumerate() {
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
        let exp_x: Vec<f64> = case["x"]
            .as_array()
            .expect("x")
            .iter()
            .map(parse_f64)
            .collect();
        let exp_n = case["n"].as_u64().expect("n") as usize;

        let fitter = WindowedFitter::new(&SweepData { vg, id_abs }, transform);

        assert_eq!(fitter.n(), exp_n, "case {i}: n");
        assert_eq!(fitter.x().len(), exp_x.len(), "case {i}: x length");
        // x is the masked, Vg-sorted ORIGINAL voltages -> exact comparison.
        for (k, (&a, &e)) in fitter.x().iter().zip(exp_x.iter()).enumerate() {
            assert!(a == e, "case {i}: x[{k}] = {a}, want {e}");
        }
    }
}
