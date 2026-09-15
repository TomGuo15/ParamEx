use crate::common::{assert_close, load_numpy_reference, parse_f64};
use paramex_core::shared::numpy_compat::unique_mean;

const RTOL: f64 = 1e-12;
const ATOL: f64 = 1e-12;

#[test]
fn unique_mean_matches_numpy() {
    let reference = load_numpy_reference("unique_mean");
    let cases = reference["cases"].as_array().expect("cases array");
    assert!(!cases.is_empty(), "reference has no cases");

    for (ci, case) in cases.iter().enumerate() {
        let x: Vec<f64> = case["x"]
            .as_array()
            .expect("x array")
            .iter()
            .map(parse_f64)
            .collect();
        let y: Vec<f64> = case["y"]
            .as_array()
            .expect("y array")
            .iter()
            .map(parse_f64)
            .collect();
        let exp_ux: Vec<f64> = case["unique_x"]
            .as_array()
            .expect("unique_x array")
            .iter()
            .map(parse_f64)
            .collect();
        let exp_my: Vec<f64> = case["mean_y"]
            .as_array()
            .expect("mean_y array")
            .iter()
            .map(parse_f64)
            .collect();

        let (ux, my) = unique_mean(&x, &y);

        // unique_x: identity values, never arithmetic -> compared exactly.
        assert_eq!(
            ux.len(),
            exp_ux.len(),
            "case {ci}: unique_x length mismatch"
        );
        for (k, (&a, &e)) in ux.iter().zip(exp_ux.iter()).enumerate() {
            assert!(
                a == e || (a.is_nan() && e.is_nan()),
                "case {ci}: unique_x[{k}] = {a:?}, expected {e:?}"
            );
        }

        // mean_y: arithmetic output -> tolerance compare.
        assert_eq!(my.len(), exp_my.len(), "case {ci}: mean_y length mismatch");
        for (&a, &e) in my.iter().zip(exp_my.iter()) {
            assert_close(a, e, RTOL, ATOL);
        }
    }
}
