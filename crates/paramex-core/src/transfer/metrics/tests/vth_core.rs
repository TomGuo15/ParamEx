use crate::transfer::metrics::vth::{extract_vt_mu, extract_vth_elr, FitGate, VtFitResult};
use crate::transfer::test_support::{assert_close, f64_vec, load_reference_in, opt_win, parse_f64};
use crate::transfer::types::ExtractionContext;
use serde_json::Value;

fn assert_fit(got: &VtFitResult, exp: &Value, label: &str) {
    assert_close(got.vt, parse_f64(&exp["vt"]), 1e-9, 1e-12);
    assert_close(got.mobility, parse_f64(&exp["mobility"]), 1e-9, 1e-12);
    assert_close(got.slope, parse_f64(&exp["slope"]), 1e-9, 1e-12);
    assert_close(got.intercept, parse_f64(&exp["intercept"]), 1e-9, 1e-12);
    assert_close(got.r2, parse_f64(&exp["r2"]), 1e-9, 1e-12);
    assert_eq!(
        got.points,
        exp["points"].as_u64().unwrap() as usize,
        "points for {label}"
    );
}

#[test]
fn vth_core_matches_reference() {
    let g = load_reference_in("metrics", "vth_core");
    for case in g["cases"].as_array().unwrap() {
        let label = case["label"].as_str().unwrap();
        let vg = f64_vec(&case["vg"]);
        let id_abs = f64_vec(&case["id_abs"]);
        let fr = opt_win(&case["fit_range"]);
        let gate = FitGate {
            min_points: case["min_points"].as_u64().unwrap() as usize,
            min_r2: parse_f64(&case["min_r2"]),
        };
        let ctx = ExtractionContext {
            cox_f_per_cm2: parse_f64(&case["cox"]),
            aspect_ratio: parse_f64(&case["aspect"]),
        };
        let elr = extract_vth_elr(&vg, &id_abs, fr, gate);
        assert_fit(&elr, &case["elr"], label);
        let mu = extract_vt_mu(&vg, &id_abs, ctx, fr, gate);
        assert_fit(&mu, &case["vt_mu"], label);
    }
}
