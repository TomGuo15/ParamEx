use crate::common::{load_numpy_reference, parse_f64};
use paramex_core::shared::numpy_compat::{nanargmax, nanargmin};

fn expected_index(v: &serde_json::Value) -> Option<usize> {
    if v.is_null() {
        None
    } else {
        Some(
            v.as_u64()
                .expect("expected index must be a non-negative integer") as usize,
        )
    }
}

#[test]
fn nanargmax_nanargmin_match_numpy_reference() {
    let reference = load_numpy_reference("nanargmax_nanargmin");
    let cases = reference["cases"]
        .as_array()
        .expect("cases must be an array");

    for (i, case) in cases.iter().enumerate() {
        let vals: Vec<f64> = case["vals"]
            .as_array()
            .expect("vals must be an array")
            .iter()
            .map(parse_f64)
            .collect();

        let want_max = expected_index(&case["argmax"]);
        let want_min = expected_index(&case["argmin"]);

        // Indices are exact: no tolerance.
        assert_eq!(
            nanargmax(&vals),
            want_max,
            "nanargmax mismatch at case {i}: vals={vals:?}"
        );
        assert_eq!(
            nanargmin(&vals),
            want_min,
            "nanargmin mismatch at case {i}: vals={vals:?}"
        );
    }
}
