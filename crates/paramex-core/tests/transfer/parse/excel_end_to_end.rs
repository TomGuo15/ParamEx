use crate::common::{assert_close, f64_vec, load_reference_in, parse_fixture_dir};
use paramex_core::transfer::{parse_transfer_file, ParseError, ParsedCurve};

// Coerced floats compare at rtol=1e-12 because decimal parsers can differ by one ULP.
const RTOL: f64 = 1e-12;
const ATOL: f64 = 0.0;

#[test]
fn excel_end_to_end_matches_reference() {
    let reference = load_reference_in("parse", "excel_end_to_end");
    for case in reference["cases"].as_array().expect("cases") {
        let fixture = case["fixture"].as_str().unwrap();
        let result = &case["result"];
        let rust: Result<ParsedCurve, ParseError> =
            parse_transfer_file(&parse_fixture_dir().join(fixture));

        if let Some(err) = result.get("error").and_then(|e| e.as_str()) {
            match rust {
                Err(e) => assert_eq!(e.0, err, "{fixture}: error message"),
                Ok(c) => panic!("{fixture}: expected error {err:?}, got curve {c:?}"),
            }
        } else {
            let exp = &result["ok"];
            let curve =
                rust.unwrap_or_else(|e| panic!("{fixture}: expected curve, got error {e:?}"));
            assert_eq!(curve.name, exp["name"].as_str().unwrap(), "{fixture}: name");
            let exp_vg = f64_vec(&exp["vg"]);
            assert_eq!(curve.vg.len(), exp_vg.len(), "{fixture}: vg len");
            for (&a, &e) in curve.vg.iter().zip(exp_vg.iter()) {
                assert_close(a, e, RTOL, ATOL);
            }
            let exp_id = f64_vec(&exp["id_abs"]);
            for (&a, &e) in curve.id_abs.iter().zip(exp_id.iter()) {
                assert_close(a, e, RTOL, ATOL);
            }
        }
    }
}
