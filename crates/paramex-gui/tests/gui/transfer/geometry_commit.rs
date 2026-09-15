use egui_kittest::{
    kittest::{NodeT, Queryable},
    Harness,
};
use egui_notify::Toasts;
use paramex_core::transfer::{GeometryError, GeometrySource, Session};
use paramex_gui::state::EditBuffers;
use paramex_gui::workspaces::transfer::panels::geometry::commit_row_geometry;
use paramex_gui::workspaces::transfer::state::GeometryUi;

use crate::transfer_curve as curve;

#[test]
fn editing_width_preserves_length_sets_manual_and_recomputes() {
    let mut session = Session::new();
    let id = session.add_curve(curve("a.csv")).unwrap();

    // Edit W only (length = None preserves the current L = 50, source -> "manual").
    commit_row_geometry(&mut session, &id, Some(220.0), None).expect("positive W");
    let row = session.file_geometry_rows().into_iter().next().unwrap();
    assert_eq!(row.width_um, 220.0);
    assert_eq!(row.length_um, 50.0); // preserved
    assert_eq!(row.source, GeometrySource::Manual);
    // recompute(id) ran -> result reflects the new geometry.
    assert_eq!(
        session
            .selected_file_metrics_projection()
            .expect("selected metrics")
            .result
            .width_um,
        220.0
    );
}

#[test]
fn nonpositive_dimension_is_rejected_without_mutating() {
    let mut session = Session::new();
    let id = session.add_curve(curve("a.csv")).unwrap();
    let before = session.file_geometry_rows().into_iter().next().unwrap();

    let err = commit_row_geometry(&mut session, &id, Some(0.0), None).unwrap_err();
    assert_eq!(err, GeometryError);
    assert_eq!(
        session.file_geometry_rows().into_iter().next().unwrap(),
        before
    ); // unchanged
}

#[test]
fn geometry_global_inputs_parse_numeric_text_only() {
    let geo = GeometryUi::with_global_inputs("1500", "50");
    assert_eq!(geo.parse_global_wl(), Some((1500.0, 50.0)));

    let bad = GeometryUi::with_global_inputs("abc", "50");
    assert_eq!(bad.parse_global_wl(), None);
}

struct GeometryHarnessApp {
    session: Session,
    geo: GeometryUi,
    edits: EditBuffers,
    toasts: Toasts,
}

impl eframe::App for GeometryHarnessApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.allocate_ui(egui::Vec2::new(380.0, 300.0), |ui| {
            paramex_gui::workspaces::transfer::panels::geometry::show_setup(
                ui,
                &mut self.session,
                &mut self.geo,
                &mut self.edits,
                &mut self.toasts,
            );
        });
    }
}

#[test]
fn empty_global_wl_apply_action_keeps_slot_disabled() {
    let state = GeometryHarnessApp {
        session: Session::new(),
        geo: GeometryUi::default(),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(420.0, 430.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    assert!(
        harness
            .get_by_label("Apply W/L to All Files")
            .accesskit_node()
            .is_disabled(),
        "empty-state global W/L apply should render disabled"
    );
}

#[test]
fn global_wl_pair_inputs_share_one_row_baseline() {
    let state = GeometryHarnessApp {
        session: Session::new(),
        geo: GeometryUi::default(),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(420.0, 430.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

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
    assert!(inputs.len() >= 2, "global W/L inputs should render");

    let w = inputs[0];
    let l = inputs[1];
    let pixels_per_point = harness.ctx.pixels_per_point();
    crate::common::assert_same_raster_span(
        "global W/L field widths",
        (w.left(), w.right()),
        (l.left(), l.right()),
        pixels_per_point,
    );
    crate::common::assert_same_raster_edge(
        "global W/L field top edge",
        w.top(),
        l.top(),
        pixels_per_point,
    );
    crate::common::assert_same_raster_edge(
        "global W/L field bottom edge",
        w.bottom(),
        l.bottom(),
        pixels_per_point,
    );
}

#[test]
fn geometry_rows_scroll_to_late_files() {
    let mut session = Session::new();
    for idx in 0..24 {
        session.add_curve(curve(&format!("A_{idx:02}.csv")));
    }

    let state = GeometryHarnessApp {
        session,
        geo: GeometryUi::default(),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(420.0, 430.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    for _ in 0..120 {
        harness
            .input_mut()
            .events
            .push(egui::Event::PointerMoved(egui::pos2(210.0, 240.0)));
        harness.input_mut().events.push(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -7.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        harness.run();
    }

    let late = harness.get_by_label("A_23.csv").rect();
    assert!(
        late.top() >= 0.0 && late.bottom() <= 390.0,
        "late geometry rows should be visible after scrolling: {late:?}"
    );
}

#[test]
fn geometry_table_file_cells_use_quiet_table_clipping_contract() {
    let long_name = "very_long_geometry_filename_that_should_not_break_the_card_width.csv";
    let mut session = Session::new();
    session.add_curve(curve(long_name));

    let state = GeometryHarnessApp {
        session,
        geo: GeometryUi::default(),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(420.0, 430.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    assert!(
        harness.get_by_label("File").rect().is_positive(),
        "geometry table should use the same capitalized File header as other quiet tables"
    );
    assert!(
        harness
            .get_all_by_label("W (µm)")
            .any(|node| node.rect().is_positive()),
        "geometry table width header should carry the same unit language as results tables"
    );
    assert!(
        harness
            .get_all_by_label("L (µm)")
            .any(|node| node.rect().is_positive()),
        "geometry table length header should carry the same unit language as results tables"
    );
    assert!(
        harness.query_by_label("Source").is_none(),
        "geometry table should not spend narrow right-column width on a Source column"
    );

    let name = harness.get_by_label(long_name).rect();
    assert!(
        name.right() <= 420.0,
        "long geometry filenames should clip inside the card instead of painting across columns: {name:?}"
    );
}

#[test]
fn geometry_table_wl_inputs_spend_their_table_columns() {
    let mut session = Session::new();
    session.add_curve(curve("a.csv"));

    let state = GeometryHarnessApp {
        session,
        geo: GeometryUi::default(),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(420.0, 430.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    let inputs: Vec<_> = harness
        .get_all_by_role(egui::accesskit::Role::TextInput)
        .map(|node| node.rect())
        .collect();
    let row_y = inputs
        .iter()
        .map(|rect| rect.center().y)
        .fold(f32::MIN, f32::max);
    let mut row_inputs: Vec<_> = inputs
        .into_iter()
        .filter(|rect| (rect.center().y - row_y).abs() < 12.0)
        .collect();
    row_inputs.sort_by(|a, b| a.left().total_cmp(&b.left()));

    assert_eq!(row_inputs.len(), 2, "one editable W/L row should render");
    crate::common::assert_same_raster_span(
        "geometry-table W/L field widths",
        (row_inputs[0].left(), row_inputs[0].right()),
        (row_inputs[1].left(), row_inputs[1].right()),
        harness.ctx.pixels_per_point(),
    );
    assert!(
        row_inputs[0].width() >= 64.0,
        "W/L fields should fill useful table columns instead of tiny islands: {row_inputs:?}"
    );
}

#[test]
fn manual_geometry_rows_use_a_badge_instead_of_a_source_column() {
    let mut session = Session::new();
    let id = session.add_curve(curve("manual_width.csv")).unwrap();
    commit_row_geometry(&mut session, &id, Some(900.0), None).unwrap();

    let state = GeometryHarnessApp {
        session,
        geo: GeometryUi::default(),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(420.0, 430.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    assert!(harness.get_by_label("MANUAL").rect().is_positive());
    assert!(harness.query_by_label("Source").is_none());
}

/// Non-numeric global W/L text must report "must be numeric" (one warning
/// toast), not be coerced to 0.0 and misreported as "must be positive".
#[test]
fn non_numeric_global_wl_warns_must_be_numeric() {
    let mut session = Session::new();
    session.add_curve(curve("a.csv"));
    let geo = GeometryUi::with_global_inputs("abc", "50");

    let state = GeometryHarnessApp {
        session,
        geo,
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(420.0, 430.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    harness.get_by_label("Apply W/L to All Files").click();
    harness.run();

    assert_eq!(harness.state().toasts.len(), 1, "exactly one warning toast");
    // The committed geometry is untouched by the failed apply.
    let rows = harness.state().session.file_geometry_rows();
    assert_eq!(rows.first().unwrap().width_um, 1500.0); // DeviceGeometry default W
}

#[test]
fn global_wl_apply_wins_over_a_focused_per_file_field() {
    // Applying global W/L while a per-file field has focus must win over that field's
    // stale lost-focus value, keeping the file's geometry source global.
    let mut session = Session::new();
    session.add_curve(curve("a.csv")).unwrap(); // default W=1500, L=50
    let state = GeometryHarnessApp {
        session,
        geo: GeometryUi::with_global_inputs("700", "50"), // apply sets W=700
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
    };
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(420.0, 430.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();

    // Focus the per-file W field (no typing). The per-file W/L inputs are the table row,
    // BELOW the two global W/L inputs; within that lowest row, W is the left one.
    {
        let nodes: Vec<_> = harness
            .get_all_by_role(egui::accesskit::Role::TextInput)
            .collect();
        let max_y = nodes
            .iter()
            .map(|n| n.rect().center().y)
            .fold(f32::MIN, f32::max);
        let w = nodes
            .into_iter()
            .filter(|n| (n.rect().center().y - max_y).abs() < 12.0)
            .min_by(|a, b| a.rect().center().x.total_cmp(&b.rect().center().x))
            .expect("a per-file W text input is present");
        w.focus();
    }
    harness.run();

    harness.get_by_label("Apply W/L to All Files").click();
    harness.run();

    let row = harness
        .state()
        .session
        .file_geometry_rows()
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(
        row.width_um, 700.0,
        "global Apply must win over a focused per-file W field (got stale {})",
        row.width_um
    );
    assert_eq!(
        row.source,
        GeometrySource::Global,
        "after Apply, the focused file's source must be global, not overridden to manual"
    );
}

#[test]
fn no_edit_focus_loss_does_not_flip_geometry_source_to_manual() {
    // Losing focus without editing must not commit an unchanged per-file value or
    // change its geometry source from default to manual.
    let mut session = Session::new();
    session.add_curve(curve("a.csv")).unwrap(); // source "default"
    let state = GeometryHarnessApp {
        session,
        geo: GeometryUi::default(),
        edits: EditBuffers::default(),
        toasts: Toasts::default(),
    };
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(420.0, 430.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            state
        });
    harness.run();
    assert_eq!(
        harness.state().session.file_geometry_rows()[0].source,
        GeometrySource::Default
    );

    // Focus the per-file W field (lowest row, left), then steal its focus by focusing the
    // top-most input (global W) -- a no-edit focus loss.
    let pick = |harness: &mut Harness<GeometryHarnessApp>, lowest: bool| {
        let nodes: Vec<_> = harness
            .get_all_by_role(egui::accesskit::Role::TextInput)
            .collect();
        let y: Vec<f32> = nodes.iter().map(|n| n.rect().center().y).collect();
        let target_y = if lowest {
            y.iter().cloned().fold(f32::MIN, f32::max)
        } else {
            y.iter().cloned().fold(f32::MAX, f32::min)
        };
        let n = nodes
            .into_iter()
            .filter(|n| (n.rect().center().y - target_y).abs() < 12.0)
            .min_by(|a, b| a.rect().center().x.total_cmp(&b.rect().center().x))
            .expect("a text input");
        n.focus();
    };
    pick(&mut harness, true); // per-file W
    harness.run();
    pick(&mut harness, false); // global W -> steals the per-file field's focus
    harness.run();

    assert_eq!(
        harness.state().session.file_geometry_rows()[0].source,
        GeometrySource::Default,
        "a no-edit focus loss on a per-file W field must not flip the source to manual"
    );
}
