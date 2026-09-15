//! Palette identity: the base tokens, which groups the app tokens draw from,
//! contrast of the muted voice, and the semantic tints derived from them.

use eframe::egui::Color32;

use paramex_gui::theme::{
    token_alpha, tokens, utility_black_alpha, utility_white_alpha, INK_RAISED, KANADE_GOLD_PASTEL,
    RADEN_GREEN_PASTEL, RIRIKA_PINK_PASTEL, SUISEI_DARK, SUISEI_LIGHT, SUISEI_MAIN, UTILITY_BLACK,
    UTILITY_CHARCOAL, UTILITY_GRAY, UTILITY_WHITE,
};
use paramex_gui::ui_kit::{semantic_badge_colors, BadgeTone};
use paramex_gui::workspaces::transfer::selector::bands;

#[test]
fn base_tokens_match_the_approved_palette() {
    assert_eq!(SUISEI_DARK, Color32::from_rgb(0x00, 0x0F, 0x30));
    assert_eq!(SUISEI_MAIN, Color32::from_rgb(0x00, 0x3C, 0xFF));
    assert_eq!(SUISEI_LIGHT, Color32::from_rgb(0xCD, 0xDE, 0xE5));
    assert_eq!(KANADE_GOLD_PASTEL, Color32::from_rgb(0xF8, 0xE7, 0xB9));
    assert_eq!(RIRIKA_PINK_PASTEL, Color32::from_rgb(0xDF, 0x83, 0xA7));
    assert_eq!(RADEN_GREEN_PASTEL, Color32::from_rgb(0x9B, 0xE5, 0xD4));
    assert_eq!(UTILITY_WHITE, Color32::from_rgb(0xFF, 0xFF, 0xFF));
    assert_eq!(UTILITY_BLACK, Color32::from_rgb(0x00, 0x00, 0x00));
    assert_eq!(UTILITY_CHARCOAL, Color32::from_rgb(0x60, 0x60, 0x60));
    assert_eq!(UTILITY_GRAY, Color32::from_rgb(0x88, 0x88, 0x88));
}

#[test]
fn app_tokens_use_only_approved_palette_groups() {
    let t = tokens();
    assert_eq!(INK_RAISED, UTILITY_BLACK);
    assert_eq!(t.ink, SUISEI_DARK);
    assert_eq!(t.primary, SUISEI_MAIN);
    assert_eq!(t.green, RADEN_GREEN_PASTEL);
    assert_eq!(t.yellow, KANADE_GOLD_PASTEL);
    assert_eq!(t.red, RIRIKA_PINK_PASTEL);
    assert_eq!(t.bg, SUISEI_LIGHT);
    assert_eq!(t.border, SUISEI_LIGHT);
    assert_eq!(t.surface, UTILITY_WHITE);
    assert_eq!(t.surface_muted, SUISEI_LIGHT);
    assert_eq!(t.ink_soft, UTILITY_CHARCOAL);
}

#[test]
fn muted_text_meets_normal_text_contrast_on_app_surfaces() {
    fn luminance(color: Color32) -> f64 {
        let linear = |channel: u8| {
            let value = f64::from(channel) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b())
    }

    let t = tokens();
    let text = luminance(t.ink_soft);
    for surface in [t.surface, t.surface_muted] {
        let background = luminance(surface);
        let contrast = (text.max(background) + 0.05) / (text.min(background) + 0.05);
        assert!(contrast >= 4.5, "muted text contrast was {contrast}");
    }
}

#[test]
fn utility_alpha_helpers_wrap_the_egui_constructors() {
    assert_eq!(utility_white_alpha(48), Color32::from_white_alpha(48));
    assert_eq!(utility_black_alpha(15), Color32::from_black_alpha(15));
}

#[test]
fn graph_colors_stay_in_studio_stellar() {
    assert_eq!(
        bands::forward_fill(),
        token_alpha(SUISEI_MAIN, bands::FORWARD_FILL_ALPHA)
    );
    assert_eq!(
        bands::backward_fill(),
        token_alpha(SUISEI_DARK, bands::BACKWARD_FILL_ALPHA)
    );
}

#[test]
fn semantic_badges_use_regloss_accents_with_studio_text() {
    assert_eq!(
        semantic_badge_colors(BadgeTone::Ok),
        (RADEN_GREEN_PASTEL, RADEN_GREEN_PASTEL, SUISEI_DARK)
    );
    assert_eq!(
        semantic_badge_colors(BadgeTone::Warning),
        (KANADE_GOLD_PASTEL, KANADE_GOLD_PASTEL, SUISEI_DARK)
    );
    assert_eq!(
        semantic_badge_colors(BadgeTone::Error),
        (RIRIKA_PINK_PASTEL, RIRIKA_PINK_PASTEL, SUISEI_DARK)
    );
}
