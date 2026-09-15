use crate::transfer::metrics::on_off::on_off_ratio;
use crate::transfer::test_support::{assert_close, load_reference_in, parse_f64};

#[test]
fn on_off_matches_reference() {
    let g = load_reference_in("metrics", "on_off");
    for case in g["cases"].as_array().unwrap() {
        let id_abs: Vec<f64> = case["id_abs"]
            .as_array()
            .unwrap()
            .iter()
            .map(parse_f64)
            .collect();
        let (ion, ioff, ratio) = on_off_ratio(&id_abs);
        assert_close(ion, parse_f64(&case["ion"]), 1e-12, 1e-12);
        assert_close(ioff, parse_f64(&case["ioff"]), 1e-12, 1e-12);
        assert_close(ratio, parse_f64(&case["ratio"]), 1e-9, 1e-12);
    }
}
