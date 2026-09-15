use crate::common::{assert_close, f64_vec, load_reference_in, parse_fixture_dir};
use paramex_core::transfer::{parse_transfer_bytes, parse_transfer_file, ParseError, ParsedCurve};

// Coerced floats compare at rtol=1e-12, not exact: decimal parsers can diverge
// by one ULP. Structure stays exact.
const RTOL: f64 = 1e-12;
const ATOL: f64 = 0.0;

fn assert_curve(actual: &ParsedCurve, exp: &serde_json::Value, label: &str) {
    assert_eq!(actual.name, exp["name"].as_str().unwrap(), "{label}: name");
    let exp_vg = f64_vec(&exp["vg"]);
    assert_eq!(actual.vg.len(), exp_vg.len(), "{label}: vg len");
    for (&a, &e) in actual.vg.iter().zip(exp_vg.iter()) {
        assert_close(a, e, RTOL, ATOL);
    }
    let exp_id = f64_vec(&exp["id_abs"]);
    for (&a, &e) in actual.id_abs.iter().zip(exp_id.iter()) {
        assert_close(a, e, RTOL, ATOL);
    }
}

#[test]
fn csv_family_end_to_end_matches_reference() {
    let reference = load_reference_in("parse", "end_to_end");
    for case in reference["cases"].as_array().expect("cases") {
        let name = case["name"].as_str().unwrap();
        let result = &case["result"];

        // Build the Rust result from either the on-disk fixture (parse_transfer_file)
        // or the bytes path (unsupported-extension case has fixture=null).
        let rust: Result<ParsedCurve, ParseError> = if case["fixture"].is_null() {
            parse_transfer_bytes(name, b"{}")
        } else {
            let fixture = case["fixture"].as_str().unwrap();
            parse_transfer_file(&parse_fixture_dir().join(fixture))
        };

        if let Some(err) = result.get("error").and_then(|e| e.as_str()) {
            match rust {
                Err(e) => assert_eq!(e.0, err, "{name}: error message"),
                Ok(c) => panic!("{name}: expected error {err:?}, got curve {c:?}"),
            }
        } else if result["ok"].is_null() {
            assert!(rust.is_err(), "{name}: expected error (ok=null)");
        } else {
            let curve = rust.unwrap_or_else(|e| panic!("{name}: expected curve, got error {e:?}"));
            assert_curve(&curve, &result["ok"], name);
        }
    }
}
