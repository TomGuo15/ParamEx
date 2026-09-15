use crate::common::{assert_close, load_reference_in, parse_f64};
use paramex_core::shared::numerics::linear_fit_with_r2;

// polyfit-path tolerance: closed-form OLS vs numpy SVD polyfit on well-conditioned data.
const RTOL: f64 = 1e-9;
const ATOL: f64 = 1e-12;

#[test]
fn linear_fit_matches_reference_polyfit() {
    let reference = load_reference_in("numerics", "linear_fit");
    let cases = reference["cases"].as_array().expect("cases array");
    assert!(!cases.is_empty(), "reference has no cases");

    for (i, case) in cases.iter().enumerate() {
        let x: Vec<f64> = case["x"]
            .as_array()
            .expect("x")
            .iter()
            .map(parse_f64)
            .collect();
        let y: Vec<f64> = case["y"]
            .as_array()
            .expect("y")
            .iter()
            .map(parse_f64)
            .collect();
        let exp_slope = parse_f64(&case["slope"]);
        let exp_intercept = parse_f64(&case["intercept"]);
        let exp_r2 = parse_f64(&case["r2"]);
        let exp_points = case["points"].as_u64().expect("points") as usize;

        let (slope, intercept, r2, points) = linear_fit_with_r2(&x, &y);

        assert_eq!(points, exp_points, "case {i}: points");
        assert_close(slope, exp_slope, RTOL, ATOL);
        assert_close(intercept, exp_intercept, RTOL, ATOL);
        assert_close(r2, exp_r2, RTOL, ATOL);
    }
}
