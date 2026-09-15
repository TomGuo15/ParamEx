use crate::common::{assert_close, load_numpy_reference, parse_f64};
use paramex_core::shared::numpy_compat::banker_round;

#[test]
fn banker_round_matches_numpy_reference() {
    let reference = load_numpy_reference("banker_round");
    let cases = reference["cases"]
        .as_array()
        .expect("cases must be an array");
    assert!(!cases.is_empty(), "reference file has no cases");

    for (i, case) in cases.iter().enumerate() {
        let x = parse_f64(&case["x"]); // may be nan/inf
        let expected = parse_f64(&case["y"]); // may be nan/inf
        let actual = banker_round(x);
        // assert_close is NaN-aware & inf-aware; for finite outputs the result
        // is an exact integer-valued f64, but we still use the shared tolerance.
        assert_close(actual, expected, 1e-12, 1e-12);
        // also assert it is NaN-for-NaN / inf-for-inf at the bit-class level
        if x.is_nan() {
            assert!(actual.is_nan(), "case {i}: expected NaN for NaN input");
        }
    }
}

#[test]
fn banker_round_spec_ties_to_even() {
    // The defining contract, asserted directly (independent of the reference file).
    assert_eq!(banker_round(0.5), 0.0);
    assert_eq!(banker_round(1.5), 2.0);
    assert_eq!(banker_round(2.5), 2.0);
    assert_eq!(banker_round(3.5), 4.0);
    assert_eq!(banker_round(-0.5), 0.0); // -0.0 == 0.0
    assert_eq!(banker_round(-2.5), -2.0);
}

#[test]
fn banker_round_usize_cast_idiom() {
    // Documented caller pattern: (banker_round(n * frac)) as usize.
    let n = 7.0_f64;
    let frac = 0.5_f64; // 3.5 -> ties to even -> 4
    assert_eq!(banker_round(n * frac) as usize, 4usize);
    let frac2 = 0.5_f64 / 1.0; // sanity: 2.5 -> 2
    assert_eq!(banker_round(5.0 * frac2) as usize, 2usize);
}
