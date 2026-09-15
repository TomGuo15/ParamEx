use crate::transfer::metrics::hysteresis::extract_delta_vth_hysteresis_curve_shift;
use crate::transfer::split_double_sweep;
use crate::transfer::test_support::{assert_close, f64_vec, load_reference_in, parse_f64};
use crate::transfer::types::SweepData;

#[test]
fn hysteresis_matches_reference() {
    let g = load_reference_in("metrics", "hysteresis");
    for case in g["cases"].as_array().unwrap() {
        let exp = &case["result"];
        let result = match case["kind"].as_str().unwrap() {
            "shift" => {
                let f = SweepData {
                    vg: f64_vec(&case["f_vg"]),
                    id_abs: f64_vec(&case["f_id"]),
                };
                let b = SweepData {
                    vg: f64_vec(&case["b_vg"]),
                    id_abs: f64_vec(&case["b_id"]),
                };
                // Extraction defaults: trim_fraction=0.2, min_points=12.
                extract_delta_vth_hysteresis_curve_shift(&f, &b, 0.2, 12)
            }
            "auto" => {
                let vg = f64_vec(&case["vg"]);
                let id_abs = f64_vec(&case["id_abs"]);
                let (forward, backward) = split_double_sweep(&vg, &id_abs);
                extract_delta_vth_hysteresis_curve_shift(&forward, &backward, 0.2, 12)
            }
            other => panic!("unknown kind {other}"),
        };
        assert_close(result, parse_f64(&exp["delta_vt"]), 1e-9, 1e-12);
    }
}
