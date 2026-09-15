use eframe::egui::{self, pos2, Rect};
use egui_kittest::{kittest::Queryable, Harness};
use paramex_gui::layout::{
    ShellRects, BODY_GAP, LEFT_MAX_WIDTH, LEFT_WIDTH, PAGE_PAD_X, PAGE_PAD_Y, RIGHT_MAX_WIDTH,
    RIGHT_WIDTH, TOP_BAR_HEIGHT,
};
use paramex_gui::workspaces::tlm::layout as tlm_layout;

fn approx(a: f32, b: f32) {
    crate::common::assert_same_raster_edge("layout contract", a, b, 1.0);
}

#[test]
fn tlm_layout_constants_mirror_the_transfer_shell() {
    use paramex_gui::layout;
    // The left column's DATA+ANALYSIS top slot plus the GROUPS bottom band must
    // fit the 800px reference body.
    let body_h = 800.0 - layout::TOP_BAR_HEIGHT - 2.0 * layout::PAGE_PAD_Y;
    // Left column = DATA (flex inside top slot) + ANALYSIS (fixed) + GROUPS (shared bottom band).
    let fixed =
        tlm_layout::TLM_DATA_CARD_HEIGHT + tlm_layout::TLM_ANALYSIS_HEIGHT + 2.0 * layout::CARD_GAP;
    assert!(
        fixed + tlm_layout::TLM_GROUPS_MIN_HEIGHT <= body_h,
        "TLM left column ({}) overflows the 800px reference body ({body_h})",
        fixed + tlm_layout::TLM_GROUPS_MIN_HEIGHT,
    );
    // Right column = FILES (flex with a floor) + the shared bottom band.
    assert!(
        layout::FILES_MIN_HEIGHT + layout::CARD_GAP + layout::SELECTED_METRICS_HEIGHT <= body_h,
        "TLM right column ({}) overflows the 800px reference body ({body_h})",
        layout::FILES_MIN_HEIGHT + layout::CARD_GAP + layout::SELECTED_METRICS_HEIGHT,
    );
    let top_slot_h = layout::fixed_bottom_stack(body_h, layout::SELECTED_METRICS_HEIGHT).top;
    assert!(
        fixed - layout::CARD_GAP <= top_slot_h,
        "TLM DATA+ANALYSIS should fit before the shared GROUPS bottom band"
    );
}

#[test]
fn paired_action_cards_keep_input_stacks_compact_enough() {
    let tlm_data_h = tlm_layout::TLM_DATA_CARD_HEIGHT;

    assert!(
        tlm_data_h <= 166.0,
        "paired DATA actions should keep the TLM top slot from crowding ANALYSIS"
    );
}

#[test]
fn fixed_bottom_stack_preserves_the_page_band_math() {
    use paramex_gui::layout;

    let body_h = 800.0 - layout::TOP_BAR_HEIGHT - 2.0 * layout::PAGE_PAD_Y;
    let stack = layout::fixed_bottom_stack(body_h, layout::SELECTED_METRICS_HEIGHT);

    approx(stack.bottom, layout::SELECTED_METRICS_HEIGHT);
    approx(
        stack.top,
        body_h - layout::SELECTED_METRICS_HEIGHT - layout::CARD_GAP,
    );
}

#[test]
fn tlm_load_error_state_fits_the_reference_input_slot() {
    use paramex_gui::layout;

    let body_h = 800.0 - layout::TOP_BAR_HEIGHT - 2.0 * layout::PAGE_PAD_Y;
    let slot_h = layout::fixed_bottom_stack(body_h, layout::SELECTED_METRICS_HEIGHT).top;
    let content_h =
        tlm_layout::TLM_DATA_CARD_HEIGHT + layout::CARD_GAP + tlm_layout::TLM_ANALYSIS_HEIGHT;
    assert!(
        content_h < slot_h,
        "tightened DATA+ANALYSIS stack should fit inside the shared top slot"
    );

    let screenshot_body_h = 759.0 - layout::TOP_BAR_HEIGHT - 2.0 * layout::PAGE_PAD_Y;
    let screenshot_slot_h =
        layout::fixed_bottom_stack(screenshot_body_h, layout::SELECTED_METRICS_HEIGHT).top;
    assert!(
        screenshot_slot_h >= tlm_layout::TLM_DATA_CARD_HEIGHT + layout::CARD_GAP + 129.0,
        "screenshot-scale TLM input stack should fit without scrolling the whole control stack"
    );
}

#[test]
fn fixed_bottom_stack_keeps_card_positions_data_independent() {
    use paramex_gui::layout;

    let empty = layout::fixed_bottom_stack(720.0, layout::SELECTED_METRICS_HEIGHT);
    let loaded = layout::fixed_bottom_stack(720.0, layout::SELECTED_METRICS_HEIGHT);
    assert_eq!(empty, loaded);
}

#[test]
fn show_card_stack_renders_top_gap_and_bottom_slots() {
    use paramex_gui::layout::{self, StackHeights, StackSlot};

    let stack = StackHeights {
        top: 40.0,
        bottom: 30.0,
    };
    let mut harness = Harness::builder()
        .with_size(egui::vec2(180.0, 140.0))
        .build_ui(|ui| {
            layout::show_card_stack(ui, stack, |ui, slot| match slot {
                StackSlot::Top => {
                    ui.set_min_height(stack.top);
                    ui.label("TOP SLOT");
                }
                StackSlot::Bottom => {
                    ui.set_min_height(stack.bottom);
                    ui.label("BOTTOM SLOT");
                }
            });
        });
    harness.run();

    let top = harness.get_by_label("TOP SLOT").rect();
    let bottom = harness.get_by_label("BOTTOM SLOT").rect();
    crate::common::assert_same_raster_edge(
        "bottom slot seam after top allocation and card gap",
        bottom.top(),
        top.top() + stack.top + layout::CARD_GAP,
        harness.ctx.pixels_per_point(),
    );
}

#[test]
fn show_in_rect_places_content_inside_requested_rect() {
    use paramex_gui::layout;

    let rect = Rect::from_min_max(pos2(24.0, 32.0), pos2(140.0, 96.0));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(180.0, 140.0))
        .build_ui(|ui| {
            layout::show_in_rect(ui, "test_rect", rect, |ui| {
                ui.label("IN RECT");
            });
        });
    harness.run();

    let label = harness.get_by_label("IN RECT").rect();
    assert!(
        label.left() >= rect.left() && label.top() >= rect.top(),
        "content should start inside the target rect: rect={rect:?}, label={label:?}"
    );
    assert!(
        label.right() <= rect.right() && label.bottom() <= rect.bottom(),
        "content should stay inside the target rect: rect={rect:?}, label={label:?}"
    );
}

fn shell_at(w: f32, h: f32) -> ShellRects {
    ShellRects::from_content(Rect::from_min_max(pos2(0.0, 0.0), pos2(w, h)))
}

/// At the reference width the responsive math reproduces the original fixed
/// columns exactly, so the committed 1280×800 snapshots do not churn.
#[test]
fn reference_window_reproduces_the_base_bento_grid() {
    let content = Rect::from_min_max(pos2(0.0, 0.0), pos2(1280.0, 800.0));
    let shell = ShellRects::from_content(content);

    approx(shell.top.height(), TOP_BAR_HEIGHT);
    approx(shell.body.left() - content.left(), PAGE_PAD_X);
    approx(content.right() - shell.body.right(), PAGE_PAD_X);
    approx(shell.body.top() - shell.top.bottom(), PAGE_PAD_Y);
    approx(content.bottom() - shell.body.bottom(), PAGE_PAD_Y);

    approx(shell.left.width(), LEFT_WIDTH);
    approx(shell.right.width(), RIGHT_WIDTH);
    approx(shell.center.left() - shell.left.right(), BODY_GAP);
    approx(shell.right.left() - shell.center.right(), BODY_GAP);
    assert!(
        shell.center.width() >= 520.0,
        "center column is too narrow: {}",
        shell.center.width()
    );
}

/// Wider window: the side columns grow past their base but never past their cap,
/// and the center (graphs) absorbs the remaining surplus, so it grows the most.
#[test]
fn columns_grow_then_cap_so_the_center_absorbs_surplus() {
    let base = shell_at(1280.0, 800.0);
    let wide = shell_at(1920.0, 1080.0);

    assert!(wide.left.width() > base.left.width(), "left did not grow");
    assert!(
        wide.right.width() > base.right.width(),
        "right did not grow"
    );
    assert!(wide.left.width() <= LEFT_MAX_WIDTH, "left exceeded cap");
    assert!(wide.right.width() <= RIGHT_MAX_WIDTH, "right exceeded cap");

    let center_growth = wide.center.width() - base.center.width();
    let left_growth = wide.left.width() - base.left.width();
    assert!(
        center_growth > left_growth,
        "center should absorb the surplus (center +{center_growth}, left +{left_growth})"
    );
}

/// Across the supported size range the columns never overlap, share the same top
/// and bottom, keep the gutter, and leave the center usable.
#[test]
fn no_overlap_and_shared_edges_across_window_sizes() {
    for (w, h) in [
        (1280.0, 800.0),
        (1500.0, 900.0),
        (1920.0, 1080.0),
        (2560.0, 1440.0),
    ] {
        let s = shell_at(w, h);
        approx(s.center.left() - s.left.right(), BODY_GAP);
        approx(s.right.left() - s.center.right(), BODY_GAP);
        approx(s.left.top(), s.center.top());
        approx(s.center.top(), s.right.top());
        approx(s.left.bottom(), s.center.bottom());
        approx(s.center.bottom(), s.right.bottom());
        assert!(
            s.center.width() >= 520.0,
            "center column too narrow at {w}x{h}: {}",
            s.center.width()
        );
    }
}
