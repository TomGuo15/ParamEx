//! Card-title rails: the accent-marked title recipe, right-pinned actions,
//! optional local navigation, and section headers with trailing metadata.

use eframe::egui::{self, CornerRadius, FontId, Sense, Stroke};

use crate::richtext;
use crate::theme::{tokens, type_scale};

use super::bold_family;

pub const HEADER_RAIL_HEIGHT: f32 = 20.0;
/// Space between the rail's rule and the card body below it.
const HEADER_BODY_GAP: f32 = 4.0;

const HEADER_ACCENT_WIDTH: f32 = 3.0;
const HEADER_ACCENT_HEIGHT: f32 = 12.0;
/// The rule sits this far below the rail so it clears the label descenders.
const HEADER_RULE_OFFSET: f32 = 2.0;

/// The app's card-title recipe: a restrained primary marker plus the shared
/// bold/ink markup-aware label at the control size. Pass the title in its final
/// case.
fn header_label(ui: &mut egui::Ui, title: &str) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(HEADER_ACCENT_WIDTH, HEADER_ACCENT_HEIGHT),
            Sense::hover(),
        );
        ui.painter()
            .rect_filled(rect, CornerRadius::same(1), tokens().primary);
        let base = FontId::new(type_scale::CONTROL, bold_family(ui));
        ui.label(richtext::layout_sub_sup(title, base, tokens().ink));
    });
}

/// Pin content to the right edge of the current horizontal row, vertically
/// centered. This is the shared row-layout seam for header actions, dismiss
/// affordances, and right-side row metadata.
pub fn right_aligned<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), add)
        .inner
}

fn header_rail<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let width = ui.available_width();
    let (rail, _) = ui.allocate_exact_size(egui::vec2(width, HEADER_RAIL_HEIGHT), Sense::hover());
    let inner = ui
        .scope_builder(
            egui::UiBuilder::new()
                .max_rect(rail)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                ui.set_width(width);
                ui.set_height(HEADER_RAIL_HEIGHT);
                add(ui)
            },
        )
        .inner;
    let y = rail.bottom() + HEADER_RULE_OFFSET;
    ui.painter().line_segment(
        [egui::pos2(rail.left(), y), egui::pos2(rail.right(), y)],
        Stroke::new(1.0_f32, tokens().border),
    );
    ui.add_space(HEADER_BODY_GAP);
    inner
}

/// Card header row with one or more right-pinned actions. This keeps panels at
/// the semantic level: provide the title and action content, while `ui_kit`
/// owns the horizontal row, title recipe, right pinning, and header/body gap.
pub fn header_action_row<R>(
    ui: &mut egui::Ui,
    title: &str,
    action: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    header_rail(ui, |ui| {
        header_label(ui, title);
        right_aligned(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            action(ui)
        })
    })
}

/// Card header row with title, optional local page switch/navigation beside it,
/// and actions pinned to the far right. This mirrors the app banner grammar:
/// product/title first, page switch next, utility actions right.
pub fn header_nav_action_row<N, A>(
    ui: &mut egui::Ui,
    title: &str,
    nav: impl FnOnce(&mut egui::Ui) -> N,
    action: impl FnOnce(&mut egui::Ui) -> A,
) -> (N, A) {
    header_rail(ui, |ui| {
        if !title.is_empty() {
            header_label(ui, title);
        }
        let nav_out = nav(ui);
        let action_out = right_aligned(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            action(ui)
        });
        (nav_out, action_out)
    })
}

/// A card section header: title plus optional right-pinned context metadata.
pub fn section_header(ui: &mut egui::Ui, title: &str, metadata: Option<&str>) {
    header_rail(ui, |ui| {
        header_label(ui, title);
        if let Some(metadata) = metadata {
            right_aligned(ui, |ui| {
                header_metadata(ui, metadata);
            });
        }
    });
}

fn header_metadata(ui: &mut egui::Ui, text: &str) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.label(richtext::layout_sub_sup(
            text,
            FontId::new(type_scale::BADGE, bold_family(ui)),
            tokens().ink_soft,
        ));
        let (rule, _) = ui.allocate_exact_size(egui::vec2(1.0, 10.0), Sense::hover());
        ui.painter().rect_filled(rule, 0, tokens().border);
    });
}
