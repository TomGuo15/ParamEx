//! Interaction guard for the ANALYSIS V_G picker strip: dragging the strip band
//! must re-analyze at a different measured V_G (the strip is custom-painted, so
//! nothing but a real pointer drive proves it responds).

use crate::common::{self, loaded_tlm_app as seed_tlm_app};
use eframe::egui;
use egui_kittest::{
    kittest::{NodeT, Queryable},
    Harness,
};
use egui_notify::Toasts;
use paramex_gui::format_ui::DASH;
use paramex_gui::state::EditBuffers;
use paramex_gui::workspaces::tlm::panels::analysis::{self, commit_vg, vg_slider_index};
use paramex_gui::workspaces::tlm::state::TlmState;

struct TlmAnalysisHarnessApp {
    tlm: TlmState,
    edits: EditBuffers,
    toasts: Toasts,
    size: egui::Vec2,
}

impl eframe::App for TlmAnalysisHarnessApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.allocate_ui(self.size, |ui| {
            analysis::show(ui, &mut self.tlm, &mut self.edits, &mut self.toasts);
        });
    }
}

#[test]
fn vg_commit_accepts_only_finite_numbers() {
    assert_eq!(commit_vg("-1.25"), Ok(-1.25));
    assert_eq!(commit_vg(" 2 "), Ok(2.0));
    assert!(commit_vg("abc").is_err());
    assert!(commit_vg("inf").is_err());
    assert!(commit_vg("").is_err());
}

#[test]
fn vg_slider_index_selects_nearest_measured_value() {
    let values = [-3.0, -1.0, 0.5, 2.0];
    assert_eq!(vg_slider_index(&values, -1.2), 1);
    assert_eq!(vg_slider_index(&values, 1.6), 3);
    assert_eq!(vg_slider_index(&[], 1.6), 0);
}

#[test]
fn empty_tlm_analysis_readout_shows_dash_with_unit() {
    let state = TlmAnalysisHarnessApp {
        tlm: TlmState::default(),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
        size: egui::Vec2::new(320.0, 190.0),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(360.0, 230.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    harness.get_by_label(DASH);
    harness.get_by_label("V");
}

#[test]
fn empty_tlm_analysis_strip_is_inert_gray_rail_only() {
    let state = TlmAnalysisHarnessApp {
        tlm: TlmState::default(),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
        size: egui::Vec2::new(320.0, 190.0),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(360.0, 230.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    let (slider_rect, slider_is_disabled) = {
        let slider = harness.get_by_role(egui::accesskit::Role::Slider);
        (slider.rect(), slider.accesskit_node().is_disabled())
    };
    let image = common::render(&mut harness);
    let blue_thumb_pixels = image
        .enumerate_pixels()
        .filter(|(x, y, pixel)| {
            slider_rect.contains(egui::pos2(*x as f32 + 0.5, *y as f32 + 0.5))
                && pixel[3] > 200
                && pixel[2] > 180
                && pixel[2].saturating_sub(pixel[0]) > 50
                && pixel[2].saturating_sub(pixel[1]) > 30
        })
        .count();

    assert_eq!(
        blue_thumb_pixels, 0,
        "empty TLM ANALYSIS should match Transfer's disabled strip: gray rail only, no primary-blue thumb"
    );
    assert!(
        slider_is_disabled,
        "empty TLM ANALYSIS should expose the stable V_G slider node as disabled"
    );
}

#[test]
fn vg_strip_exposes_accessible_slider_contract() {
    let harness = common::app_harness(seed_tlm_app());

    let slider = harness.get_by_role(egui::accesskit::Role::Slider);
    let node = slider.accesskit_node();
    let picker = harness
        .state()
        .tlm()
        .vg_picker()
        .expect("loaded V_G picker");

    assert_eq!(node.label().as_deref(), Some("Gate voltage VG (V)"));
    assert_eq!(node.numeric_value(), Some(picker.selected_vg));
    assert_eq!(
        node.min_numeric_value(),
        picker.vg_values.iter().copied().reduce(f64::min)
    );
    assert_eq!(
        node.max_numeric_value(),
        picker.vg_values.iter().copied().reduce(f64::max)
    );
    assert!(node
        .data()
        .supports_action(egui::accesskit::Action::SetValue));
}

#[test]
fn focused_vg_strip_arrow_key_steps_one_measured_voltage() {
    let mut harness = common::app_harness(seed_tlm_app());

    let picker = harness
        .state()
        .tlm()
        .vg_picker()
        .expect("loaded V_G picker");
    let values = picker.vg_values.to_vec();
    let before_idx = vg_slider_index(&values, picker.selected_vg);
    let (key, expected_idx) = if before_idx + 1 < values.len() {
        (egui::Key::ArrowRight, before_idx + 1)
    } else {
        (egui::Key::ArrowLeft, before_idx - 1)
    };

    let unfocused = common::render(&mut harness);
    let strip_rect = {
        let slider = harness.get_by_role(egui::accesskit::Role::Slider);
        slider.focus();
        slider.rect().expand(4.0)
    };
    harness.run();
    let focused = common::render(&mut harness);
    let mut changed_pixels = 0;
    for y in
        strip_rect.top().max(0.0) as u32..strip_rect.bottom().min(focused.height() as f32) as u32
    {
        for x in
            strip_rect.left().max(0.0) as u32..strip_rect.right().min(focused.width() as f32) as u32
        {
            changed_pixels += usize::from(unfocused.get_pixel(x, y) != focused.get_pixel(x, y));
        }
    }
    assert!(
        changed_pixels > 0,
        "focused V_G strip should paint a visible focus halo"
    );
    harness.key_press(key);
    harness.run();
    harness.run();

    assert_eq!(
        harness.state().tlm().selected_vg(),
        Some(values[expected_idx])
    );
}

#[test]
fn vg_strip_accesskit_set_value_snaps_to_a_measured_voltage() {
    let mut harness = common::app_harness(seed_tlm_app());

    let picker = harness
        .state()
        .tlm()
        .vg_picker()
        .expect("loaded V_G picker");
    let values = picker.vg_values.to_vec();
    let before_idx = vg_slider_index(&values, picker.selected_vg);
    let target_idx = if before_idx == 0 { values.len() - 1 } else { 0 };
    let neighbor_idx = if target_idx == 0 { 1 } else { target_idx - 1 };
    let requested = 0.8 * values[target_idx] + 0.2 * values[neighbor_idx];
    let expected_idx = vg_slider_index(&values, requested);
    assert_ne!(expected_idx, before_idx);

    let (target_node, target_tree) = harness
        .get_by_role(egui::accesskit::Role::Slider)
        .accesskit_node()
        .locate();
    harness
        .input_mut()
        .events
        .push(egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::SetValue,
                target_node,
                target_tree,
                data: Some(egui::accesskit::ActionData::NumericValue(requested)),
            },
        ));
    harness.run();
    harness.run();

    assert_eq!(
        harness.state().tlm().selected_vg(),
        Some(values[expected_idx])
    );
}

#[test]
fn dragging_the_vg_strip_changes_the_selected_vg() {
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1280.0, 800.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            seed_tlm_app()
        });
    harness.run();
    harness.run();

    let before = harness
        .state()
        .tlm()
        .selected_vg()
        .expect("a V_G is selected");

    let vg = harness
        .get_all_by_role(egui::accesskit::Role::TextInput)
        .max_by(|a, b| a.rect().center().y.total_cmp(&b.rect().center().y))
        .expect("a V_G text input is present");
    let y = vg.rect().top() - 21.0;
    let mut moved = false;
    for (from_x, to_x) in [(45.0_f32, 170.0), (170.0, 45.0)] {
        let from = egui::pos2(from_x, y);
        let to = egui::pos2(to_x, y);
        harness
            .input_mut()
            .events
            .push(egui::Event::PointerMoved(from));
        harness.run();
        harness.input_mut().events.push(egui::Event::PointerButton {
            pos: from,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        });
        harness.run();
        harness
            .input_mut()
            .events
            .push(egui::Event::PointerMoved(to));
        harness.run();
        harness.input_mut().events.push(egui::Event::PointerButton {
            pos: to,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        });
        harness.run();
        harness.run();

        if harness.state().tlm().selected_vg() != Some(before) {
            moved = true;
            break;
        }
    }
    assert!(
        moved,
        "dragging across the V_G strip should re-analyze at a different measured V_G \
         (stayed at {before})"
    );
}

#[test]
fn vg_strip_click_wins_over_a_focused_vg_field() {
    // Clicking the V_G strip while the numeric field has focus moves focus away from
    // the field. The strip selection must win over the field's stale lost-focus value.
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1280.0, 800.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            seed_tlm_app()
        });
    harness.run();
    harness.run();

    let before = harness
        .state()
        .tlm()
        .selected_vg()
        .expect("a V_G is selected");

    let mut moved = false;
    // Try a left and a right index so at least one differs from the current selection.
    for x in [168.0_f32, 50.0] {
        // Focus the V_G field, then deliver a single COALESCED click (press+release at
        // one point, same frame) on the strip at a different index. vg_strip commits on
        // clicked() (idx <- click x), and the click steals the field's focus that SAME
        // frame, so the field's stale lost_focus commit competes with the strip's pick:
        // the bug's exact trigger. (A multi-frame drag loses field focus on the press,
        // before the strip's release-pick, so the strip would win even unfixed; and the
        // field can't be focused mid-drag — hence the coalesced single click.)
        //
        // The settings-row input is pinned to the right card edge, so a position click
        // is unreliable -> focus by TextInput role; the V_G field is the LOWEST text
        // input on the page (the DATA card's fallback-V_D field is above it). The strip
        // sits one control row above that field.
        let vg = harness
            .get_all_by_role(egui::accesskit::Role::TextInput)
            .max_by(|a, b| a.rect().center().y.total_cmp(&b.rect().center().y))
            .expect("a V_G text input is present");
        let at = egui::pos2(x, vg.rect().top() - 21.0);
        vg.focus();
        harness.run();

        {
            let events = &mut harness.input_mut().events;
            events.push(egui::Event::PointerMoved(at));
            events.push(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            });
            events.push(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            });
        }
        harness.run();
        harness.run();

        if harness.state().tlm().selected_vg() != Some(before) {
            moved = true;
            break;
        }
    }
    assert!(
        moved,
        "the V_G strip pick must win over a focused V_G field's stale commit \
         (stayed at {before} -> the field re-pinned the old V_G)"
    );
}
