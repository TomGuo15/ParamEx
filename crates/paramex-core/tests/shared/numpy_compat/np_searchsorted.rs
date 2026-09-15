use crate::common::{load_numpy_reference, parse_f64};
use paramex_core::shared::numpy_compat::{searchsorted, Side};

#[test]
fn np_searchsorted_matches_numpy() {
    let g = load_numpy_reference("searchsorted");
    let cases = g["cases"].as_array().expect("cases array");
    assert!(!cases.is_empty(), "reference has no cases");

    for (i, case) in cases.iter().enumerate() {
        let a: Vec<f64> = case["a"]
            .as_array()
            .expect("a array")
            .iter()
            .map(parse_f64)
            .collect();
        let v = parse_f64(&case["v"]);
        let side = match case["side"].as_str().expect("side string") {
            "left" => Side::Left,
            "right" => Side::Right,
            other => panic!("unknown side {other:?}"),
        };
        let expected = case["expected"].as_u64().expect("expected index") as usize;

        let actual = searchsorted(&a, v, side);
        assert_eq!(
            actual, expected,
            "case {i}: searchsorted(a={a:?}, v={v}, side={side:?}) = {actual}, want {expected}"
        );

        // index must always be within bounds
        assert!(
            actual <= a.len(),
            "case {i}: index {actual} out of range for len {}",
            a.len()
        );
    }
}
