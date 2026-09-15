//! The Technical Guide modal: input contracts and fit mathematics for each
//! workspace, rendered from committed equation SVGs.

use eframe::egui;

use crate::state::Workspace;
use crate::theme::{radius, token_alpha, tokens, type_scale};
use crate::ui_kit;

const GUIDE_TABS: [&str; 2] = ["Transfer", "TLM"];
const GUIDE_TAB_LABELS: [&str; 2] = ["Transfer guide", "TLM guide"];
const GUIDE_WIDTH: f32 = 900.0;
/// Window height the modal leaves free above and below its body.
const GUIDE_VIEWPORT_HEADROOM: f32 = 120.0;
const GUIDE_MIN_BODY_HEIGHT: f32 = 360.0;
const GUIDE_MAX_BODY_HEIGHT: f32 = 620.0;
/// Modal height reserved above the scrolling body for the title rail and its
/// rule/gap.
const GUIDE_HEADER_RESERVE: f32 = 30.0;
const GUIDE_TEXT_WIDTH: f32 = 660.0;
const GUIDE_CONTRACT_WIDTH: f32 = 720.0;
const GUIDE_CONTRACT_KEY_WIDTH: f32 = 64.0;
const GUIDE_SECTION_GAP: f32 = 14.0;

/// Show the guide when `open`; `page` is the tab shown and persists between
/// openings. A click outside the modal closes it, except on the frame it was
/// opened (the opening click is still in flight).
pub(super) fn show_help_window(
    ctx: &egui::Context,
    open: &mut bool,
    page: &mut Workspace,
    just_opened: bool,
) {
    if !*open {
        return;
    }
    egui_extras::install_image_loaders(ctx);
    let mut close_clicked = false;
    let body_height = (ctx.viewport_rect().height() - GUIDE_VIEWPORT_HEADROOM)
        .clamp(GUIDE_MIN_BODY_HEIGHT, GUIDE_MAX_BODY_HEIGHT);
    let response = egui::Modal::new(egui::Id::new("technical_guide"))
        .frame(ui_kit::card_frame())
        .backdrop_color(token_alpha(
            tokens().ink,
            crate::theme::MODAL_BACKDROP_ALPHA,
        ))
        .show(ctx, |ui| {
            ui.set_width(GUIDE_WIDTH);
            ui.set_height(body_height + GUIDE_HEADER_RESERVE);
            let (next_page, close) = ui_kit::header_nav_action_row(
                ui,
                "TECHNICAL GUIDE",
                |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui_kit::segmented_with_accessibility_labels(
                        ui,
                        &GUIDE_TABS,
                        &GUIDE_TAB_LABELS,
                        page.index(),
                        ui_kit::SegStyle::Card,
                        None,
                    )
                },
                |ui| ui_kit::close_button(ui, "Close guide").clicked(),
            );
            if let Some(index) = next_page {
                *page = Workspace::from_index(index);
            }
            close_clicked = close;

            egui::ScrollArea::vertical()
                .id_salt(("technical_guide_body", page.index()))
                .auto_shrink([false, true])
                .max_height(body_height)
                .min_scrolled_height(0.0)
                .scroll_bar_visibility(
                    egui::containers::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                )
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 7.0;
                    match *page {
                        Workspace::Transfer => transfer_guide(ui),
                        Workspace::Tlm => tlm_guide(ui),
                    }
                });
        });
    if close_clicked || (!just_opened && response.should_close()) {
        *open = false;
    }
}

fn transfer_guide(ui: &mut egui::Ui) {
    ui_kit::section_header(ui, "INPUT", None);
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        guide_contract_row(ui, "Files", ".csv · .tsv · .txt · .xls · .xlsx");
        guide_contract_row(ui, "Columns", "V<sub>G</sub> · I<sub>D</sub>");
        guide_contract_row(
            ui,
            "Points",
            "At least 12 measured points across the gate sweep.",
        );
    });

    ui.add_space(GUIDE_SECTION_GAP);
    ui_kit::section_header(ui, "FIT MATHEMATICS", None);
    guide_term(ui, "Threshold + saturation mobility");
    guide_math(
        ui,
        "Square root of absolute drain current equals m V G plus b; V T H equals minus b over m; mu sat equals two m squared over C ox times W over L",
        egui::include_image!("../../assets/math/transfer-threshold.svg"),
    );
    guide_text(ui, "Manual fit range: at least 5 points.");
    guide_term(ui, "Subthreshold swing");
    guide_math(
        ui,
        "Log base ten absolute drain current equals s V G plus c; subthreshold swing equals absolute one thousand over s millivolts per decade",
        egui::include_image!("../../assets/math/transfer-subthreshold.svg"),
    );
    guide_term(ui, "On/off + round-trip hysteresis");
    guide_math(
        ui,
        "I on equals maximum absolute I D; I off equals minimum positive absolute I D; on over off equals I on divided by I off; delta V T H equals the median over log current of backward V G minus forward V G.",
        egui::include_image!("../../assets/math/transfer-on-off-hysteresis.svg"),
    );
    guide_text(
        ui,
        "Hysteresis needs forward + reverse sweeps (≥12 points each).",
    );
}

fn tlm_guide(ui: &mut egui::Ui) {
    ui_kit::section_header(ui, "INPUT", None);
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        guide_contract_row(ui, "Files", ".xlsx");
        guide_code(ui, "root/\n  group/\n    <length_um>/\n      device.xlsx");
        guide_contract_row(
            ui,
            "Length",
            "Numeric folder = channel length in µm (for example, 50 means 50 µm).",
        );
        guide_contract_row(ui, "Data", "List(*) sheet: vg · abs_id · abs_is");
        guide_contract_row(
            ui,
            "Bias",
            "Setup(*) sheet: V<sub>D</sub>; else Fallback V<sub>D</sub>.",
        );
    });

    ui.add_space(GUIDE_SECTION_GAP);
    ui_kit::section_header(ui, "FIT MATHEMATICS", None);
    guide_term(ui, "Current at selected gate bias");
    guide_math(
        ui,
        "j d minimizes absolute measured V G minus selected V G; channel current is the minimum of absolute I D and absolute I S at j d; I L is the maximum channel current across devices at length L; R total equals absolute V D divided by I L.",
        egui::include_image!("../../assets/math/tlm-current.svg"),
    );
    guide_term(ui, "Ordinary least squares");
    guide_math(
        ui,
        "R total of L equals m L plus b equals m L plus two R c; R c per contact equals b over two; m and b are the displayed ordinary least-squares estimators; R squared equals one minus residual sum of squares over total sum of squares.",
        egui::include_image!("../../assets/math/tlm-regression.svg"),
    );
    guide_text(
        ui,
        "Need ≥2 lengths (≥3 for R<sup>2</sup>). Primary: highest-current device per L; median: diagnostic. m is slope (Ω/µm), not sheet resistance.",
    );
}

fn guide_ink(ui: &egui::Ui) -> egui::Color32 {
    if ui.is_enabled() {
        tokens().ink
    } else {
        tokens().ink_soft
    }
}

fn guide_term(ui: &mut egui::Ui, text: &str) {
    let mut job = crate::richtext::layout_sub_sup(
        text,
        egui::FontId::new(type_scale::GUIDE_TERM, ui_kit::bold_family(ui)),
        guide_ink(ui),
    );
    job.wrap.max_width = ui.available_width();
    ui.label(job);
}

fn guide_text(ui: &mut egui::Ui, markup: &str) {
    let mut job = crate::richtext::layout_sub_sup(
        markup,
        egui::FontId::proportional(type_scale::GUIDE_BODY),
        guide_ink(ui),
    );
    job.wrap.max_width = ui.available_width().min(GUIDE_TEXT_WIDTH);
    ui.label(job);
}

fn guide_contract_text(ui: &mut egui::Ui, markup: &str) {
    let width = ui.available_width().min(GUIDE_CONTRACT_WIDTH);
    ui.allocate_ui_with_layout(
        egui::vec2(width, 0.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.set_width(width);
            guide_text(ui, markup);
        },
    );
}

fn guide_contract_row(ui: &mut egui::Ui, term: &str, markup: &str) {
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        ui.allocate_ui_with_layout(
            egui::vec2(GUIDE_CONTRACT_KEY_WIDTH, 0.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| guide_term(ui, term),
        );
        guide_contract_text(ui, markup);
    });
}

fn guide_code(ui: &mut egui::Ui, markup: &str) {
    let outer_width = ui.available_width();
    let frame = egui::Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .corner_radius(egui::CornerRadius::same(radius::CHIP))
        .inner_margin(egui::Margin::symmetric(9, 6));
    let width = (outer_width - frame.total_margin().sum().x).max(0.0);
    frame.show(ui, |ui| {
        ui.set_min_width(width);
        let mut job = crate::richtext::layout_sub_sup(
            markup,
            egui::FontId::monospace(type_scale::HEADER_TAB),
            tokens().ink,
        );
        job.wrap.max_width = ui.available_width();
        ui.label(job);
    });
}

fn guide_math(ui: &mut egui::Ui, alt: &str, source: egui::ImageSource<'static>) {
    let outer_width = ui.available_width();
    let frame = egui::Frame::new()
        .fill(token_alpha(
            tokens().primary,
            crate::theme::GUIDE_MATH_FILL_ALPHA,
        ))
        .stroke(egui::Stroke::new(
            1.0_f32,
            token_alpha(tokens().primary, crate::theme::GUIDE_MATH_STROKE_ALPHA),
        ))
        .corner_radius(egui::CornerRadius::same(radius::SEGMENT))
        .inner_margin(egui::Margin::symmetric(12, 9));
    let width = (outer_width - frame.total_margin().sum().x).max(0.0);
    frame.show(ui, |ui| {
        ui.set_min_width(width);
        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            ui.add(
                egui::Image::new(source)
                    .alt_text(alt)
                    .fit_to_original_size(1.0)
                    .max_width(width),
            );
        });
    });
}
