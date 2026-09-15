use crate::common::{load_numpy_reference, parse_f64};
use paramex_core::shared::numpy_compat::argsort;

#[test]
fn argsort_matches_numpy_on_unique_keys() {
    let g = load_numpy_reference("argsort");
    for case in g["cases"].as_array().unwrap() {
        let x: Vec<f64> = case["x"]
            .as_array()
            .unwrap()
            .iter()
            .map(parse_f64)
            .collect();
        let expected: Vec<usize> = case["order"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as usize)
            .collect();
        let order = argsort(&x);
        assert_eq!(order, expected, "argsort mismatch for x={x:?}");
    }
}
