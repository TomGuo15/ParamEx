use crate::common::{assert_close, load_numpy_reference, parse_f64};
use paramex_core::shared::numpy_compat::ptp;

#[test]
fn ptp_matches_numpy_reference() {
    let reference = load_numpy_reference("ptp");
    let cases = reference["cases"].as_array().expect("cases array");
    assert!(!cases.is_empty(), "reference has no cases");

    for case in cases {
        let vals: Vec<f64> = case["vals"]
            .as_array()
            .expect("vals array")
            .iter()
            .map(parse_f64)
            .collect();
        // expected may be non-finite (empty input -> NaN), so use parse_f64.
        let expected = parse_f64(&case["expected"]);

        let actual = ptp(&vals);

        // assert_close is NaN-aware, so the empty -> NaN case is handled here.
        assert_close(actual, expected, 1e-12, 1e-12);
    }
}
