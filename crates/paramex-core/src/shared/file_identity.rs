//! Generic file identity shared across product workflows.

use std::path::{Path, PathBuf};

/// Lowercase file stem with whitespace, underscores, and hyphens collapsed to `-`.
pub fn normalized_file_stem(name: &str) -> String {
    let stem = Path::new(name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(name);
    let mut normalized = String::new();
    for ch in stem.to_lowercase().chars() {
        if ch.is_whitespace() || ch == '_' || ch == '-' {
            if !normalized.is_empty() && !normalized.ends_with('-') {
                normalized.push('-');
            }
        } else {
            normalized.push(ch);
        }
    }
    while normalized.ends_with('-') {
        normalized.pop();
    }
    normalized
}

/// Best-effort path resolution: canonicalize an existing source, otherwise
/// retain its verbatim spelling.
fn resolve_source_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// True when two source paths resolve to the same best-effort canonical path.
pub fn same_source_path(left: &Path, right: &Path) -> bool {
    resolve_source_path(left) == resolve_source_path(right)
}

/// True when two named file sources identify the same measurement source.
///
/// Canonical paths are authoritative when both are available. If either side
/// has no path (for example, an in-memory parser input), exact display-name
/// equality is the stable fallback.
pub fn same_named_source(
    left_name: &str,
    left_path: Option<&Path>,
    right_name: &str,
    right_path: Option<&Path>,
) -> bool {
    match (left_path, right_path) {
        (Some(left), Some(right)) => same_source_path(left, right),
        _ => left_name == right_name,
    }
}

#[cfg(test)]
mod tests {
    use super::{normalized_file_stem, same_named_source, same_source_path};

    #[test]
    fn normalizes_common_separators() {
        assert_eq!(normalized_file_stem("Dev A_transfer.csv"), "dev-a-transfer");
        assert_eq!(normalized_file_stem("dev_A--output.xlsx"), "dev-a-output");
    }

    #[test]
    fn source_identity_canonicalizes_equivalent_existing_paths() {
        let crate_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        assert!(same_source_path(
            &crate_root.join("Cargo.toml"),
            &crate_root.join("src").join("..").join("Cargo.toml"),
        ));
    }

    #[test]
    fn named_source_identity_falls_back_only_when_a_path_is_missing() {
        let left = std::path::Path::new("lot-a/device_output.csv");
        let right = std::path::Path::new("lot-b/device_output.csv");

        assert!(!same_named_source(
            "device_output.csv",
            Some(left),
            "device_output.csv",
            Some(right),
        ));
        assert!(same_named_source(
            "device_output.csv",
            Some(left),
            "device_output.csv",
            None,
        ));
        assert!(!same_named_source(
            "device_output.csv",
            None,
            "other_output.csv",
            None,
        ));
    }
}
