// Shared integration-test helpers. Each test binary compiles this module
// independently and uses only a subset, so unused-helper warnings are expected.
#![allow(dead_code)]

use serde_json::Value;
use std::path::{Path, PathBuf};

pub fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

pub fn crate_file(path: impl AsRef<Path>) -> PathBuf {
    crate_root().join(path)
}

pub fn read_crate_file(path: impl AsRef<Path>) -> String {
    read(&crate_file(path))
}

pub fn reference_dir(category: impl AsRef<Path>) -> PathBuf {
    crate_file(Path::new("tests/reference").join(category))
}

pub fn reference_file(category: impl AsRef<Path>, name: &str) -> PathBuf {
    reference_dir(category).join(format!("{name}.json"))
}

pub fn fixture_dir(category: impl AsRef<Path>) -> PathBuf {
    crate_file(Path::new("tests/fixtures").join(category))
}

pub fn tlm_reference_dir() -> PathBuf {
    reference_dir("tlm")
}

pub fn tlm_corpus_dir() -> PathBuf {
    tlm_reference_dir().join("corpus")
}

pub fn tlm_fixture_dir() -> PathBuf {
    fixture_dir("tlm")
}

pub fn parse_fixture_dir() -> PathBuf {
    fixture_dir("parse")
}

/// Recursively collect every file under `dir` whose extension equals `ext`.
pub fn collect_files_with_ext(dir: &Path, ext: &str, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
        let path = entry.expect("read entry").path();
        if path.is_dir() {
            collect_files_with_ext(&path, ext, files);
        } else if path.extension().is_some_and(|e| e == ext) {
            files.push(path);
        }
    }
}

/// Load `tests/reference/<category>/<name>.json` as a serde_json Value.
pub fn load_reference_in(category: &str, name: &str) -> Value {
    let path = reference_file(category, name);
    let text = read(&path);
    serde_json::from_str(&text).expect("parse reference json")
}

/// Load `tests/reference/numpy_compat/<name>.json`.
pub fn load_numpy_reference(name: &str) -> Value {
    load_reference_in("numpy_compat", name)
}

/// Parse a reference float: a JSON number, or the tagged strings "nan"/"inf"/"-inf".
pub fn parse_f64(v: &Value) -> f64 {
    if let Some(n) = v.as_f64() {
        return n;
    }
    match v.as_str() {
        Some("nan") => f64::NAN,
        Some("inf") => f64::INFINITY,
        Some("-inf") => f64::NEG_INFINITY,
        other => panic!("bad float encoding in reference data: {other:?}"),
    }
}

pub fn f64_vec(value: &Value) -> Vec<f64> {
    value.as_array().unwrap().iter().map(parse_f64).collect()
}

/// Assert `actual` matches `expected` within tolerance. NaN- and inf-aware:
/// NaN matches only NaN; ±inf matches only the same ±inf; otherwise the test is
/// `|actual - expected| <= atol + rtol * |expected|`.
pub fn assert_close(actual: f64, expected: f64, rtol: f64, atol: f64) {
    if expected.is_nan() {
        assert!(actual.is_nan(), "expected NaN, got {actual}");
        return;
    }
    if expected.is_infinite() {
        assert!(actual == expected, "expected {expected}, got {actual}");
        return;
    }
    let diff = (actual - expected).abs();
    assert!(
        diff <= atol + rtol * expected.abs(),
        "not close: actual={actual} expected={expected} diff={diff} (rtol={rtol}, atol={atol})"
    );
}

/// Decode an `Option<(f64, f64)>` window from reference JSON (null or `[lo, hi]`).
pub fn opt_win(v: &Value) -> Option<(f64, f64)> {
    if v.is_null() {
        None
    } else {
        let a = v.as_array().unwrap();
        Some((parse_f64(&a[0]), parse_f64(&a[1])))
    }
}

/// Minimal standard-alphabet base64 decoder (no external dep). Ignores `=`
/// padding and any whitespace. Used to compare reference CSV bytes exactly.
pub fn b64_decode(s: &str) -> Vec<u8> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut rev = [255u8; 256];
    let mut i = 0;
    while i < 64 {
        rev[ALPHABET[i] as usize] = i as u8;
        i += 1;
    }
    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0u32;
    for &b in s.as_bytes() {
        let v = rev[b as usize];
        if v == 255 {
            continue; // skip '=', newlines, anything non-alphabet
        }
        buf = (buf << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    out
}
