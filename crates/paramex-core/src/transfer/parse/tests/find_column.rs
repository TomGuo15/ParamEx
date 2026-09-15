use crate::shared::grid_headers::find_column_by_label;
use crate::transfer::parse::labels::{VD_LABELS, VG_LABELS};
use crate::transfer::test_support::load_reference_in;

fn vocab_of(name: &str) -> &'static [&'static str] {
    match name {
        "vg" => &VG_LABELS,
        "vd" => &VD_LABELS,
        other => panic!("unknown vocab {other:?}"),
    }
}

#[test]
fn find_column_by_label_matches_reference() {
    let reference = load_reference_in("parse", "find_column_by_label");
    let cases = reference["cases"].as_array().expect("cases array");
    assert!(!cases.is_empty(), "reference has no cases");

    for (i, case) in cases.iter().enumerate() {
        let values: Vec<String> = case["values"]
            .as_array()
            .expect("values")
            .iter()
            .map(|v| v.as_str().expect("string cell").to_string())
            .collect();
        let vocab = vocab_of(case["vocab"].as_str().expect("vocab"));
        let expected: Option<usize> = case["expected"].as_u64().map(|n| n as usize);

        assert_eq!(find_column_by_label(&values, vocab), expected, "case {i}");
    }
}
