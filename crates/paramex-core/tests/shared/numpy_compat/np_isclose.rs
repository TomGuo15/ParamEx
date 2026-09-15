use crate::common::{load_numpy_reference, parse_f64};
use paramex_core::shared::numpy_compat::isclose;

#[test]
fn isclose_matches_numpy_reference() {
    let reference = load_numpy_reference("isclose");
    let cases = reference["cases"]
        .as_array()
        .expect("reference `cases` must be an array");
    assert!(!cases.is_empty(), "reference file has no cases");

    for (i, case) in cases.iter().enumerate() {
        // Any of a/b may be non-finite (nan/inf) -> parse_f64. rtol/atol are finite.
        let a = parse_f64(&case["a"]);
        let b = parse_f64(&case["b"]);
        let rtol = parse_f64(&case["rtol"]);
        let atol = parse_f64(&case["atol"]);
        let expected = case["expected"]
            .as_bool()
            .expect("`expected` must be a bool");

        let got = isclose(a, b, rtol, atol);

        // Boolean output: EXACT comparison against numpy ground truth.
        assert_eq!(
            got, expected,
            "case {i}: isclose(a={a:?}, b={b:?}, rtol={rtol:?}, atol={atol:?}) \
             = {got}, expected {expected} (numpy)"
        );
    }
}
