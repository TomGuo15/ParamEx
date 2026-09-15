//! Overall-row stacked-cell rendering policy.

use eframe::egui;

use crate::richtext;
use crate::table_kit::CELL_INSET;

/// Row height for an Overall summary row: it stacks the mean over a smaller
/// std/N line, so it needs room for two lines. Stacking keeps metric columns as
/// narrow as per-file single values.
pub(super) const OVERALL_ROW_H: f32 = 36.0;
/// The std/N sub-line is this fraction of the body font size.
const STD_SCALE: f32 = 0.8;

pub(super) fn small_font(base: &egui::FontId) -> egui::FontId {
    egui::FontId::new(base.size * STD_SCALE, base.family.clone())
}

/// Split an Overall-row cell into a primary line and an optional smaller
/// sub-line so it can be stacked: `"0.90 +/- 0.14"` style values split before
/// the std marker, and `"Forward N=2"` splits before `N=`.
pub(super) fn split_cell(text: &str) -> (&str, Option<&str>) {
    if let Some(idx) = text.find(" \u{00B1} ") {
        // Keep the +/- marker on the sub-line: drop only the leading space
        // before it.
        (&text[..idx], Some(&text[idx + 1..]))
    } else if let Some(idx) = text.find(" N=") {
        (&text[..idx], Some(&text[idx + 1..]))
    } else {
        (text, None)
    }
}

/// Paint an Overall-row cell as a primary line over a smaller muted sub-line,
/// aligned per the column. Single-line cells render as one aligned line.
pub(super) fn paint_stacked(ui: &mut egui::Ui, text: &str, right: bool) {
    if text.is_empty() {
        return;
    }
    let (head, tail) = split_cell(text);
    let cell = ui.max_rect();
    let center_y = cell.center().y;
    let anchor_x = |w: f32| {
        if right {
            cell.right() - CELL_INSET - w
        } else {
            cell.left() + CELL_INSET
        }
    };

    let text_color = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let base = egui::TextStyle::Body.resolve(ui.style());
    let small = small_font(&base);

    let head_galley = ui
        .painter()
        .layout_job(richtext::layout_sub_sup(head, base, text_color));
    let (hw, hh) = (head_galley.rect.width(), head_galley.rect.height());
    match tail {
        Some(tail) => {
            let tail_galley = ui
                .painter()
                .layout_job(richtext::layout_sub_sup(tail, small, weak));
            let (tw, th) = (tail_galley.rect.width(), tail_galley.rect.height());
            let line_gap = 1.0;
            let top = center_y - (hh + line_gap + th) / 2.0;
            ui.painter()
                .galley(egui::pos2(anchor_x(hw), top), head_galley, text_color);
            ui.painter().galley(
                egui::pos2(anchor_x(tw), top + hh + line_gap),
                tail_galley,
                weak,
            );
        }
        None => {
            ui.painter().galley(
                egui::pos2(anchor_x(hw), center_y - hh / 2.0),
                head_galley,
                text_color,
            );
        }
    }
}
