use crate::common::{b64_decode, f64_vec, load_reference_in};
use paramex_core::transfer::{ParsedCurve, Session};

#[test]
fn session_export_bytes_match_reference() {
    let g = load_reference_in("session", "end_to_end");
    let mut session = Session::new();
    for c in g["curves"].as_array().unwrap() {
        let curve = ParsedCurve {
            name: c["name"].as_str().unwrap().to_string(),
            vg: f64_vec(&c["vg"]),
            id_abs: f64_vec(&c["id_abs"]),
            source_path: None,
        };
        assert!(
            session.add_curve(curve).is_some(),
            "corpus curves are distinct"
        );
    }
    let got = session.report_bytes();
    let expected = b64_decode(g["csv_b64"].as_str().unwrap());
    assert_eq!(
        got,
        expected,
        "end-to-end CSV bytes differ\n got:\n{}\n exp:\n{}",
        String::from_utf8_lossy(&got),
        String::from_utf8_lossy(&expected),
    );
}
