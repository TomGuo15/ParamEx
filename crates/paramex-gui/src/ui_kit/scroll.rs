//! The one vertical scroll recipe for list/table bodies whose height is owned
//! by the card slot math.

use eframe::egui;

/// A vertical scroll area capped at `max_height` that never auto-shrinks.
pub fn scroll_body<R>(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    max_height: f32,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    egui::ScrollArea::vertical()
        .id_salt(id_salt)
        .auto_shrink([false, false])
        .max_height(max_height.max(0.0))
        // egui floors a scrollable axis at `min_scrolled_size` (64px by default),
        // which would silently override a smaller `max_height` and make the host
        // card overflow its slot. The slot math owns the height here, so let the
        // viewport shrink all the way to the cap.
        .min_scrolled_height(0.0)
        .show(ui, add)
        .inner
}
