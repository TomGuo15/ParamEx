use crate::common::{assert_close, load_numpy_reference, parse_f64};
use paramex_core::shared::numpy_compat::nanmedian;

#[test]
fn nanmedian_matches_numpy_reference() {
    let reference = load_numpy_reference("nanmedian");
    let cases = reference["cases"].as_array().expect("cases array");
    for case in cases {
        let vals: Vec<f64> = case["vals"]
            .as_array()
            .expect("vals array")
            .iter()
            .map(parse_f64)
            .collect();
        let expected = parse_f64(&case["expected"]);
        let actual = nanmedian(&vals);
        assert_close(actual, expected, 1e-12, 1e-12);
    }
}
