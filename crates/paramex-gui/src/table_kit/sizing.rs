//! Quiet-table column measurement and width-fitting policy.

use eframe::egui;

use super::{galley_width, muted_header_font, CELL_PAD_X, COL_MAX_WIDTH};

/// The padding floor when a fill table is tight on space: columns keep
/// a sliver of breathing room while every value stays fully readable.
const CELL_PAD_X_MIN: f32 = 2.0;

/// Pad a measured galley width and clamp to `[floor, COL_MAX_WIDTH]`. The floor
/// is capped first: `f32::clamp` panics when min > max, this module is the path
/// every table takes, and a future >260px floor must not become an every-frame
/// panic.
pub fn pad_and_clamp(galley_w: f32, floor: f32) -> f32 {
    (galley_w + CELL_PAD_X).clamp(floor.min(COL_MAX_WIDTH), COL_MAX_WIDTH)
}

/// Raw per-column content widths: the max galley of the header + every cell
/// (markup-aware), with no padding or clamping applied. Headers are measured at
/// the muted font they RENDER in (`muted_header_label`); measuring them at the
/// larger Body size would inflate every unit-bearing column ~15% and push
/// near-fitting tables (TLM Fits-vs-V_G) into a needless horizontal scrollbar.
pub fn measure_grid_col_galleys(ui: &egui::Ui, headers: &[&str], rows: &[Vec<String>]) -> Vec<f32> {
    let base = egui::TextStyle::Body.resolve(ui.style());
    let header_font = muted_header_font();
    headers
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let mut w = galley_width(ui, h, &header_font);
            for row in rows {
                if let Some(cell) = row.get(i) {
                    if !cell.is_empty() {
                        w = w.max(galley_width(ui, cell, &base));
                    }
                }
            }
            w
        })
        .collect()
}

/// Measure plain string-grid columns: max of header + every cell (markup-aware),
/// plus padding, clamped to `[min_w, COL_MAX_WIDTH]`.
pub fn measure_grid_col_widths(
    ui: &egui::Ui,
    headers: &[&str],
    min_widths: &[f32],
    rows: &[Vec<String>],
) -> Vec<f32> {
    measure_grid_col_galleys(ui, headers, rows)
        .into_iter()
        .enumerate()
        .map(|(i, w)| pad_and_clamp(w, min_widths.get(i).copied().unwrap_or(56.0)))
        .collect()
}

/// Per-column widths for a vertical-only fill table. Loose (the ideal
/// padded/floored widths fit the card): scale them UP to span the full width.
/// Tight (they would overflow): text must never cut at the card edge, so the
/// decorations yield instead: per-column floors drop and the padding shrinks
/// evenly (12 -> 2px) until every value fits. Only when even minimal padding
/// cannot hold the raw text does the caller's horizontal scrolling take over.
pub fn fit_fill_widths(galleys: &[f32], min_ws: &[f32], card_w: f32, gap_x: f32) -> Vec<f32> {
    let n = galleys.len();
    if n == 0 {
        return Vec::new();
    }
    let gaps = (n - 1) as f32 * gap_x;
    let avail = (card_w - gaps).max(0.0);

    let mut widths: Vec<f32> = galleys
        .iter()
        .enumerate()
        .map(|(i, g)| pad_and_clamp(*g, min_ws.get(i).copied().unwrap_or(56.0)))
        .collect();
    if widths.iter().sum::<f32>() > avail {
        let content: f32 = galleys.iter().sum();
        let pad = ((avail - content) / n as f32).clamp(CELL_PAD_X_MIN, CELL_PAD_X);
        widths = galleys
            .iter()
            .map(|g| (g + pad).min(COL_MAX_WIDTH))
            .collect();
    }
    fill_card_width(&mut widths, avail);
    widths
}

/// Scale columns up proportionally so the table fills the card when narrower.
pub fn fill_card_width(widths: &mut [f32], card_w: f32) {
    let content_w: f32 = widths.iter().sum();
    if content_w > 0.0 && card_w > content_w {
        let scale = card_w / content_w;
        for w in widths {
            *w *= scale;
        }
    }
}

/// Fit measured grid columns to a card, letting prose/status columns yield
/// before the whole table overflows. Fixed columns keep their measured widths;
/// yielding columns share the leftover card width proportionally, never below
/// their declared floors. If even the floors do not fit, widths stay wider than
/// the card and the caller's horizontal scroll handles it.
pub fn fit_yielding_widths(
    widths: &mut [f32],
    min_ws: &[f32],
    yields: &[bool],
    card_w: f32,
    gap_x: f32,
) {
    let gaps = widths.len().saturating_sub(1) as f32 * gap_x;
    let avail = (card_w - gaps).max(0.0);
    let fixed: f32 = widths
        .iter()
        .zip(yields.iter().copied())
        .filter(|(_, yields)| !yields)
        .map(|(w, _)| *w)
        .sum();
    let yielding: f32 = widths
        .iter()
        .zip(yields.iter().copied())
        .filter(|(_, yields)| *yields)
        .map(|(w, _)| *w)
        .sum();
    let leftover = (avail - fixed).max(0.0);
    if yielding > leftover && yielding > 0.0 {
        for (i, width) in widths.iter_mut().enumerate() {
            if yields.get(i).copied().unwrap_or(false) {
                let floor = min_ws.get(i).copied().unwrap_or(0.0).min(COL_MAX_WIDTH);
                *width = (leftover * *width / yielding).max(floor);
            }
        }
    }
    fill_card_width(widths, avail);
}
