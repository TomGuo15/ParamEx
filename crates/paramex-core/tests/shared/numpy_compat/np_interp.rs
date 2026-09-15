use crate::common::{assert_close, load_numpy_reference, parse_f64};
use paramex_core::shared::numpy_compat::interp;

#[test]
fn interp_matches_numpy_reference() {
    let reference = load_numpy_reference("interp");
    let cases = reference["cases"]
        .as_array()
        .expect("cases must be an array");
    assert!(!cases.is_empty(), "reference file has no cases");

    for case in cases {
        let label = case["label"].as_str().unwrap_or("<unlabeled>");

        let xq: Vec<f64> = case["xq"]
            .as_array()
            .expect("xq array")
            .iter()
            .map(parse_f64)
            .collect();
        let xp: Vec<f64> = case["xp"]
            .as_array()
            .expect("xp array")
            .iter()
            .map(parse_f64)
            .collect();
        let fp: Vec<f64> = case["fp"]
            .as_array()
            .expect("fp array")
            .iter()
            .map(parse_f64)
            .collect();
        let expected: Vec<f64> = case["expected"]
            .as_array()
            .expect("expected array")
            .iter()
            .map(parse_f64)
            .collect();

        let actual = interp(&xq, &xp, &fp);

        assert_eq!(
            actual.len(),
            expected.len(),
            "case '{}': output length {} != expected {}",
            label,
            actual.len(),
            expected.len()
        );

        for (&a, &e) in actual.iter().zip(expected.iter()) {
            // rtol=1e-12, atol=1e-12; assert_close is NaN-/inf-aware.
            assert_close(a, e, 1e-12, 1e-12);
        }
    }
}

#[test]
fn nan_query_returns_nan_without_panicking() {
    // A NaN query satisfies neither clamp comparison; it must return NaN (numpy
    // semantics) instead of underflowing the interior index search.
    let out = interp(&[f64::NAN, 1.5], &[0.0, 1.0, 2.0], &[10.0, 20.0, 30.0]);
    assert!(out[0].is_nan(), "NaN query must map to NaN");
    assert!(
        (out[1] - 25.0).abs() < 1e-12,
        "1.5 interpolates to 25 (interior)"
    );
}
