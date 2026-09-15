//! Pure column specs for the four TLM tables.
//! Headers carry units; cells stay bare. Rendered via `richtext` everywhere —
//! never literal underscores or raw Unicode sub/superscript codepoints.

/// How a cell renders: plain text, a colored ok/error status, or yellow warnings.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CellKind {
    Text,
    Status,
    Warnings,
}

/// One TLM table column: markup header label, minimum width, render kind,
/// whether it right-aligns (numeric columns — the quiet-table convention;
/// Segoe UI digits are tabular, so right-aligned figures line up), and whether
/// it YIELDS width: prose columns (warnings, file paths) clip-with-hover by
/// design, so they give up width before the whole table takes a horizontal
/// scrollbar.
pub struct TlmCol {
    pub label: &'static str,
    pub min_w: f32,
    pub kind: CellKind,
    pub right: bool,
    pub yields: bool,
}

const fn text(label: &'static str, min_w: f32) -> TlmCol {
    TlmCol {
        label,
        min_w,
        kind: CellKind::Text,
        right: false,
        yields: false,
    }
}

const fn num(label: &'static str, min_w: f32) -> TlmCol {
    TlmCol {
        label,
        min_w,
        kind: CellKind::Text,
        right: true,
        yields: false,
    }
}

/// A prose column (file path): left-aligned, clips with hover, yields width.
const fn prose(label: &'static str, min_w: f32) -> TlmCol {
    TlmCol {
        label,
        min_w,
        kind: CellKind::Text,
        right: false,
        yields: true,
    }
}

/// Warning diagnostics render as a compact badge with full text on hover.
/// The column still yields width so fixed numeric columns remain readable.
const fn warnings(label: &'static str, min_w: f32) -> TlmCol {
    TlmCol {
        label,
        min_w,
        kind: CellKind::Warnings,
        right: false,
        yields: true,
    }
}

/// Results table (per group at the chosen V_G). No V_G column: it is one
/// constant value for the whole table (the ANALYSIS pick, shown in the plot
/// title rail) — same rule as the Length-points table.
pub const RESULT_COLS: [TlmCol; 7] = [
    text("group", 70.0),
    num("intercept (2R<sub>c</sub>) (\u{2126})", 56.0),
    num("R<sub>c</sub>/contact (\u{2126})", 56.0),
    num("slope (\u{2126}/\u{00B5}m)", 56.0),
    num("R<sup>2</sup>", 56.0),
    num("lengths", 50.0),
    warnings("warnings", 90.0),
];

/// Voltage-Sweep table (per group × V_G) — the Results shape PLUS the V_G that
/// actually varies here.
pub const SWEEP_COLS: [TlmCol; 8] = [
    text("group", 70.0),
    num("V<sub>G</sub> (V)", 56.0),
    num("intercept (2R<sub>c</sub>) (\u{2126})", 56.0),
    num("R<sub>c</sub>/contact (\u{2126})", 56.0),
    num("slope (\u{2126}/\u{00B5}m)", 56.0),
    num("R<sup>2</sup>", 56.0),
    num("lengths", 50.0),
    warnings("warnings", 90.0),
];

/// Length-Points table (per group × length). No selected-V_G column: it is one
/// constant value for the whole table (the ANALYSIS pick) and read as a duplicate
/// of the per-point actual V_G next to it.
pub const LENGTH_COLS: [TlmCol; 9] = [
    text("group", 70.0),
    num("L (\u{00B5}m)", 50.0),
    num("V<sub>G</sub> (V)", 56.0),
    num("I (A)", 56.0),
    num("R<sub>tot</sub> (\u{2126})", 56.0),
    num("I median (A)", 56.0),
    num("R<sub>tot</sub> median (\u{2126})", 56.0),
    num("devices", 50.0),
    prose("file", 110.0),
];

/// File-Status table (the right-column FILES card): file + the pass/fail signal.
/// The error MESSAGE rides along as a hover payload on the status cell — see
/// `status_rows` / `grid_table`'s Status arm.
pub const STATUS_COLS: [TlmCol; 2] = [
    prose("file", 130.0),
    TlmCol {
        label: "status",
        min_w: 56.0,
        kind: CellKind::Status,
        right: false,
        yields: false,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_literal_underscore_or_unicode_subscript_in_labels() {
        let all: Vec<&str> = RESULT_COLS
            .iter()
            .chain(SWEEP_COLS.iter())
            .chain(LENGTH_COLS.iter())
            .chain(STATUS_COLS.iter())
            .map(|c| c.label)
            .collect();
        for label in all {
            assert!(!label.contains('_'), "literal underscore in {label:?}");
            // the tofu range richtext exists to kill (U+2080..=U+209C subscripts)
            assert!(
                !label
                    .chars()
                    .any(|c| ('\u{2080}'..='\u{209C}').contains(&c)),
                "raw Unicode sub/superscript in {label:?}"
            );
        }
    }

    #[test]
    fn numeric_columns_right_align_text_columns_left() {
        assert!(!RESULT_COLS[0].right && !RESULT_COLS[6].right); // group, warnings
        assert!(RESULT_COLS[1..6].iter().all(|c| c.right)); // R_c..lengths
        assert!(!SWEEP_COLS[0].right && !SWEEP_COLS[7].right);
        assert!(SWEEP_COLS[1..7].iter().all(|c| c.right)); // V_G..lengths
        assert!(!LENGTH_COLS[0].right && !LENGTH_COLS[8].right); // group, file
        assert!(LENGTH_COLS[1..8].iter().all(|c| c.right));
        assert!(STATUS_COLS.iter().all(|c| !c.right));
    }

    #[test]
    fn only_prose_columns_yield_width() {
        // Warnings + file-path columns clip-with-hover, so they alone yield.
        assert!(RESULT_COLS[6].yields && !RESULT_COLS[..6].iter().any(|c| c.yields));
        assert!(SWEEP_COLS[7].yields && !SWEEP_COLS[..7].iter().any(|c| c.yields));
        assert!(LENGTH_COLS[8].yields && !LENGTH_COLS[..8].iter().any(|c| c.yields));
        assert!(STATUS_COLS[0].yields && !STATUS_COLS[1].yields);
    }

    #[test]
    fn column_counts_match_row_builders() {
        assert_eq!(RESULT_COLS.len(), 7);
        assert_eq!(SWEEP_COLS.len(), 8);
        assert_eq!(LENGTH_COLS.len(), 9);
        assert_eq!(STATUS_COLS.len(), 2);
    }
}
