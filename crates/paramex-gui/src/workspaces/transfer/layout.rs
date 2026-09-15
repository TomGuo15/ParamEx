//! Transfer page layout policy layered on the shared shell/card-stack geometry.

use eframe::egui;

/// COX card height while the stack estimator shows only its layer rows and actions.
pub const COX_STACK_SETUP_HEIGHT: f32 = 244.0;
/// COX card height once an estimate is shown: adds the estimate label and the
/// "Use Estimated" action below the layer rows.
pub const COX_ESTIMATED_SETUP_HEIGHT: f32 = 278.0;

/// A plot pair stacks vertically once the body is this many times taller than
/// it is wide; below that the two plots sit side by side.
const TALL_PLOT_PAIR_ASPECT: f32 = 1.5;

/// Minimum y-axis gutter shared by every Transfer plot so the plot areas of the
/// selector graphs and the output pair start at the same x inside their cards.
pub(crate) const Y_AXIS_MIN_THICKNESS: f32 = 58.0;
/// X-axis tick-label spacing range shared by every Transfer plot.
pub(crate) fn x_axis_label_spacing() -> egui::Rangef {
    egui::Rangef::new(24.0, 40.0)
}

pub(crate) fn plot_pair_should_stack(size: egui::Vec2) -> bool {
    size.x > 0.0 && size.y >= size.x * TALL_PLOT_PAIR_ASPECT
}

/// Split `height` into two equal stacked plot slots separated by one card gap
/// and render `add` into each; the second slot absorbs any rounding remainder.
pub(crate) fn show_stacked_plot_pair(
    ui: &mut egui::Ui,
    id_salt: &'static str,
    height: f32,
    mut add: impl FnMut(&mut egui::Ui, usize),
) {
    let height = height.clamp(0.0, ui.available_height().max(0.0));
    let size = egui::vec2(ui.available_width().max(0.0), height);
    let (host, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let slot_h = ((host.height() - crate::layout::CARD_GAP).max(0.0) * 0.5).floor();

    for index in 0..2 {
        let top = host.top() + index as f32 * (slot_h + crate::layout::CARD_GAP);
        let bottom = if index == 1 {
            host.bottom()
        } else {
            top + slot_h
        };
        let rect = egui::Rect::from_min_max(
            egui::pos2(host.left(), top),
            egui::pos2(host.right(), bottom),
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .id_salt((id_salt, index))
                .max_rect(rect)
                .layout(*ui.layout()),
        );
        child.set_min_size(rect.size());
        child.set_clip_rect(rect);
        add(&mut child, index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plot_pair_breakpoint_only_stacks_tall_narrow_bodies() {
        assert!(!plot_pair_should_stack(egui::vec2(638.0, 400.0)));
        assert!(!plot_pair_should_stack(egui::vec2(1100.0, 800.0)));
        assert!(plot_pair_should_stack(egui::vec2(638.0, 1075.0)));
    }
}
