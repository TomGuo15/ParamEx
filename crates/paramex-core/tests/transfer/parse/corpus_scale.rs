use crate::common::{assert_close, f64_vec, load_reference_in, parse_fixture_dir};
use paramex_core::transfer::parse_transfer_file;

// Coerced floats compare at rtol=1e-12, not exact. The
// 400-point synthetic curves hit many such ≤1-ULP to_numeric-vs-from_str gaps,
// so exact equality here would fail outright.
const RTOL: f64 = 1e-12;
const ATOL: f64 = 0.0;

#[test]
fn corpus_scale_matches_reference() {
    let reference = load_reference_in("parse", "corpus_scale");
    let cases = reference["cases"].as_array().expect("cases");
    assert!(!cases.is_empty(), "reference has no cases");

    for case in cases {
        let fixture = case["fixture"].as_str().unwrap();
        let exp = &case["expected"];
        let curve = parse_transfer_file(&parse_fixture_dir().join(fixture))
            .unwrap_or_else(|e| panic!("{fixture}: {e:?}"));
        assert_eq!(curve.name, exp["name"].as_str().unwrap(), "{fixture}: name");
        let exp_vg = f64_vec(&exp["vg"]);
        assert_eq!(curve.vg.len(), exp_vg.len(), "{fixture}: vg len");
        for (&a, &e) in curve.vg.iter().zip(exp_vg.iter()) {
            assert_close(a, e, RTOL, ATOL);
        }
        let exp_id = f64_vec(&exp["id_abs"]);
        for (&a, &e) in curve.id_abs.iter().zip(exp_id.iter()) {
            assert_close(a, e, RTOL, ATOL);
        }
    }
}
