//! Guards: the committed TLM data is synthetic (no real labels/IDs) and still
//! exercises the warning / error-status / multi-device code paths.

use crate::common::{collect_files_with_ext, read, tlm_corpus_dir, tlm_reference_dir};

/// Substrings/patterns that must never appear in committed TLM data.
fn contains_real_token(s: &str) -> bool {
    if s.contains("blade") || s.contains("spin_coating") {
        return true;
    }
    // Real device IDs take the form digit_digit_digit.
    let bytes = s.as_bytes();
    bytes.windows(5).any(|w| {
        w[0].is_ascii_digit()
            && w[1] == b'_'
            && w[2].is_ascii_digit()
            && w[3] == b'_'
            && w[4].is_ascii_digit()
    })
}

fn oracle(name: &str) -> String {
    read(&tlm_reference_dir().join("oracle").join(name))
}

/// Column index of `header` in the first CSV line (BOM-tolerant).
fn column_index(csv: &str, header: &str) -> usize {
    csv.lines()
        .next()
        .expect("header row")
        .trim_start_matches('\u{feff}')
        .split(',')
        .position(|h| h == header)
        .unwrap_or_else(|| panic!("missing column {header}"))
}

#[test]
fn corpus_and_oracle_contain_no_real_labels() {
    let mut workbooks = Vec::new();
    collect_files_with_ext(&tlm_corpus_dir(), "xlsx", &mut workbooks);
    assert!(!workbooks.is_empty(), "corpus is empty");
    for entry in workbooks {
        let name = entry.to_string_lossy().to_string();
        assert!(!contains_real_token(&name), "real token in path: {name}");
    }
    for f in ["result.csv", "sweep.csv", "length_points.csv", "status.csv"] {
        assert!(!contains_real_token(&oracle(f)), "real token in oracle/{f}");
    }
}

#[test]
fn oracle_shape_and_behavior_preserved() {
    let result = oracle("result.csv");
    let status = oracle("status.csv");
    let points = oracle("length_points.csv");

    // 4 process groups, all present.
    for g in ["process_a", "process_b", "process_c", "process_d"] {
        assert!(result.contains(g), "missing group {g} in result.csv");
    }
    // Warning split: at least one poor-fit warning AND at least one clean group.
    assert!(result.contains("Poor TLM fit"), "no poor-fit warning row");
    // A clean group row ends with the trailing empty warnings column.
    assert!(
        result
            .lines()
            .any(|l| l.starts_with("process_b") && l.trim_end().ends_with(',')),
        "no clean (no-warning) group row"
    );
    // Error-status path preserved (exactly one error row, vd_source unread).
    let errors = status.lines().filter(|l| l.contains(",error,")).count();
    assert_eq!(errors, 1, "expected exactly one error status row");
    assert!(
        status.contains("unread"),
        "error row should be vd_source=unread"
    );
    // Multi-device aggregation preserved: device_count is 2 on every point.
    let device_count = column_index(&points, "device_count");
    assert!(
        points
            .lines()
            .skip(1)
            .all(|l| l.split(',').nth(device_count) == Some("2")),
        "expected device_count=2 on every length point"
    );
}
