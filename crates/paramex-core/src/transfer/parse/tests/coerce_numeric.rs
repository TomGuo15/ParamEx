use crate::shared::grid_ingest::coerce_numeric;
use crate::transfer::test_support::{assert_close, load_reference_in, parse_f64};

// Compare at rtol=1e-12 (NaN/inf-aware via assert_close), NOT exact: the
// reference values come from a decimal parser that is not correctly rounded
// and can differ from Rust `f64::from_str` by ≤1 ULP on some decimal strings
// (e.g. "3.16…794e-12").
const RTOL: f64 = 1e-12;
const ATOL: f64 = 0.0;

#[test]
fn coerce_numeric_matches_reference() {
    let reference = load_reference_in("parse", "coerce_numeric");
    let cases = reference["cases"].as_array().expect("cases array");
    assert!(!cases.is_empty(), "reference has no cases");

    for case in cases {
        let input = case["input"].as_str().expect("input");
        let expected = parse_f64(&case["expected"]);
        let actual = coerce_numeric(input);
        // assert_close is NaN/inf-aware (NaN matches NaN, ±inf matches same ±inf).
        assert_close(actual, expected, RTOL, ATOL);
    }
}
