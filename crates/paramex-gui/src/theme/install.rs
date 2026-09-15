//! Runtime egui theme installation and viewport icon decoding.

use eframe::egui::{self, Color32, CornerRadius, Stroke, Visuals};

use super::{
    radius, soft_shadow, token_alpha, tokens, ACTIVE_WIDGET_FILL_ALPHA, FAINT_BG_ALPHA,
    SELECTION_FILL_ALPHA,
};

/// Install fonts + visuals once (call from `ParamExApp::new` via `cc.egui_ctx`).
pub fn install(ctx: &egui::Context) {
    install_fonts(ctx);
    install_visuals(ctx);

    // egui's debug-only "rect changed id" overlay (default-on in debug builds) paints
    // red warning boxes when a widget keeps its screen rect but its Id changes between
    // frames. Several panels intentionally do this on a file switch — the fit-window
    // range strips key their Id on the selected file, and the results table shifts rows
    // when a single↔double-sweep selection changes the row count — so the overlay fires
    // as a one-frame red flash. It is a false positive for those intentional patterns
    // and never appears in release (the whole `DebugOptions` is `#[cfg(debug_assertions)]`).
    // Silence it so debug builds match release.
    #[cfg(debug_assertions)]
    ctx.all_styles_mut(|style| style.debug.warn_if_rect_changes_id = false);

    // egui paints a fade gradient over a ScrollArea's overflowing edges
    // (`spacing.scroll.fade`: strength 0.5 over 20px). Every fixed-height card
    // with a scrollable body therefore showed its partially clipped bottom row
    // permanently "blurred" at rest — the results table, the file list, and the
    // TLM tables all sit in such cards. Scrollbars are this app's scroll
    // affordance; partially clipped rows must cut crisply.
    ctx.all_styles_mut(|style| style.spacing.scroll.fade.strength = 0.0);

    // …but the default floating scroll style hides the handle entirely at rest
    // (`dormant_handle_opacity: 0`), so that affordance was invisible until the
    // pointer reached the scroll area. Keep the floating behavior (hover still
    // expands/brightens the bar) and give the dormant handle a quiet presence —
    // the thin ink line at the card edge says "this scrolls" at a glance.
    ctx.all_styles_mut(|style| style.spacing.scroll.dormant_handle_opacity = 0.35);
}

/// Map tokens onto a light `Visuals`: white surfaces / ink text, the light blue-grey
/// `bg` behind cards, brand-blue selection, and rounded widgets with a brand hover so
/// buttons/fields read as part of the design (not raw egui defaults).
fn install_visuals(ctx: &egui::Context) {
    let t = tokens();
    let mut v = Visuals::light();
    v.window_fill = t.surface;
    v.panel_fill = t.bg; // light blue-grey body; panel content sits on white card frames
    v.extreme_bg_color = t.surface; // text-edit / plot interiors stay white
    v.override_text_color = Some(t.ink);
    v.hyperlink_color = t.primary;
    v.selection.bg_fill = token_alpha(t.primary, SELECTION_FILL_ALPHA);
    v.selection.stroke = Stroke::new(1.0_f32, t.primary);
    // Stripe color for `.striped(true)` grids/tables. The light-theme default is
    // `from_additive_luminance(5)`, which adds onto the already-bright card
    // surface and renders as nothing. Use a barely cool tint over white; in the
    // quiet tables the stripes carry the row structure because rows are boxless.
    v.faint_bg_color = token_alpha(t.bg, FAINT_BG_ALPHA);

    let r = CornerRadius::same(radius::WIDGET);
    let ink = Stroke::new(1.0_f32, t.ink);

    // All four widget states share `fg_stroke = ink` and `corner_radius = r`, set a
    // flat `fill` (used for both bg + weak_bg), and a hairline `bg_stroke` of the
    // given color. (noninteractive == inactive; hovered/active differ only in
    // fill + bg_stroke color.)
    let apply = |w: &mut egui::style::WidgetVisuals, fill: Color32, stroke_color: Color32| {
        w.bg_fill = fill;
        w.weak_bg_fill = fill;
        w.bg_stroke = Stroke::new(1.0_f32, stroke_color);
        w.fg_stroke = ink;
        w.corner_radius = r;
    };

    // Idle labels / separators / panel chrome.
    apply(&mut v.widgets.noninteractive, t.surface, t.border);
    // Idle interactive (buttons, text edits): white fill, ink text, hairline border.
    apply(&mut v.widgets.inactive, t.surface, t.border);
    // Hover: a light blue-grey wash + brand-blue outline (clear, not jarring).
    apply(&mut v.widgets.hovered, t.surface_muted, t.primary);
    // Pressed/active: a soft brand tint.
    apply(
        &mut v.widgets.active,
        token_alpha(t.primary, ACTIVE_WIDGET_FILL_ALPHA),
        t.primary,
    );

    v.widgets.open.corner_radius = r;

    // Tooltips, popups, and menus carry the card surface language (card
    // corners, `border` hairline, the shared soft shadow) instead of egui's
    // grey-stroked, hard-shadowed defaults — they appear over cards (TLM
    // cell-overflow hover, file-status hover) and should float at the same
    // elevation. In egui 0.34 `Frame::popup`/`Frame::menu` (tooltips included)
    // read `menu_corner_radius` + `window_stroke` + `popup_shadow`; the
    // `window_corner_radius`/`window_shadow` pair styles only `egui::Window`,
    // which this app never creates, so they are deliberately left alone.
    v.menu_corner_radius = CornerRadius::same(radius::CARD);
    v.window_stroke = Stroke::new(1.0_f32, t.border);
    v.popup_shadow = soft_shadow();
    // Brand-blue caret in text fields (the egui default is off-palette).
    v.text_cursor.stroke = Stroke::new(2.0_f32, t.primary);

    ctx.set_visuals(v);
}

/// Install a clean proportional UI font with a real bold face. On Windows this
/// loads the system **Segoe UI** (Regular + Bold), a humanist sans that ships
/// with every supported Windows, and registers a `Name("bold")` family so
/// headers/buttons can be bold. (A cross-platform build would bundle a font via
/// `include_bytes!`.) The `"bold"` family is always registered (aliased to the
/// proportional stack if no bold face is found) so `ui_kit::bold_family()` never
/// references a missing family.
fn install_fonts(ctx: &egui::Context) {
    use eframe::egui::{FontData, FontDefinitions, FontFamily};
    use std::sync::Arc;

    let mut fonts = FontDefinitions::default();

    // Resolve the fonts dir from %WINDIR% like every other OS path in the app
    // (`platform` env-resolves LOCALAPPDATA/USERPROFILE); a non-C: SystemRoot
    // would otherwise silently lose Segoe and ship egui's default font.
    let fonts_dir = std::env::var_os("WINDIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("C:/Windows"))
        .join("Fonts");

    if let Ok(regular) = std::fs::read(fonts_dir.join("segoeui.ttf")) {
        fonts
            .font_data
            .insert("ui".to_owned(), Arc::new(FontData::from_owned(regular)));
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "ui".to_owned());
    }

    let bold_list = if let Ok(bold) = std::fs::read(fonts_dir.join("segoeuib.ttf")) {
        fonts
            .font_data
            .insert("ui-bold".to_owned(), Arc::new(FontData::from_owned(bold)));
        // Reference "ui" only when the regular face actually loaded: the two
        // reads are independent, and a family naming unloaded font data makes
        // epaint panic on the first text layout (release aborts at startup).
        let mut list = vec!["ui-bold".to_owned()];
        if fonts.font_data.contains_key("ui") {
            list.push("ui".to_owned());
        }
        list
    } else {
        // No bold face available: alias "bold" to the current proportional stack.
        fonts
            .families
            .get(&FontFamily::Proportional)
            .cloned()
            .unwrap_or_default()
    };
    fonts
        .families
        .insert(FontFamily::Name("bold".into()), bold_list);

    ctx.set_fonts(fonts);
    // Mark the `bold` family as registered so `ui_kit::bold_family` uses it (panels
    // rendered in tests without the theme fall back to Proportional instead).
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(crate::ui_kit::BOLD_READY_FLAG), true));
}

/// Decode the bundled window icon PNG into an `egui::IconData` for the viewport.
pub fn window_icon() -> Option<egui::IconData> {
    eframe::icon_data::from_png_bytes(include_bytes!("../../assets/icon.png")).ok()
}
