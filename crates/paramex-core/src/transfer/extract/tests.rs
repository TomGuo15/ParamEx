use crate::transfer::metrics::vth::FitGate;
use crate::transfer::test_support::{f64_vec, load_reference_in, opt_win, parse_f64};
use crate::transfer::types::{ExtractionContext, SweepData, SweepExtractionResult};
use serde_json::Value;

use super::sweep::{extract_single_sweep, SWEEP_EXTRACT_MIN_POINTS};

fn assert_sweep(got: &SweepExtractionResult, exp: &Value, label: &str) {
    for (field, actual, expected) in [
        ("vt", got.vt, parse_f64(&exp["vt"])),
        ("mobility", got.mobility, parse_f64(&exp["mobility"])),
        ("ss_mv_dec", got.ss_mv_dec, parse_f64(&exp["ss_mv_dec"])),
        ("ion", got.ion, parse_f64(&exp["ion"])),
        ("ioff", got.ioff, parse_f64(&exp["ioff"])),
        (
            "on_off_ratio",
            got.on_off_ratio,
            parse_f64(&exp["on_off_ratio"]),
        ),
    ] {
        assert!(
            close(actual, expected),
            "{label}: {field} actual={actual} expected={expected}"
        );
    }
}

fn close(actual: f64, expected: f64) -> bool {
    if expected.is_nan() {
        return actual.is_nan();
    }
    if expected.is_infinite() {
        return actual == expected;
    }
    (actual - expected).abs() <= 1e-12 + 1e-9 * expected.abs()
}

#[test]
fn extract_pipeline_matches_reference() {
    let g = load_reference_in("extract", "extract_pipeline");
    for case in g["cases"].as_array().unwrap() {
        let label = case["label"].as_str().unwrap();
        let vg = f64_vec(&case["vg"]);
        let f_id = f64_vec(&case["f_id"]);
        let b_id = f64_vec(&case["b_id"]);
        let ctx = ExtractionContext {
            cox_f_per_cm2: parse_f64(&case["cox"]),
            aspect_ratio: parse_f64(&case["aspect"]),
        };
        let vt_range = opt_win(&case["vt_range"]);
        let ss_range = opt_win(&case["ss_range"]);
        let gate = |min_r2: f64| FitGate {
            min_points: SWEEP_EXTRACT_MIN_POINTS,
            min_r2,
        };
        let case_gate = gate(parse_f64(&case["min_r2"]));

        let fwd = SweepData {
            vg: vg.clone(),
            id_abs: f_id.clone(),
        };
        let bwd = SweepData {
            vg: vg.clone(),
            id_abs: b_id.clone(),
        };

        let single = extract_single_sweep(&fwd, ctx, vt_range, ss_range, case_gate);
        assert_sweep(&single, &case["single"], &format!("{label} single"));

        let forward = extract_single_sweep(&fwd, ctx, vt_range, ss_range, gate(0.98));
        let backward = extract_single_sweep(&bwd, ctx, vt_range, ss_range, gate(0.98));
        assert_sweep(&forward, &case["dual_forward"], &format!("{label} forward"));
        assert_sweep(
            &backward,
            &case["dual_backward"],
            &format!("{label} backward"),
        );
    }
}
