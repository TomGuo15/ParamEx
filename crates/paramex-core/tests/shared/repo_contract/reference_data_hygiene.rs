//! Guard: committed reference data stays free of absolute or private
//! filesystem paths. Every text file under `tests/reference` (`*.json`,
//! `*.csv`, `*.tsv`, `*.txt`, including the regenerable TLM oracle CSVs) is
//! scanned; any recorded path must be `null` or a repo-relative name. `.rs`
//! and binary `.xlsx` files are excluded: test source legitimately contains
//! path literals, and xlsx is not text.

use crate::common::{collect_files_with_ext, crate_file, read};

/// True if `line` embeds an absolute/private filesystem root: a Windows
/// drive-letter root (`X:\`, the JSON-escaped `X:\\`, or `X:/`) or a POSIX
/// home root (`/Users/`, `/home/`).
///
/// The drive-letter check requires a *standalone* single letter (the char
/// before it is not a letter) so URL schemes such as `https://` are not
/// mistaken for an `s:/` drive.
fn line_has_absolute_path(line: &str) -> bool {
    if line.contains("/Users/") || line.contains("/home/") {
        return true;
    }
    let bytes = line.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if !b.is_ascii_alphabetic() {
            continue;
        }
        // Reject letters that are part of a longer word (e.g. the `s` in `https`).
        if i > 0 && bytes[i - 1].is_ascii_alphabetic() {
            continue;
        }
        if bytes.get(i + 1) != Some(&b':') {
            continue;
        }
        if matches!(bytes.get(i + 2), Some(b'\\' | b'/')) {
            return true;
        }
    }
    false
}

#[test]
fn reference_data_embeds_no_absolute_filesystem_paths() {
    let root = crate_file("tests/reference");
    let mut files = Vec::new();
    for ext in ["json", "csv", "tsv", "txt"] {
        collect_files_with_ext(&root, ext, &mut files);
    }
    files.sort();
    assert!(
        !files.is_empty(),
        "expected reference data under {}",
        root.display()
    );

    let mut hits = Vec::new();
    for file in &files {
        let text = read(file);
        for (n, line) in text.lines().enumerate() {
            if line_has_absolute_path(line) {
                hits.push(format!("{}:{}: {}", file.display(), n + 1, line.trim()));
            }
        }
    }

    assert!(
        hits.is_empty(),
        "reference data must not embed absolute/private filesystem paths \
         (scrub to null or a repo-relative name):\n{}",
        hits.join("\n")
    );
}

#[test]
fn absolute_path_detector_catches_private_roots_and_ignores_clean_text() {
    // Must flag the leak class this guard exists to prevent (raw strings keep
    // backslashes literal, matching the JSON-escaped `\\` on disk).
    assert!(line_has_absolute_path(
        r#"        "source_path": "Q:\\data\\x.csv","#
    ));
    assert!(line_has_absolute_path(r#"  "p": "C:/Users/dev/x.csv""#));
    assert!(line_has_absolute_path(r#"  "p": "/Users/dev/x.csv""#));
    assert!(line_has_absolute_path(r#"  "p": "/home/dev/x.csv""#));
    assert!(line_has_absolute_path(r"E:\data\run"));

    // Must NOT flag clean reference content or non-path strings.
    assert!(!line_has_absolute_path(
        r#"        "name": "corpus_single_a.csv","#
    ));
    assert!(!line_has_absolute_path(r#"        "source_path": null,"#));
    assert!(!line_has_absolute_path(
        r#"  "url": "https://github.com/TomGuo15/ParamEx""#
    ));
    assert!(!line_has_absolute_path(r#"  "label": "Id-Vg-IGZO","#));
    assert!(!line_has_absolute_path(r#"  "ts": "12:30","#));
    assert!(!line_has_absolute_path(r#"  "vg": [-2.0, -1.98]"#));
}
