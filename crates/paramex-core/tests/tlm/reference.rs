//! End-to-end parity: TLM output vs the committed CSV oracle over the synthetic,
//! generated corpus. Structure + non-float cells exact; float cells by value
//! (rtol=1e-9).

use std::path::Path;

use crate::common::{tlm_corpus_dir, tlm_reference_dir};
use paramex_core::tlm::{
    analyze_dataset, analyze_sweep, length_points_csv, load_dataset, result_csv, status_csv,
    sweep_csv,
};

/// Columns that hold floats (compared by rtol); everything else compares exactly.
fn float_columns() -> std::collections::HashSet<&'static str> {
    [
        "selected_vg",
        "actual_vg",
        "Rcontact_script_ohm",
        "Rc_per_contact_ohm",
        "slope_ohm_per_um",
        "r_squared",
        "Rcontact_median_ohm",
        "Rc_per_contact_median_ohm",
        "slope_median_ohm_per_um",
        "r_squared_median",
        "current_a",
        "Rtotal_ohm",
        "current_median_a",
        "Rtotal_median_ohm",
        // length_um is integer-valued here but compare as float to be safe:
        "length_um",
    ]
    .into_iter()
    .collect()
}

fn parse_csv(bytes: &[u8]) -> (Vec<String>, Vec<Vec<String>>) {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(bytes);
    let mut records: Vec<Vec<String>> = rdr
        .records()
        .map(|r| r.unwrap().iter().map(|s| s.to_string()).collect())
        .collect();
    let headers = records.remove(0);
    (headers, records)
}

fn assert_csv_parity(actual: &[u8], oracle_path: &Path) {
    let oracle = std::fs::read(oracle_path).expect("oracle csv present");
    let (ah, ar) = parse_csv(actual);
    let (oh, or) = parse_csv(&oracle);
    assert_eq!(ah, oh, "headers differ for {}", oracle_path.display());
    assert_eq!(
        ar.len(),
        or.len(),
        "row count differs for {}",
        oracle_path.display()
    );
    let floats = float_columns();
    for (ri, (arow, orow)) in ar.iter().zip(&or).enumerate() {
        assert_eq!(arow.len(), orow.len(), "row {ri} width differs");
        for (ci, col) in ah.iter().enumerate() {
            let (a, o) = (&arow[ci], &orow[ci]);
            if floats.contains(col.as_str()) {
                cmp_float(a, o, col, ri);
            } else {
                assert_eq!(a, o, "col '{col}' row {ri} differs");
            }
        }
    }
}

fn cmp_float(a: &str, o: &str, col: &str, ri: usize) {
    let (ea, eo) = (a.trim().is_empty(), o.trim().is_empty());
    assert_eq!(
        ea, eo,
        "col '{col}' row {ri}: NaN/empty mismatch ('{a}' vs '{o}')"
    );
    if ea {
        return;
    }
    let (fa, fo): (f64, f64) = (a.parse().unwrap(), o.parse().unwrap());
    let denom = fo.abs().max(1.0);
    assert!(
        (fa - fo).abs() / denom <= 1e-9,
        "col '{col}' row {ri}: {fa} vs {fo} exceeds rtol"
    );
}

#[test]
fn tlm_corpus_matches_committed_oracle() {
    let oracle = tlm_reference_dir().join("oracle");
    let ds = load_dataset(&tlm_corpus_dir(), None).expect("loads corpus");
    let res = analyze_dataset(&ds, None);
    let swp = analyze_sweep(&ds);

    assert_csv_parity(&result_csv(&res), &oracle.join("result.csv"));
    assert_csv_parity(&sweep_csv(&swp), &oracle.join("sweep.csv"));
    assert_csv_parity(&length_points_csv(&res), &oracle.join("length_points.csv"));
    assert_csv_parity(&status_csv(&res), &oracle.join("status.csv"));
}
