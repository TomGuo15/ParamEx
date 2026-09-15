use crate::shared::grid_ingest::coerce_numeric;
use crate::transfer::parse::curve::{build_curve, validate_curve_integrity};
use crate::transfer::test_support::{assert_close, f64_vec, load_reference_in};
use crate::transfer::types::ParsedCurve;

// Coerced floats compare at rtol=1e-12, not exact (the reference's decimal
// parser and Rust `from_str` can diverge by ≤1 ULP). Structure (lengths,
// names, validate booleans) stays exact.
const RTOL: f64 = 1e-12;
const ATOL: f64 = 0.0;

#[test]
fn build_curve_matches_reference() {
    let reference = load_reference_in("parse", "build_curve");

    for (i, case) in reference["build"].as_array().unwrap().iter().enumerate() {
        let vg = f64_vec(&case["vg"]);
        let current = f64_vec(&case["current"]);
        let actual = build_curve("dev.csv", None, &vg, &current);

        if case["expected"].is_null() {
            assert!(
                actual.is_none(),
                "build case {i}: expected None, got {actual:?}"
            );
        } else {
            let curve = actual.unwrap_or_else(|| panic!("build case {i}: expected Some"));
            let exp = &case["expected"];
            assert_eq!(
                curve.name,
                exp["name"].as_str().unwrap(),
                "build case {i}: name"
            );
            assert!(curve.source_path.is_none(), "build case {i}: source_path");
            let exp_vg = f64_vec(&exp["vg"]);
            let exp_id = f64_vec(&exp["id_abs"]);
            assert_eq!(curve.vg.len(), exp_vg.len(), "build case {i}: vg len");
            assert_eq!(curve.id_abs.len(), exp_id.len(), "build case {i}: id len");
            for (&a, &e) in curve.vg.iter().zip(exp_vg.iter()) {
                assert_close(a, e, RTOL, ATOL);
            }
            for (&a, &e) in curve.id_abs.iter().zip(exp_id.iter()) {
                assert_close(a, e, RTOL, ATOL);
            }
        }
    }

    for (i, case) in reference["validate"].as_array().unwrap().iter().enumerate() {
        let curve = ParsedCurve {
            name: "x".to_string(),
            vg: f64_vec(&case["vg"]),
            id_abs: f64_vec(&case["id_abs"]),
            source_path: None,
        };
        assert_eq!(
            validate_curve_integrity(&curve),
            case["valid"].as_bool().unwrap(),
            "validate case {i}"
        );
    }

    // sanity: coerce + build composes (the labeled-path call shape)
    let vg: Vec<f64> = (0..12).map(|i| coerce_numeric(&i.to_string())).collect();
    let cur: Vec<f64> = (0..12)
        .map(|i| coerce_numeric(&format!("{}", 1e-9 * (i as f64 + 1.0))))
        .collect();
    assert!(build_curve("c.csv", None, &vg, &cur).is_some());
}
