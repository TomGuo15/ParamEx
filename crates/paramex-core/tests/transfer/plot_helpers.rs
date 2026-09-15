use crate::common::{assert_close, f64_vec, load_reference_in, opt_win, parse_f64};
use paramex_core::transfer::{
    axis_bounds, clamp_window_to_axis, log_current_axis_range, sqrt_current_axis_range,
};
use serde_json::Value;

fn pair(v: &Value) -> (f64, f64) {
    let a = v.as_array().unwrap();
    (parse_f64(&a[0]), parse_f64(&a[1]))
}
fn assert_pair_close(got: (f64, f64), exp: &Value) {
    let a = exp.as_array().unwrap();
    assert_close(got.0, parse_f64(&a[0]), 1e-12, 1e-12);
    assert_close(got.1, parse_f64(&a[1]), 1e-12, 1e-12);
}
fn assert_arr2_close(got: [f64; 2], exp: &Value) {
    let a = exp.as_array().unwrap();
    assert_close(got[0], parse_f64(&a[0]), 1e-12, 1e-12);
    assert_close(got[1], parse_f64(&a[1]), 1e-12, 1e-12);
}

#[test]
fn plot_helpers_match_reference() {
    let g = load_reference_in("plot", "plot_helpers");

    for case in g["arrays"].as_array().unwrap() {
        let values = f64_vec(&case["values"]);
        assert_pair_close(axis_bounds(&values), &case["axis_bounds"]);
        assert_arr2_close(log_current_axis_range(&values), &case["log_range"]);
        assert_arr2_close(sqrt_current_axis_range(&values), &case["sqrt_range"]);
    }

    for case in g["clamps"].as_array().unwrap() {
        let window = opt_win(&case["window"]);
        let axis = pair(&case["axis"]);
        assert_eq!(
            clamp_window_to_axis(window, axis),
            opt_win(&case["clamped"])
        );
    }
}
