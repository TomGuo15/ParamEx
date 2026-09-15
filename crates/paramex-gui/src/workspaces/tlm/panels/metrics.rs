//! TLM selected-group metric tiles: the reported max-current fit plus ALL the
//! median diagnostics the engine exposes (8 tiles). Labels are the canonical
//! markup constants from `labels`; values use the shared `format_ui` formatters.

use eframe::egui;
use paramex_core::tlm::GroupAnalysis;

use crate::format_ui::{fmt_ohm, fmt_r2, fmt_slope, DASH};
use crate::table_kit;
use crate::ui_kit;
use crate::workspaces::tlm::panels::labels;
use crate::workspaces::tlm::state::TlmState;

const TLM_SELECTED_ROW_H: f32 = 24.0;
const TLM_SELECTED_STATUS_GAP: f32 = 8.0;

/// Labelled metrics for one group's fit.
pub fn group_tiles(g: &GroupAnalysis) -> Vec<(&'static str, String)> {
    vec![
        (labels::TILE_RCONTACT, fmt_ohm(g.fit.intercept_ohm)),
        (
            labels::TILE_RC_PER_CONTACT,
            fmt_ohm(g.fit.rc_per_contact_ohm),
        ),
        (labels::TILE_SLOPE, fmt_slope(g.fit.slope_ohm_per_um)),
        (labels::TILE_R2, fmt_r2(g.fit.r_squared)),
        (
            labels::TILE_RCONTACT_MED,
            fmt_ohm(g.fit.intercept_median_ohm),
        ),
        (
            labels::TILE_RC_PER_CONTACT_MED,
            fmt_ohm(g.fit.rc_per_contact_median_ohm),
        ),
        (
            labels::TILE_SLOPE_MED,
            fmt_slope(g.fit.slope_median_ohm_per_um),
        ),
        (labels::TILE_R2_MED, fmt_r2(g.fit.r_squared_median)),
    ]
}

fn empty_group_tiles(value: &str) -> Vec<(&'static str, String)> {
    labels::TILE_LABELS
        .iter()
        .map(|label| (*label, value.to_string()))
        .collect()
}

pub fn show(ui: &mut egui::Ui, tlm: &TlmState) {
    ui_kit::card_slot(ui, |ui| {
        let selected = tlm.selected_group_analysis();
        let group_label = selected.map(|g| g.group.as_str());
        ui_kit::section_header(ui, "SELECTED", group_label);
        let warnings = selected.map(|g| g.warnings.as_slice()).unwrap_or_default();
        let warning_summary = (!warnings.is_empty()).then(|| {
            format!(
                "{} fit warning{}; see Results.",
                warnings.len(),
                if warnings.len() == 1 { "" } else { "s" }
            )
        });
        let (status_badge, status_tone, status_detail) = match selected {
            None => (
                "empty",
                ui_kit::BadgeTone::Warning,
                ui_kit::StatusLineText::Inline("No group selected."),
            ),
            Some(_) if warnings.is_empty() => (
                "ok",
                ui_kit::BadgeTone::Ok,
                ui_kit::StatusLineText::Inline("Fit quality acceptable."),
            ),
            Some(_) => (
                "warn",
                ui_kit::BadgeTone::Warning,
                ui_kit::StatusLineText::Inline(
                    warning_summary.as_deref().expect("non-empty warnings"),
                ),
            ),
        };
        let row_cells: Vec<Vec<String>> = selected
            .map(group_tiles)
            .unwrap_or_else(|| empty_group_tiles(DASH))
            .into_iter()
            .map(|(label, value)| vec![label.to_string(), value])
            .collect();
        let card_w = ui.available_width();
        // Plain striped label/value rows (the original Transfer look) — the user
        // explicitly rejected card-inside-card tiles here. Rendered
        // as a full-card-width table so the stripes span the whole card.
        table_kit::striped_fill_table(
            ui,
            table_kit::FillTableSpec {
                id: "tlm_group_tiles",
                headers: &[],
                base_ws: &[110.0, 64.0],
                rows: &row_cells,
                card_w,
                row_h: TLM_SELECTED_ROW_H,
            },
            ui_kit::metric_table_cell,
        );
        ui.add_space(TLM_SELECTED_STATUS_GAP);
        let status =
            ui_kit::status_badge_line(ui, status_badge, status_tone, status_detail, |_| {});
        if !warnings.is_empty() {
            status.response.on_hover_text(warnings.join("\n"));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_rows_and_status_gap_keep_the_200_point_body() {
        assert_eq!(8.0 * TLM_SELECTED_ROW_H + TLM_SELECTED_STATUS_GAP, 200.0);
    }
}
