use crate::common::{assert_close, load_numpy_reference, parse_f64};
use paramex_core::shared::numpy_compat::std_sample;

#[test]
fn std_sample_matches_pandas() {
    let g = load_numpy_reference("std_sample");
    for case in g["cases"].as_array().unwrap() {
        let a: Vec<f64> = case["a"]
            .as_array()
            .unwrap()
            .iter()
            .map(parse_f64)
            .collect();
        assert_close(std_sample(&a), parse_f64(&case["expected"]), 1e-12, 1e-12);
    }
}
