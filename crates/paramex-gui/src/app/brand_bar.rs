//! Top brand bar: the painted logo mark, the "ParamEx" wordmark, the
//! Transfer/TLM workspace toggle, the guide action, and the 7-click Suisei
//! easter egg. The gold comet mark is the egg's only feedback; it deliberately
//! raises no toast.

use eframe::egui::{self, FontId, Pos2, Rect, Response, RichText, Sense, Stroke, StrokeKind, Vec2};

use crate::state::{EasterEgg, Workspace};
use crate::theme::{self, type_scale};
use crate::ui_kit;

const BANNER_WORDMARK_SIZE: f32 = 15.0;
const BRAND_MARK_WIDTH: f32 = 58.0;
const BRAND_MARK_HEIGHT: f32 = 34.0;
const BRAND_MARK_CORNER_RADIUS: u8 = 8;
const BRAND_MARK_BORDER_ALPHA: u8 = 28;
const BRAND_LOGO_PRIMARY_WIDTH: f32 = 2.0;
const BRAND_LOGO_CONNECTOR_WIDTH: f32 = 1.5;
const BRAND_LOGO_GUIDE_WIDTH: f32 = 1.0;
const BRAND_LOGO_DOT_RADIUS: f32 = 1.6;
const BRAND_LOGO_CONNECTOR_ALPHA: u8 = 76;
const BRAND_LOGO_GUIDE_ALPHA: u8 = 114;
const BRAND_LOGO_DOT_SOFT_ALPHA: u8 = 102;
const BRAND_LOGO_DOT_STRONG_ALPHA: u8 = 216;
const SUISEI_WATERMARK_ALPHA: u8 = 48;
/// Rendered height of the comet mark; its width follows the PNG's aspect.
const SUISEI_MARK_HEIGHT: f32 = 24.0;
/// The comet PNG is 142×88.
const SUISEI_ASPECT_RATIO: f32 = 142.0 / 88.0;
/// Gap between the comet mark and the guide action.
const BANNER_ITEM_GAP: f32 = 8.0;
const BANNER_ACTION_SIZE: f32 = 24.0;
const BANNER_ACTION_REST_ALPHA: u8 = 150;
const BANNER_ACTION_HOVER_ALPHA: u8 = 230;

/// Draw the brand bar. `egg` is the easter-egg counter; the 7th logo click toggles
/// the gold Suisei comet on the right. `active` is the current workspace — the
/// `[Transfer][TLM]` toggle reads + writes it.
pub fn show(ui: &mut egui::Ui, egg: &mut EasterEgg, active: &mut Workspace, show_help: &mut bool) {
    ui.add_space(10.0);
    ui.horizontal_centered(|ui| {
        ui.add_space(6.0);
        // The 58×34 brand mark sits on the dark bar with a subtle white hairline
        // so the white logo art reads against the ink banner.
        let (rect, resp) = ui.allocate_exact_size(
            egui::vec2(BRAND_MARK_WIDTH, BRAND_MARK_HEIGHT),
            Sense::click(),
        );
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, BRAND_MARK_CORNER_RADIUS, theme::INK_RAISED);
        painter.rect_stroke(
            rect.shrink(0.5),
            BRAND_MARK_CORNER_RADIUS,
            Stroke::new(1.0_f32, theme::utility_white_alpha(BRAND_MARK_BORDER_ALPHA)),
            StrokeKind::Inside,
        );
        paint_logo(&painter, rect);
        if resp.clicked() {
            // The 7th click toggles the gold comet on the right; that tint is
            // the whole payoff, so no toast fires here.
            egg.register_click();
        }

        ui.add_space(10.0);
        // White name + the workspace toggle, legible on the dark banner.
        ui.label(
            RichText::new("ParamEx")
                .color(theme::tokens().surface)
                .size(BANNER_WORDMARK_SIZE)
                .strong(),
        );
        ui.add_space(8.0);
        // The workspace toggle: white lifted pill = active, transparent + white label =
        // inactive, both on the black raised track.
        if let Some(idx) = ui_kit::segmented(
            ui,
            &["Transfer", "TLM"],
            active.index(),
            ui_kit::SegStyle::Banner,
            Some(86.0),
        ) {
            *active = Workspace::from_index(idx);
        }

        // Right end of the bar, laid out right-to-left: the guide action sits
        // flush with the card column's right edge, and the Suisei comet mark
        // sits to its left — a faint watermark normally, gold when the 7-click
        // easter egg is active. The bar background is painted full-width by
        // the app shell, so this block only places its two items and does not
        // drive the banner width.
        ui_kit::right_aligned(ui, |ui| {
            ui.add_space(crate::layout::PAGE_PAD_X);
            if banner_action(ui, "Technical guide").clicked() {
                *show_help = true;
            }
            ui.add_space(BANNER_ITEM_GAP);
            let tex = suisei_texture(ui.ctx());
            let size = Vec2::new(SUISEI_MARK_HEIGHT * SUISEI_ASPECT_RATIO, SUISEI_MARK_HEIGHT);
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            let tint = if egg.is_shown() {
                theme::tokens().yellow
            } else {
                theme::utility_white_alpha(SUISEI_WATERMARK_ALPHA)
            };
            ui.painter().image(
                tex.id(),
                rect,
                Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                tint,
            );
        });
    });
}

fn banner_action(ui: &mut egui::Ui, label: &str) -> Response {
    let enabled = ui.is_enabled();
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, mut response) = ui.allocate_exact_size(egui::Vec2::splat(BANNER_ACTION_SIZE), sense);
    let alpha = if response.is_pointer_button_down_on() {
        255
    } else if response.hovered() || response.has_focus() {
        BANNER_ACTION_HOVER_ALPHA
    } else {
        BANNER_ACTION_REST_ALPHA
    };
    let color = theme::utility_white_alpha(alpha);
    ui.painter()
        .circle_stroke(rect.center(), 9.0, Stroke::new(1.4_f32, color));
    ui.painter().text(
        rect.center() + egui::vec2(0.0, -0.5),
        egui::Align2::CENTER_CENTER,
        "?",
        FontId::new(type_scale::HEADER_TAB, ui_kit::bold_family(ui)),
        color,
    );
    if enabled {
        response = response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(label);
    }
    let owned = label.to_owned();
    response.widget_info(move || {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, owned.clone())
    });
    response
}

/// Load (and cache per-context) the Suisei comet PNG as a texture. White on
/// transparent, so callers tint it.
fn suisei_texture(ctx: &egui::Context) -> egui::TextureHandle {
    let id = egui::Id::new("paramex_suisei_tex");
    if let Some(handle) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return handle;
    }
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../../assets/suisei.png"))
        .expect("baked suisei.png decodes");
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [icon.width as usize, icon.height as usize],
        &icon.rgba,
    );
    let handle = ctx.load_texture("suisei", image, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, handle.clone()));
    handle
}

/// The ParamEx logo, painted from its 100×62 design-space coordinates: 7
/// scatter dots, the bold central fit line, and two vertical fit-window guides,
/// white on the dark mark. The coordinates scale into the mark's inner area.
fn paint_logo(painter: &egui::Painter, rect: Rect) {
    // Inset the 100×62 design space so the art clears the mark's border.
    let inset = Vec2::new(8.0, 4.0);
    let area = Rect::from_min_max(rect.min + inset, rect.max - inset);
    let sx = area.width() / 100.0;
    let sy = area.height() / 62.0;
    let p = |x: f32, y: f32| Pos2::new(area.min.x + x * sx, area.min.y + y * sy);
    let primary_stroke = Stroke::new(BRAND_LOGO_PRIMARY_WIDTH, theme::utility_white_alpha(255));
    let connector_stroke = Stroke::new(
        BRAND_LOGO_CONNECTOR_WIDTH,
        theme::utility_white_alpha(BRAND_LOGO_CONNECTOR_ALPHA),
    );
    let guide_stroke = Stroke::new(
        BRAND_LOGO_GUIDE_WIDTH,
        theme::utility_white_alpha(BRAND_LOGO_GUIDE_ALPHA),
    );
    // Bold central fit line (30,43)->(70,18).
    painter.line_segment([p(30.0, 43.0), p(70.0, 18.0)], primary_stroke);
    // Faint connector segments.
    painter.line_segment([p(8.0, 56.0), p(30.0, 43.0)], connector_stroke);
    painter.line_segment([p(70.0, 18.0), p(92.0, 5.0)], connector_stroke);
    // Two thin vertical guides at x=30 and x=70 marking the fit window.
    for x in [30.0_f32, 70.0] {
        painter.line_segment([p(x, 6.0), p(x, 57.0)], guide_stroke);
    }
    // Scatter dots (cx, cy, opacity) in design space.
    for (cx, cy, a) in [
        (8.0, 56.0, 0.4),
        (23.0, 47.0, 0.4),
        (38.0, 38.0, 0.85),
        (53.0, 29.0, 0.85),
        (68.0, 20.0, 0.85),
        (77.0, 14.0, 0.4),
        (92.0, 5.0, 0.4),
    ] {
        let alpha = if a > 0.5 {
            BRAND_LOGO_DOT_STRONG_ALPHA
        } else {
            BRAND_LOGO_DOT_SOFT_ALPHA
        };
        painter.circle_filled(
            p(cx, cy),
            BRAND_LOGO_DOT_RADIUS,
            theme::utility_white_alpha(alpha),
        );
    }
}
