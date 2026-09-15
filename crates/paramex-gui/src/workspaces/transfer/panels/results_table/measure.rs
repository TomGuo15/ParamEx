//! Results-table column measurement, shared by the transfer and output tables.

use eframe::egui;

use crate::table_kit::{fill_card_width, galley_width, pad_and_clamp};

pub(super) struct FittedWidths {
    pub column_widths: Vec<f32>,
    pub painted_w: f32,
}

/// Measure each GUI column's stable render width from its header label and
/// declared floor only. Row contents clip on overflow instead of moving the
/// column grid after load.
pub(super) fn measure_col_widths(
    ui: &egui::Ui,
    specs: &[super::columns::GuiColumnSpec],
) -> Vec<f32> {
    measure_header_widths(
        ui,
        specs.iter().map(|spec| (spec.label_html, spec.min_width)),
    )
}

/// Measure each column's stable render width from its header label and
/// declared floor only. Row contents clip on overflow instead of moving the
/// column grid after load.
pub(super) fn measure_header_widths<'a>(
    ui: &egui::Ui,
    columns: impl Iterator<Item = (&'a str, f32)>,
) -> Vec<f32> {
    // Headers measure at the muted 11px font they render in (same rule as
    // table_kit::measure_grid_col_galleys).
    let header_font = crate::table_kit::muted_header_font();
    columns
        .map(|(label, min_width)| pad_and_clamp(galley_width(ui, label, &header_font), min_width))
        .collect()
}

/// Fit the stable measured widths to the live card width: shrink toward the
/// floors when the table overflows, then fill any spare width. The summed
/// floors are the declared table width that keeps the narrow center card
/// scrolling horizontally instead of collapsing columns.
pub(super) fn fit_table_widths(
    ui: &egui::Ui,
    measured: &[f32],
    min_widths: &[f32],
    card_w: f32,
) -> FittedWidths {
    // Columns are sized from stable schema/header widths. The live card width
    // changes per frame, so fit a local copy before rendering.
    let mut column_widths = measured.to_vec();
    // Gap-aware fill: the PAINTED table is columns plus inter-column gaps, so
    // fill against the card width minus those gaps. Only genuinely wider content
    // keeps the mid-cell cut and horizontal scroll.
    let gaps = min_widths.len().saturating_sub(1) as f32 * ui.spacing().item_spacing.x;
    let avail = (card_w - gaps).max(0.0);
    shrink_widths_to_fit(&mut column_widths, min_widths, avail);
    fill_card_width(&mut column_widths, avail);
    // Declare the full table width so the narrow center card scrolls
    // horizontally. Group separators span the painted width (columns + gaps).
    let table_min_width = min_widths.iter().sum::<f32>();
    let table_w = column_widths.iter().sum::<f32>().max(table_min_width);
    let painted_w = table_w + gaps;
    FittedWidths {
        column_widths,
        painted_w,
    }
}

fn shrink_widths_to_fit(widths: &mut [f32], min_widths: &[f32], avail: f32) {
    let current = widths.iter().sum::<f32>();
    if current <= avail {
        return;
    }
    let min_sum = min_widths.iter().sum::<f32>();
    if min_sum > avail {
        return;
    }
    let shrink_capacity = widths
        .iter()
        .zip(min_widths.iter())
        .map(|(width, floor)| (width - floor).max(0.0))
        .sum::<f32>();
    if shrink_capacity <= 0.0 {
        return;
    }
    let excess = current - avail;
    for (width, floor) in widths.iter_mut().zip(min_widths.iter()) {
        let capacity = (*width - *floor).max(0.0);
        *width = (*width - excess * capacity / shrink_capacity).max(*floor);
    }
}
