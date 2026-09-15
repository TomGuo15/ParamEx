use serde_json::Value;
use std::path::{Path, PathBuf};

use crate::shared::grid_ingest::Grid;

use super::{ExtractionStatus, GeometrySource, MetricResult};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn reference_file(category: impl AsRef<Path>, name: &str) -> PathBuf {
    crate_root()
        .join("tests/reference")
        .join(category)
        .join(format!("{name}.json"))
}

pub(super) fn load_reference_in(category: &str, name: &str) -> Value {
    let path = reference_file(category, name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    serde_json::from_str(&text).expect("parse reference json")
}

pub(super) fn parse_f64(value: &Value) -> f64 {
    if let Some(number) = value.as_f64() {
        return number;
    }
    match value.as_str() {
        Some("nan") => f64::NAN,
        Some("inf") => f64::INFINITY,
        Some("-inf") => f64::NEG_INFINITY,
        other => panic!("bad float encoding in reference data: {other:?}"),
    }
}

pub(super) fn f64_vec(value: &Value) -> Vec<f64> {
    value
        .as_array()
        .expect("float array")
        .iter()
        .map(parse_f64)
        .collect()
}

pub(super) fn grid_from(value: &Value) -> Grid {
    value
        .as_array()
        .expect("grid rows")
        .iter()
        .map(|row| {
            row.as_array()
                .expect("row cells")
                .iter()
                .map(|cell| cell.as_str().expect("string cell").to_string())
                .collect()
        })
        .collect()
}

pub(super) fn assert_close(actual: f64, expected: f64, rtol: f64, atol: f64) {
    if expected.is_nan() {
        assert!(actual.is_nan(), "expected NaN, got {actual}");
        return;
    }
    if expected.is_infinite() {
        assert_eq!(actual, expected, "expected {expected}, got {actual}");
        return;
    }
    let diff = (actual - expected).abs();
    assert!(
        diff <= atol + rtol * expected.abs(),
        "not close: actual={actual} expected={expected} diff={diff} (rtol={rtol}, atol={atol})"
    );
}

pub(super) fn opt_win(value: &Value) -> Option<(f64, f64)> {
    if value.is_null() {
        None
    } else {
        let values = value.as_array().expect("window array");
        Some((parse_f64(&values[0]), parse_f64(&values[1])))
    }
}

pub(super) fn assert_win(got: Option<(f64, f64)>, expected: &Value, label: &str) {
    if expected.is_null() {
        assert!(got.is_none(), "{label}: expected None, got {got:?}");
    } else {
        let (lo, hi) = got.unwrap_or_else(|| panic!("{label}: expected Some, got None"));
        let values = expected.as_array().unwrap();
        assert_eq!(lo, parse_f64(&values[0]), "{label}: lo exact");
        assert_eq!(hi, parse_f64(&values[1]), "{label}: hi exact");
    }
}

pub(super) fn metric_result(value: &Value) -> MetricResult {
    let float = |key: &str| parse_f64(&value[key]);
    let string = |key: &str| value[key].as_str().expect("string field").to_string();
    MetricResult {
        filename: string("filename"),
        width_um: float("width_um"),
        length_um: float("length_um"),
        aspect_ratio: float("aspect_ratio"),
        geometry_source: GeometrySource::parse(&string("geometry_source")),
        vt: float("vt"),
        mu_sat: float("mu_sat"),
        ss_mv_dec: float("ss_mv_dec"),
        ion: float("ion"),
        ioff: float("ioff"),
        on_off_ratio: float("on_off_ratio"),
        delta_vth_hysteresis: float("delta_vth_hysteresis"),
        vt_window: opt_win(&value["vt_window"]),
        ss_window: opt_win(&value["ss_window"]),
        vt_window_bwd: opt_win(&value["vt_window_bwd"]),
        ss_window_bwd: opt_win(&value["ss_window_bwd"]),
        status: ExtractionStatus::parse(&string("status")),
        message: string("message"),
        has_backward_sweep: value["has_backward_sweep"]
            .as_bool()
            .expect("boolean field"),
        vt_forward: float("vt_forward"),
        mu_sat_forward: float("mu_sat_forward"),
        ss_mv_dec_forward: float("ss_mv_dec_forward"),
        ion_forward: float("ion_forward"),
        ioff_forward: float("ioff_forward"),
        on_off_ratio_forward: float("on_off_ratio_forward"),
        vt_backward: float("vt_backward"),
        mu_sat_backward: float("mu_sat_backward"),
        ss_mv_dec_backward: float("ss_mv_dec_backward"),
        ion_backward: float("ion_backward"),
        ioff_backward: float("ioff_backward"),
        on_off_ratio_backward: float("on_off_ratio_backward"),
    }
}

pub(super) fn b64_decode(encoded: &str) -> Vec<u8> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut reverse = [255_u8; 256];
    for (index, byte) in ALPHABET.iter().copied().enumerate() {
        reverse[byte as usize] = index as u8;
    }

    let mut output = Vec::new();
    let mut buffer = 0_u32;
    let mut bits = 0_u32;
    for &byte in encoded.as_bytes() {
        let value = reverse[byte as usize];
        if value == 255 {
            continue;
        }
        buffer = (buffer << 6) | value as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((buffer >> bits) as u8);
        }
    }
    output
}
