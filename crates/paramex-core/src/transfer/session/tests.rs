use crate::transfer::session::file_set::{curve_fingerprint, curves_match};
use crate::transfer::test_support::{f64_vec, load_reference_in};
use crate::transfer::types::ParsedCurve;
use serde_json::Value;

fn build(spec: &Value) -> ParsedCurve {
    ParsedCurve {
        name: spec["name"].as_str().unwrap().to_string(),
        vg: f64_vec(&spec["vg"]),
        id_abs: f64_vec(&spec["id_abs"]),
        source_path: None,
    }
}

#[test]
fn fingerprints_match_reference() {
    let g = load_reference_in("session", "fingerprint");
    for spec in g["fingerprints"].as_array().unwrap() {
        let curve = build(spec);
        let (name_fold, hex) = curve_fingerprint(&curve);
        assert_eq!(name_fold, spec["name_fold"].as_str().unwrap(), "name_fold");
        assert_eq!(hex, spec["hex"].as_str().unwrap(), "hex for {}", curve.name);
    }
}

#[test]
fn curves_match_matrix_matches_reference() {
    let g = load_reference_in("session", "fingerprint");
    let curves: Vec<ParsedCurve> = g["fingerprints"]
        .as_array()
        .unwrap()
        .iter()
        .map(build)
        .collect();
    for m in g["matches"].as_array().unwrap() {
        let i = m["i"].as_u64().unwrap() as usize;
        let j = m["j"].as_u64().unwrap() as usize;
        assert_eq!(
            curves_match(&curves[i], &curves[j]),
            m["match"].as_bool().unwrap(),
            "match {i},{j}"
        );
    }
}
