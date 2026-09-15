use crate::transfer::parse::parse_grid;
use crate::transfer::test_support::{assert_close, f64_vec, grid_from, load_reference_in};

// Coerced floats compare at rtol=1e-12, NOT exact: the reference values come
// from a decimal parser that is not correctly rounded and can differ from Rust
// `f64::from_str` by ≤1 ULP (e.g. "3.16…794e-12"). Structure (lengths, names)
// stays exact.
const RTOL: f64 = 1e-12;
const ATOL: f64 = 0.0;

#[test]
fn parse_grid_matches_reference() {
    let reference = load_reference_in("parse", "parse_grid");
    let cases = reference["cases"].as_array().expect("cases array");
    assert!(!cases.is_empty(), "reference has no cases");

    for (i, case) in cases.iter().enumerate() {
        let grid = grid_from(&case["grid"]);
        let name = case["name"].as_str().unwrap();
        let actual = parse_grid(&grid, name, None);

        if case["expected"].is_null() {
            assert!(
                actual.is_none(),
                "case {i} ({name}): expected None, got {actual:?}"
            );
        } else {
            let curve = actual.unwrap_or_else(|| panic!("case {i} ({name}): expected Some"));
            let exp = &case["expected"];
            assert_eq!(curve.name, exp["name"].as_str().unwrap(), "case {i}: name");
            let exp_vg = f64_vec(&exp["vg"]);
            assert_eq!(curve.vg.len(), exp_vg.len(), "case {i}: vg len");
            for (&a, &e) in curve.vg.iter().zip(exp_vg.iter()) {
                assert_close(a, e, RTOL, ATOL);
            }
            let exp_id = f64_vec(&exp["id_abs"]);
            for (&a, &e) in curve.id_abs.iter().zip(exp_id.iter()) {
                assert_close(a, e, RTOL, ATOL);
            }
        }
    }
}
