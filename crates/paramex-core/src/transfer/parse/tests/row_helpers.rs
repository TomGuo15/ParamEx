use crate::shared::grid_headers::find_label_indices;
use crate::transfer::parse::labels::{ID_LABELS, VG_LABELS};
use crate::transfer::parse::sheet::normalized_row;
use crate::transfer::test_support::{grid_from, load_reference_in};

fn usize_vec(value: &serde_json::Value) -> Vec<usize> {
    value
        .as_array()
        .expect("idx array")
        .iter()
        .map(|v| v.as_u64().unwrap() as usize)
        .collect()
}

#[test]
fn row_helpers_match_reference() {
    let reference = load_reference_in("parse", "row_helpers");
    let cases = reference["cases"].as_array().expect("cases array");
    assert!(!cases.is_empty(), "reference has no cases");

    for (i, case) in cases.iter().enumerate() {
        let grid = grid_from(&case["grid"]);
        let row_idx = case["row_idx"].as_u64().unwrap() as usize;
        let exp_norm: Vec<String> = case["normalized"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c.as_str().unwrap().to_string())
            .collect();

        let norm = normalized_row(&grid, row_idx);
        assert_eq!(norm, exp_norm, "case {i}: normalized");
        assert_eq!(
            find_label_indices(&norm, &VG_LABELS),
            usize_vec(&case["vg_indices"]),
            "case {i}: vg"
        );
        assert_eq!(
            find_label_indices(&norm, &ID_LABELS),
            usize_vec(&case["id_indices"]),
            "case {i}: id"
        );
    }
}
