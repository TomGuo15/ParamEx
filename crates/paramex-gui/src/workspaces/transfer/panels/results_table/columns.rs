//! GUI-owned Transfer results-table columns.

use paramex_core::transfer::ResultsTableColumn;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GuiColumnSpec {
    pub column: ResultsTableColumn,
    pub label_html: &'static str,
    pub right_aligned: bool,
    pub min_width: f32,
}

const fn column(
    column: ResultsTableColumn,
    label_html: &'static str,
    right_aligned: bool,
    min_width: f32,
) -> GuiColumnSpec {
    GuiColumnSpec {
        column,
        label_html,
        right_aligned,
        min_width,
    }
}

const GUI_COLUMNS: [GuiColumnSpec; 8] = [
    column(ResultsTableColumn::Filename, "File", false, 122.0),
    column(ResultsTableColumn::Sweep, "Dir", false, 34.0),
    column(
        ResultsTableColumn::ThresholdVoltage,
        "V<sub>TH</sub> (V)",
        true,
        56.0,
    ),
    column(
        ResultsTableColumn::SaturationMobility,
        "\u{00B5}<sub>sat</sub> (cm<sup>2</sup> V<sup>-1</sup> s<sup>-1</sup>)",
        true,
        72.0,
    ),
    column(
        ResultsTableColumn::SubthresholdSwing,
        "SS (mV dec<sup>-1</sup>)",
        true,
        68.0,
    ),
    column(
        ResultsTableColumn::OnCurrent,
        "I<sub>on</sub> (A)",
        true,
        60.0,
    ),
    column(
        ResultsTableColumn::OffCurrent,
        "I<sub>off</sub> (A)",
        true,
        68.0,
    ),
    column(
        ResultsTableColumn::OnOffRatio,
        "I<sub>on</sub>/I<sub>off</sub>",
        true,
        68.0,
    ),
];

pub const fn gui_column_specs() -> &'static [GuiColumnSpec] {
    &GUI_COLUMNS
}

pub const fn indexed_gui_column_specs() -> &'static [GuiColumnSpec] {
    gui_column_specs()
}

pub fn gui_header_label_html(spec: &GuiColumnSpec) -> &'static str {
    spec.label_html
}

pub fn col_right_aligned(spec: &GuiColumnSpec) -> bool {
    spec.right_aligned
}

pub fn col_min_width(spec: &GuiColumnSpec) -> f32 {
    spec.min_width
}

pub fn table_min_width() -> f32 {
    GUI_COLUMNS.iter().map(|spec| spec.min_width).sum()
}
