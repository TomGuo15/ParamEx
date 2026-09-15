use crate::common::{assert_close, load_numpy_reference, parse_f64};

#[test]
fn harness_roundtrips_floats_and_specials() {
    let reference = load_numpy_reference("harness_smoke");
    let cases = reference["cases"].as_array().expect("cases array");
    let vals: Vec<f64> = cases.iter().map(|c| parse_f64(&c["value"])).collect();

    assert_close(vals[0], 1.5, 1e-12, 1e-12);
    assert!(vals[1].is_nan());
    assert_eq!(vals[2], f64::INFINITY);
    assert_eq!(vals[3], f64::NEG_INFINITY);

    // assert_close must treat NaN/inf correctly
    assert_close(f64::NAN, f64::NAN, 0.0, 0.0);
    assert_close(f64::INFINITY, f64::INFINITY, 0.0, 0.0);
}
