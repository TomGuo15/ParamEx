use crate::transfer::parse::numeric::{looks_like_scope_trace, looks_like_spectrum_trace};
use crate::transfer::test_support::{grid_from, load_reference_in};

#[test]
fn trace_rejection_matches_reference() {
    let reference = load_reference_in("parse", "reject_traces");
    let cases = reference["cases"].as_array().expect("cases array");
    assert!(!cases.is_empty(), "reference has no cases");

    for (i, case) in cases.iter().enumerate() {
        let grid = grid_from(&case["grid"]);
        assert_eq!(
            looks_like_scope_trace(&grid),
            case["scope"].as_bool().unwrap(),
            "case {i}: scope"
        );
        assert_eq!(
            looks_like_spectrum_trace(&grid),
            case["spectrum"].as_bool().unwrap(),
            "case {i}: spectrum"
        );
    }
}
