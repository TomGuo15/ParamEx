use crate::common::{assert_close, load_numpy_reference, parse_f64};
use paramex_core::shared::numpy_compat::linspace;

const RTOL: f64 = 1e-12;
const ATOL: f64 = 1e-12;

#[test]
fn linspace_matches_numpy_reference() {
    let reference = load_numpy_reference("linspace");
    let cases = reference["cases"]
        .as_array()
        .expect("cases must be an array");
    assert!(!cases.is_empty(), "reference file has no cases");

    for (ci, case) in cases.iter().enumerate() {
        let lo = parse_f64(&case["lo"]);
        let hi = parse_f64(&case["hi"]);
        let n = case["n"].as_u64().expect("n must be a u64") as usize;

        let expected = case["expected"]
            .as_array()
            .expect("expected must be an array");

        let actual = linspace(lo, hi, n);

        // Length is an exact count -> exact comparison.
        assert_eq!(
            actual.len(),
            expected.len(),
            "case {ci}: length mismatch for linspace({lo}, {hi}, {n})"
        );
        assert_eq!(
            actual.len(),
            n,
            "case {ci}: linspace must return exactly n elements"
        );

        for (i, ev) in expected.iter().enumerate() {
            let e = parse_f64(ev);
            assert_close(actual[i], e, RTOL, ATOL);
        }

        // numpy guarantee: when n >= 2 the final sample is exactly hi.
        if n >= 2 {
            assert_eq!(
                actual[n - 1],
                hi,
                "case {ci}: last point must equal hi exactly"
            );
        }
        // numpy guarantee: when n == 1 the single sample is exactly lo.
        if n == 1 {
            assert_eq!(
                actual[0], lo,
                "case {ci}: single point must equal lo exactly"
            );
        }
    }
}
