//! `theme::install` contract: the egui style overrides the theme must apply so
//! debug builds match release and scrollable cards read correctly at rest.
//!
//! The file name deliberately avoids `setup`/`install`/`update`: Windows
//! installer detection auto-elevates executables whose names carry those words,
//! and a non-elevated runner then fails with os error 740 "requires elevation".
//! Today this module is compiled into the `gui_shared` test binary, so the
//! file name never reaches an exe name; keeping the convention means promoting
//! it to a standalone test file can never trip that heuristic.

// egui's debug-only `warn_if_rect_changes_id` overlay paints red warning boxes
// when a widget keeps its screen rect but changes Id between frames. Several
// panels intentionally do this on a file switch (the fit-window range strips
// key their Id on the selected file; the results table shifts rows when a
// single↔double-sweep selection changes the row count), so the overlay fires
// as a one-frame red flash. It is a false positive and never appears in release
// (the whole `DebugOptions` is `#[cfg(debug_assertions)]`), so `theme::install`
// silences it. The `Style::debug` field only exists in debug builds, so this
// guard is debug-only.
#[cfg(debug_assertions)]
#[test]
fn install_disables_rect_id_change_overlay() {
    let ctx = egui::Context::default();
    paramex_gui::theme::install(&ctx);
    assert!(
        !ctx.global_style().debug.warn_if_rect_changes_id,
        "theme::install must disable egui's debug rect-id-change overlay \
         (otherwise red boxes flash on file switch)"
    );
}

// egui paints a fade gradient at a ScrollArea's overflowing edges
// (`paint_fade_areas`, `spacing.scroll.fade`, default strength 0.5 over 20px).
// Every fixed-height card with a scrollable body therefore showed its partially
// clipped bottom row permanently "blurred" at rest (results table, file list,
// TLM tables). Scrollbars are this app's scroll affordance; the fade must be off.
#[test]
fn install_disables_scroll_edge_fade() {
    let ctx = egui::Context::default();
    paramex_gui::theme::install(&ctx);
    assert_eq!(
        ctx.global_style().spacing.scroll.fade.strength,
        0.0,
        "theme::install must disable egui's scroll-edge fade \
         (otherwise the bottom table row renders permanently faded)"
    );
}

// The default floating scroll style hides the handle entirely at rest
// (`dormant_handle_opacity: 0.0`), so the remaining scroll affordance
// was invisible until hover. `theme::install` must keep a quiet dormant handle.
#[test]
fn install_keeps_dormant_scroll_handle_visible() {
    let ctx = egui::Context::default();
    paramex_gui::theme::install(&ctx);
    assert_eq!(
        ctx.global_style().spacing.scroll.dormant_handle_opacity,
        0.35,
        "theme::install must keep the dormant scrollbar handle visible \
         (otherwise nothing says \"this scrolls\" at rest)"
    );
}

#[test]
fn border_uses_studio_stellar_light() {
    let t = paramex_gui::theme::tokens();
    assert_eq!(t.border, paramex_gui::theme::SUISEI_LIGHT);
}

#[test]
fn accent_soft_is_low_alpha_primary() {
    let t = paramex_gui::theme::tokens();
    assert_eq!(
        t.accent_soft,
        paramex_gui::theme::token_alpha(t.primary, paramex_gui::theme::ACCENT_SOFT_ALPHA)
    );
}
