use crate::common::{assert_close, f64_vec, load_reference_in, opt_win, parse_f64};
use paramex_core::transfer::{
    extract_metrics, DeviceGeometry, ExpertRanges, ExtractionSettings, GeometrySource,
    MetricResult, ParsedCurve,
};
use serde_json::Value;

fn assert_result(got: &MetricResult, exp: &Value, label: &str) {
    assert_eq!(
        got.filename,
        exp["filename"].as_str().unwrap(),
        "filename {label}"
    );
    assert_eq!(
        got.geometry_source.as_str(),
        exp["geometry_source"].as_str().unwrap(),
        "geom_source {label}"
    );
    assert_eq!(
        got.status.as_str(),
        exp["status"].as_str().unwrap(),
        "status {label}"
    );
    assert_eq!(
        got.message,
        exp["message"].as_str().unwrap(),
        "message {label}"
    );
    assert_eq!(
        got.has_backward_sweep,
        exp["has_backward_sweep"].as_bool().unwrap(),
        "has_bwd {label}"
    );
    assert_eq!(
        got.vt_window,
        opt_win(&exp["vt_window"]),
        "vt_window {label}"
    );
    assert_eq!(
        got.ss_window,
        opt_win(&exp["ss_window"]),
        "ss_window {label}"
    );
    assert_eq!(
        got.vt_window_bwd,
        opt_win(&exp["vt_window_bwd"]),
        "vt_window_bwd {label}"
    );
    assert_eq!(
        got.ss_window_bwd,
        opt_win(&exp["ss_window_bwd"]),
        "ss_window_bwd {label}"
    );
    let f = |k: &str| parse_f64(&exp[k]);
    for (g, k) in [
        (got.width_um, "width_um"),
        (got.length_um, "length_um"),
        (got.aspect_ratio, "aspect_ratio"),
        (got.vt, "vt"),
        (got.mu_sat, "mu_sat"),
        (got.ss_mv_dec, "ss_mv_dec"),
        (got.ion, "ion"),
        (got.ioff, "ioff"),
        (got.on_off_ratio, "on_off_ratio"),
        (got.delta_vth_hysteresis, "delta_vth_hysteresis"),
        (got.vt_forward, "vt_forward"),
        (got.mu_sat_forward, "mu_sat_forward"),
        (got.ss_mv_dec_forward, "ss_mv_dec_forward"),
        (got.ion_forward, "ion_forward"),
        (got.ioff_forward, "ioff_forward"),
        (got.on_off_ratio_forward, "on_off_ratio_forward"),
        (got.vt_backward, "vt_backward"),
        (got.mu_sat_backward, "mu_sat_backward"),
        (got.ss_mv_dec_backward, "ss_mv_dec_backward"),
        (got.ion_backward, "ion_backward"),
        (got.ioff_backward, "ioff_backward"),
        (got.on_off_ratio_backward, "on_off_ratio_backward"),
    ] {
        assert_close(g, f(k), 1e-9, 1e-12);
    }
}

#[test]
fn extract_metrics_matches_reference() {
    let g = load_reference_in("extract", "metrics");
    for case in g["cases"].as_array().unwrap() {
        let label = case["label"].as_str().unwrap();
        let curve = ParsedCurve {
            name: case["name"].as_str().unwrap().to_string(),
            vg: f64_vec(&case["vg"]),
            id_abs: f64_vec(&case["id_abs"]),
            source_path: None,
        };
        let gj = &case["geometry"];
        let geometry = DeviceGeometry {
            width_um: parse_f64(&gj["width_um"]),
            length_um: parse_f64(&gj["length_um"]),
            source: GeometrySource::parse(gj["source"].as_str().unwrap()),
        };
        let sj = &case["settings"];
        let settings = ExtractionSettings {
            cox_nf_per_cm2: parse_f64(&sj["cox_nf_per_cm2"]),
        };
        let ej = &case["expert_ranges"];
        let er = ExpertRanges {
            vt_range: opt_win(&ej["vt_range"]),
            ss_range: opt_win(&ej["ss_range"]),
            vt_range_bwd: opt_win(&ej["vt_range_bwd"]),
            ss_range_bwd: opt_win(&ej["ss_range_bwd"]),
        };
        let got = extract_metrics(&curve, &settings, &er, &geometry);
        assert_result(&got, &case["result"], label);
    }
}
