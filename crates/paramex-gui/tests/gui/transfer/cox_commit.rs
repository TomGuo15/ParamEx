use egui_kittest::{kittest::Queryable, Harness};
use egui_notify::Toasts;
use paramex_core::transfer::{calculate_stack_cox_nf_per_cm2, Session};
use paramex_gui::state::EditBuffers;
use paramex_gui::ui_kit::CARD_INNER_MARGIN;
use paramex_gui::workspaces::transfer::panels::cox::commit_cox;
use paramex_gui::workspaces::transfer::state::{CoxUi, LayerRow, COX_ESTIMATE_PENDING_LABEL};

use crate::transfer_curve as curve;

#[test]
fn commit_cox_applies_only_positive_finite() {
    let mut session = Session::new();
    session.add_curve(curve("a.csv"));

    assert!(commit_cox(&mut session, "23.5")); // committed
    assert_eq!(session.cox_nf_per_cm2(), 23.5);

    assert!(!commit_cox(&mut session, "0")); // skipped (<= 0)
    assert!(!commit_cox(&mut session, "-1")); // skipped (<= 0)
    assert!(!commit_cox(&mut session, "abc")); // skipped (parse fail)
    assert_eq!(session.cox_nf_per_cm2(), 23.5); // unchanged after skips
}

#[test]
fn layers_data_and_estimator_nan_on_nonpositive() {
    // (3.9, 300.0) -> finite Cox; a non-positive layer -> whole result NaN.
    let mut cox = CoxUi::default();
    let good = calculate_stack_cox_nf_per_cm2(&cox.layers_data());
    assert!(good.is_finite() && good > 0.0);

    cox.add_layer(LayerRow::new("0", "10"));
    let bad = calculate_stack_cox_nf_per_cm2(&cox.layers_data());
    assert!(bad.is_nan());
}

#[test]
fn cox_layer_list_keeps_one_layer_and_adds_default_layer() {
    let mut cox = CoxUi::default();

    assert!(!cox.can_remove_layer());
    assert!(!cox.remove_layer(0));
    assert_eq!(cox.layers().len(), 1);

    cox.add_default_layer();
    assert!(cox.can_remove_layer());
    assert_eq!(cox.layers()[1].eps_text(), "3.9");
    assert_eq!(cox.layers()[1].th_text(), "10");
    assert!(cox.remove_layer(1));
    assert_eq!(cox.layers().len(), 1);
}

struct CoxHarnessApp {
    session: Session,
    cox: CoxUi,
    edits: EditBuffers,
    toasts: Toasts,
}

impl eframe::App for CoxHarnessApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.allocate_ui(egui::Vec2::new(380.0, 300.0), |ui| {
            paramex_gui::workspaces::transfer::panels::cox::show_setup(
                ui,
                &mut self.session,
                &mut self.cox,
                &mut self.edits,
                &mut self.toasts,
            );
        });
    }
}

fn sorted_text_input_rects(harness: &Harness<'_, CoxHarnessApp>) -> Vec<egui::Rect> {
    let mut inputs: Vec<_> = harness
        .get_all_by_role(egui::accesskit::Role::TextInput)
        .map(|node| node.rect())
        .collect();
    inputs.sort_by(|a, b| {
        a.center()
            .y
            .total_cmp(&b.center().y)
            .then(a.left().total_cmp(&b.left()))
    });
    inputs
}

#[test]
fn only_cox_layer_remove_action_is_hidden() {
    let state = CoxHarnessApp {
        session: Session::new(),
        cox: CoxUi::default(),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(420.0, 340.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    assert!(harness.query_by_label("Remove layer").is_none());
    assert_eq!(harness.state().cox.layers().len(), 1);
}

#[test]
fn use_estimated_cox_is_hidden_until_an_estimate_exists() {
    let state = CoxHarnessApp {
        session: Session::new(),
        cox: CoxUi::default(),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(420.0, 340.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    let initial_cox = harness.state().session.cox_nf_per_cm2();
    assert!(harness.query_by_label("Use Estimated Cox").is_none());
    assert_eq!(
        harness.state().session.cox_nf_per_cm2(),
        initial_cox,
        "hidden Use Estimated Cox must not commit a value"
    );
    assert_eq!(harness.state().cox.estimate_value(), None);
    assert_eq!(
        harness.state().cox.estimate_label(),
        COX_ESTIMATE_PENDING_LABEL
    );
}

#[test]
fn stack_estimator_pair_inputs_stay_inline_and_usable() {
    let state = CoxHarnessApp {
        session: Session::new(),
        cox: CoxUi::default(),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(420.0, 340.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    let inputs = sorted_text_input_rects(&harness);
    assert!(
        inputs.len() >= 3,
        "Cox value, epsilon, and thickness inputs should render"
    );
    let measured = inputs[0];
    let eps = inputs[1];
    let thickness = inputs[2];
    let pixels_per_point = harness.ctx.pixels_per_point();

    assert!(
        eps.left() <= measured.left() + 4.0,
        "epsilon input should not be pushed inward by a stacked label layout"
    );
    assert!(
        eps.right() < thickness.left(),
        "epsilon and thickness inputs should occupy separate paired cells"
    );
    crate::common::assert_same_raster_span(
        "epsilon/thickness field widths",
        (eps.left(), eps.right()),
        (thickness.left(), thickness.right()),
        pixels_per_point,
    );
    crate::common::assert_same_raster_edge(
        "epsilon/thickness top edge",
        eps.top(),
        thickness.top(),
        pixels_per_point,
    );
    crate::common::assert_same_raster_edge(
        "epsilon/thickness bottom edge",
        eps.bottom(),
        thickness.bottom(),
        pixels_per_point,
    );
    assert!(
        eps.width() >= 60.0 && thickness.width() >= 60.0,
        "paired inputs should keep usable field width"
    );
}

#[test]
fn cox_stack_first_layer_geometry_stays_fixed_after_add_layer() {
    let state = CoxHarnessApp {
        session: Session::new(),
        cox: CoxUi::default(),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(420.0, 340.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    let before = sorted_text_input_rects(&harness);
    assert!(
        before.len() >= 3,
        "Cox field plus one stack row should render three text inputs"
    );
    let before_eps = before[1];
    let before_thickness = before[2];

    harness.get_by_label("Add Layer").click();
    harness.run();

    let after = sorted_text_input_rects(&harness);
    assert!(
        after.len() >= 5,
        "Cox field plus two stack rows should render five text inputs"
    );
    let after_eps = after[1];
    let after_thickness = after[2];
    let pixels_per_point = harness.ctx.pixels_per_point();

    for (name, before, after) in [
        ("epsilon", before_eps, after_eps),
        ("thickness", before_thickness, after_thickness),
    ] {
        crate::common::assert_same_raster_rect(
            &format!("{name} input moved when the remove-button lane appeared"),
            before,
            after,
            pixels_per_point,
        );
    }
}

#[test]
fn cox_layer_rows_keep_single_line_height_when_remove_buttons_appear() {
    let mut cox = CoxUi::default();
    cox.add_default_layer();
    let state = CoxHarnessApp {
        session: Session::new(),
        cox,
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(420.0, 340.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    let inputs = sorted_text_input_rects(&harness);

    assert!(
        inputs.len() >= 5,
        "Cox field plus two stack rows should render five text inputs"
    );
    let first_layer_eps = inputs[1];
    let second_layer_eps = inputs[3];
    let row_step = second_layer_eps.center().y - first_layer_eps.center().y;
    assert!(
        row_step <= paramex_gui::ui_kit::BUTTON_HEIGHT + 8.0,
        "Cox stack layer rows should use one compact input row, got vertical step {row_step} from {first_layer_eps:?} to {second_layer_eps:?}"
    );
}

#[test]
fn cox_stack_layer_viewport_reserves_four_rows_before_actions() {
    let mut cox = CoxUi::default();
    for _ in 0..8 {
        cox.add_default_layer();
    }
    let state = CoxHarnessApp {
        session: Session::new(),
        cox,
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(420.0, 340.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    let inputs = sorted_text_input_rects(&harness);
    assert!(
        inputs.len() >= 5,
        "Cox field plus multiple stack rows should render text inputs"
    );
    let first_layer_eps = inputs[1];
    let second_layer_eps = inputs[3];
    let row_step = second_layer_eps.center().y - first_layer_eps.center().y;
    let add_layer = harness.get_by_label("Add Layer").rect();

    assert!(
        add_layer.top() - first_layer_eps.top() >= row_step * 4.0,
        "Cox stack should reserve four layer rows above the fixed footer actions; first row {first_layer_eps:?}, second row {second_layer_eps:?}, Add Layer {add_layer:?}"
    );
}

fn cox_harness_with_layers(extra_layers: usize) -> Harness<'static, CoxHarnessApp> {
    let mut cox = CoxUi::default();
    for _ in 0..extra_layers {
        cox.add_default_layer();
    }
    let state = CoxHarnessApp {
        session: Session::new(),
        cox,
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(420.0, 340.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();
    harness
}

#[test]
fn cox_stack_actions_keep_a_fixed_footer_for_one_or_four_rows() {
    let one_row = cox_harness_with_layers(0);
    let four_rows = cox_harness_with_layers(3);
    let one_add = one_row.get_by_label("Add Layer").rect();
    let four_add = four_rows.get_by_label("Add Layer").rect();

    crate::common::assert_same_raster_rect(
        "Cox Add Layer footer with one/four rows",
        one_add,
        four_add,
        one_row.ctx.pixels_per_point(),
    );
}

#[test]
fn cox_four_visible_rows_stay_above_the_fixed_actions() {
    let mut cox = CoxUi::default();
    for _ in 0..3 {
        cox.add_default_layer();
    }
    let state = CoxHarnessApp {
        session: Session::new(),
        cox,
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(420.0, 340.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    let inputs = sorted_text_input_rects(&harness);
    assert!(
        inputs.len() >= 9,
        "Cox field plus four stack rows should render nine text inputs"
    );
    let last_layer_thickness = inputs[8];
    let add_layer = harness.get_by_label("Add Layer").rect();

    assert!(
        last_layer_thickness.bottom() <= add_layer.top() - 4.0,
        "Cox four-row list should end above the fixed action buttons: last row {last_layer_thickness:?}, Add Layer {add_layer:?}"
    );
}

#[test]
fn cox_stack_actions_share_one_compact_row() {
    let state = CoxHarnessApp {
        session: Session::new(),
        cox: CoxUi::default(),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(420.0, 340.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    let add = harness.get_by_label("Add Layer").rect();
    let estimate = harness.get_by_label("Estimate Cox").rect();
    let pixels_per_point = harness.ctx.pixels_per_point();

    crate::common::assert_same_raster_edge(
        "Cox stack-action top edge",
        add.top(),
        estimate.top(),
        pixels_per_point,
    );
    crate::common::assert_same_raster_edge(
        "Cox stack-action bottom edge",
        add.bottom(),
        estimate.bottom(),
        pixels_per_point,
    );
    assert!(
        add.right() <= estimate.left() - 1.0,
        "Cox stack actions should split the row left-to-right: add={add:?}, estimate={estimate:?}"
    );
}

#[test]
fn cox_stack_actions_use_tight_card_bottom_margin() {
    let state = CoxHarnessApp {
        session: Session::new(),
        cox: CoxUi::default(),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(420.0, 340.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    let add = harness.get_by_label("Add Layer").rect();
    let inner_bottom = 300.0 - CARD_INNER_MARGIN as f32;
    let clearance = inner_bottom - add.bottom();
    assert!(
        (2.0..=6.0).contains(&clearance),
        "Cox footer should keep a tiny anti-clip clearance inside the tight card margin: add={add:?}, clearance={clearance:.1}"
    );
}

#[test]
fn estimated_cox_action_uses_tight_card_bottom_margin() {
    let state = CoxHarnessApp {
        session: Session::new(),
        cox: CoxUi::default(),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(420.0, 340.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();
    harness.get_by_label("Estimate Cox").click();
    harness.run();

    let use_estimated = harness.get_by_label("Use Estimated Cox").rect();
    let inner_bottom = 300.0 - CARD_INNER_MARGIN as f32;
    let clearance = crate::common::raster_pixel(inner_bottom)
        - crate::common::raster_pixel(use_estimated.bottom());
    assert!(
        (0..=6).contains(&clearance),
        "estimated Cox footer should keep 0-6 exact raster pixels inside the card: use_estimated={use_estimated:?}, clearance={clearance}"
    );
}

#[test]
fn estimated_cox_action_commits_the_displayed_estimate() {
    let state = CoxHarnessApp {
        session: Session::new(),
        cox: CoxUi::default(),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(420.0, 340.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    harness.get_by_label("Estimate Cox").click();
    harness.run();
    let estimated = harness
        .state()
        .cox
        .estimate_value()
        .expect("Estimate Cox should store a usable value");

    harness.get_by_label("Use Estimated Cox").click();
    harness.run();

    assert!(
        (harness.state().session.cox_nf_per_cm2() - estimated).abs() < f64::EPSILON,
        "Use Estimated Cox should commit the exact stored estimate"
    );
    assert!(
        harness
            .state()
            .cox
            .estimate_label()
            .starts_with("Using estimated C<sub>ox</sub>:"),
        "the status label should match the committed estimated-value state"
    );
}

#[test]
fn changing_the_stack_clears_the_stored_estimate() {
    let state = CoxHarnessApp {
        session: Session::new(),
        cox: CoxUi::default(),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(420.0, 340.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    harness.get_by_label("Estimate Cox").click();
    harness.run();
    assert!(harness.state().cox.estimate_value().is_some());

    harness.get_by_label("Add Layer").click();
    harness.run();

    assert_eq!(harness.state().cox.estimate_value(), None);
    assert_eq!(
        harness.state().cox.estimate_label(),
        COX_ESTIMATE_PENDING_LABEL
    );
}

#[test]
fn cox_stack_actions_stay_fixed_when_layer_rows_overflow() {
    let mut cox = CoxUi::default();
    for _ in 0..8 {
        cox.add_default_layer();
    }
    let state = CoxHarnessApp {
        session: Session::new(),
        cox,
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(420.0, 340.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    let add_layer = harness.get_by_label("Add Layer").rect();
    let estimate = harness.get_by_label("Estimate Cox").rect();

    assert!(
        add_layer.bottom() <= 330.0,
        "Add Layer should stay fixed inside the Cox card, not scroll below it: {add_layer:?}"
    );
    assert!(
        estimate.bottom() <= 330.0,
        "Estimate Cox should stay fixed inside the Cox card, not scroll below it: {estimate:?}"
    );
}
