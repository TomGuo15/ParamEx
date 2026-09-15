use crate::common::{assert_close, load_reference_in, parse_f64};
use paramex_core::transfer::calculate_stack_cox_nf_per_cm2;

#[test]
fn cox_matches_reference() {
    let g = load_reference_in("types", "cox");
    for c in g["singles"].as_array().unwrap() {
        let got = calculate_stack_cox_nf_per_cm2(&[(parse_f64(&c["eps"]), parse_f64(&c["t"]))]);
        assert_close(got, parse_f64(&c["expected"]), 1e-12, 1e-12);
    }
    for c in g["stacks"].as_array().unwrap() {
        let layers: Vec<(f64, f64)> = c["layers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| (parse_f64(&p[0]), parse_f64(&p[1])))
            .collect();
        let got = calculate_stack_cox_nf_per_cm2(&layers);
        assert_close(got, parse_f64(&c["expected"]), 1e-12, 1e-12);
    }
}

#[test]
fn stack_cox_rejects_non_finite_layers_and_zero_underflow() {
    for layers in [
        vec![(f64::NAN, 10.0)],
        vec![(3.9, f64::NAN)],
        vec![(f64::INFINITY, 10.0)],
        vec![(3.9, f64::INFINITY)],
    ] {
        assert!(calculate_stack_cox_nf_per_cm2(&layers).is_nan());
    }
}
