//! Plot legend measurement and rendering.

use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegendMark {
    Dot,
    SolidLine,
    DashedLine,
    Diamond,
}

const LEGEND_SWATCH_WIDTH: f32 = 14.0;
const LEGEND_SWATCH_HEIGHT: f32 = 10.0;
/// Extra space between legend entries beyond the ui's item spacing.
const LEGEND_ENTRY_GAP: f32 = 8.0;

/// A compact legend mark matching the plot data language: dots for measured
/// points, dashed samples for fits, and a painted diamond when the plot marker
/// itself uses a diamond.
pub fn legend_swatch(ui: &mut egui::Ui, color: egui::Color32, mark: LegendMark) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(LEGEND_SWATCH_WIDTH, LEGEND_SWATCH_HEIGHT),
        egui::Sense::hover(),
    );
    match mark {
        LegendMark::Dot => {
            ui.painter().circle_filled(rect.center(), 3.0, color);
        }
        LegendMark::SolidLine => {
            let y = rect.center().y;
            ui.painter().line_segment(
                [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                egui::Stroke::new(1.5_f32, color),
            );
        }
        LegendMark::DashedLine => {
            let y = rect.center().y;
            for seg in 0..2 {
                let x0 = rect.left() + seg as f32 * 8.0;
                ui.painter().line_segment(
                    [egui::pos2(x0, y), egui::pos2(x0 + 5.0, y)],
                    egui::Stroke::new(1.5_f32, color),
                );
            }
        }
        LegendMark::Diamond => {
            let c = rect.center();
            let r = 4.0;
            ui.painter().add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(c.x, c.y - r),
                    egui::pos2(c.x + r, c.y),
                    egui::pos2(c.x, c.y + r),
                    egui::pos2(c.x - r, c.y),
                ],
                color,
                egui::Stroke::NONE,
            ));
        }
    }
}

/// Muted rich legend label, using the same voice as the plot ticks.
pub fn legend_label(ui: &mut egui::Ui, markup: &str) -> egui::Response {
    ui.label(crate::richtext::layout_sub_sup(
        markup,
        super::tick_font(),
        super::muted_text_color(),
    ))
}

/// Pixel width for a legend entry's text label in the plot-furniture voice.
pub fn legend_label_width(ui: &egui::Ui, markup: &str) -> f32 {
    let job =
        crate::richtext::layout_sub_sup(markup, super::tick_font(), super::muted_text_color());
    ui.painter().layout_job(job).rect.width()
}

pub fn legend_entry_width(ui: &egui::Ui, markup: &str) -> f32 {
    LEGEND_SWATCH_WIDTH + ui.spacing().item_spacing.x + legend_label_width(ui, markup)
}

pub fn legend_row_width(ui: &egui::Ui, labels: &[&str]) -> f32 {
    if labels.is_empty() {
        return 0.0;
    }
    let entry_gap = LEGEND_ENTRY_GAP + 2.0 * ui.spacing().item_spacing.x;
    labels
        .iter()
        .map(|label| legend_entry_width(ui, label))
        .sum::<f32>()
        + (labels.len() - 1) as f32 * entry_gap
}

pub fn legend_entry(ui: &mut egui::Ui, color: egui::Color32, mark: LegendMark, label: &str) {
    legend_swatch(ui, color, mark);
    legend_label(ui, label);
}

pub fn centered_legend_row(ui: &mut egui::Ui, entries: &[(egui::Color32, LegendMark, &str)]) {
    let labels: Vec<&str> = entries.iter().map(|(_, _, label)| *label).collect();
    let content_w = legend_row_width(ui, &labels);
    ui.horizontal(|ui| {
        ui.add_space(((ui.available_width() - content_w) / 2.0).max(0.0));
        for (idx, (color, mark, label)) in entries.iter().copied().enumerate() {
            if idx > 0 {
                ui.add_space(LEGEND_ENTRY_GAP);
            }
            legend_entry(ui, color, mark, label);
        }
    });
}
