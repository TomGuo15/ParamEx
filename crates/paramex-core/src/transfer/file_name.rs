//! Filename conventions used by the Transfer output-association workflow.

use crate::shared::normalized_file_stem;

const OUTPUT_SUFFIXES: &[&str] = &["-output-curve", "-id-vd", "-output"];

/// True when a filename alone says this is an output/Id-Vd measurement.
pub fn output_name_hint(name: &str) -> bool {
    let key = normalized_file_stem(name);
    key.contains("id-vd")
        || OUTPUT_SUFFIXES.iter().any(|suffix| key.ends_with(suffix))
        || key
            .strip_suffix('o')
            .and_then(|base| base.chars().last())
            .is_some_and(|ch| ch.is_ascii_digit())
}

/// Base key used to attach an output measurement to a Transfer file.
pub(super) fn output_match_key(name: &str) -> String {
    let key = normalized_file_stem(name);
    for suffix in OUTPUT_SUFFIXES {
        if let Some(base) = key.strip_suffix(suffix) {
            return base.to_string();
        }
    }
    if let Some(base) = key.strip_suffix('o') {
        if base.chars().last().is_some_and(|ch| ch.is_ascii_digit()) {
            return base.to_string();
        }
    }
    key
}

#[cfg(test)]
mod tests {
    use super::{output_match_key, output_name_hint};

    #[test]
    fn recognizes_lab_output_names() {
        for name in [
            "2-6o.xlsx",
            "dev_id-vd.csv",
            "Id-Vd-device.xlsx",
            "dev output curve.txt",
        ] {
            assert!(output_name_hint(name), "{name}");
        }
        for name in ["2-6.xlsx", "demo.xlsx", "photo.csv"] {
            assert!(!output_name_hint(name), "{name}");
        }
    }

    #[test]
    fn strips_output_roles_for_attachment_matching() {
        assert_eq!(output_match_key("2-6o.xlsx"), "2-6");
        assert_eq!(output_match_key("dev_A output.csv"), "dev-a");
    }
}
