// tests/selector_wiring.rs
use egui_kittest::{kittest::Queryable, Harness};
use paramex_core::transfer::{ExpertRanges, ExpertWindow, ParsedCurve, Session};
use paramex_gui::workspaces::transfer::state::{PlotCache, SelectorUi};

const DOUBLE: &str =
    include_str!("../../../../../paramex-core/tests/fixtures/parse/corpus_double.csv");

fn seed() -> (Session, String) {
    let vg: Vec<f64> = (0..40).map(|i| -2.0 + i as f64 * 0.1).collect();
    let id_abs: Vec<f64> = vg
        .iter()
        .map(|v| 1e-9 * (10f64).powf(v.max(0.0) * 2.0))
        .collect();
    let mut s = Session::new();
    let id = s
        .add_curve(ParsedCurve {
            name: "a.csv".into(),
            vg,
            id_abs,
            source_path: None,
        })
        .unwrap();
    s.select_file(&id);
    // Pre-pin so Auto Fit has something to clear.
    s.set_expert_window(&id, ExpertWindow::FwdVt, Some((0.5, 1.5)));
    (s, id)
}

fn seed_corpus() -> (Session, String) {
    let mut s = Session::new();
    let id = s
        .add_curve(crate::common::parse_transfer_fixture(
            DOUBLE,
            "corpus_double.csv",
        ))
        .unwrap();
    s.select_file(&id);
    (s, id)
}

fn projected_ranges(session: &Session, id: &str) -> ExpertRanges {
    assert_eq!(session.active_file_id(), Some(id));
    session
        .selected_fit_window_file()
        .expect("selected fit-window projection")
        .expert_ranges
}

#[test]
fn auto_fit_clears_all_four_windows() {
    let (session, id) = seed();
    let mut sel = SelectorUi::default();
    let mut plot = PlotCache::default();
    let mut edits = paramex_gui::state::EditBuffers::default();
    // Use a tall harness so the Auto Fit button below two 220px plots is in the visible rect.
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(800.0, 1400.0))
        .build_ui_state(
            move |ui, session: &mut Session| {
                paramex_gui::workspaces::transfer::selector::show(
                    ui, session, &mut sel, &mut plot, &mut edits,
                );
            },
            session,
        );
    harness.get_by_label("Reset to Auto").click();
    harness.run();
    let er = projected_ranges(harness.state(), &id);
    assert_eq!(er.vt_range, None);
    assert_eq!(er.ss_range, None);
    assert_eq!(er.vt_range_bwd, None);
    assert_eq!(er.ss_range_bwd, None);
}

#[test]
fn auto_fit_clears_windows_even_when_a_vg_field_has_focus() {
    // Resetting to Auto while a V_G range field has focus must clear the window
    // instead of committing the field's stale lost-focus value in the same frame.
    let (session, id) = seed();
    let mut sel = SelectorUi::default();
    let mut plot = PlotCache::default();
    let mut edits = paramex_gui::state::EditBuffers::default();
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(800.0, 1400.0))
        .build_ui_state(
            move |ui, session: &mut Session| {
                paramex_gui::workspaces::transfer::selector::show(
                    ui, session, &mut sel, &mut plot, &mut edits,
                );
            },
            session,
        );
    harness.run();
    // Focus the VT "V_G min" numeric field by clicking inside its box (just right of the
    // "VG min" label in the horizontal row; the field is 56px wide). No typing needed —
    // the bug fires from the field's seed value alone. Press+release at one point = click.
    let vmin = harness.get_all_by_label("VG min").next().unwrap().rect();
    let edit_pos = egui::pos2(vmin.right() + 28.0, vmin.center().y);
    harness.drag_at(edit_pos);
    harness.drop_at(edit_pos);
    harness.run();

    harness.get_by_label("Reset to Auto").click();
    harness.run();

    let er = projected_ranges(harness.state(), &id);
    assert_eq!(
        er.vt_range, None,
        "Reset to Auto must not be silently undone by a focused V_G field"
    );
    assert_eq!(er.ss_range, None);
    assert_eq!(er.vt_range_bwd, None);
    assert_eq!(er.ss_range_bwd, None);
}

#[test]
fn direction_toggle_does_not_pin_a_window_when_a_vg_field_has_focus() {
    // Changing direction while a V_G range field has focus must not commit the
    // unchanged display value or pin a window the user never edited.
    let (session, id) = seed_corpus(); // corpus_double is hysteretic -> the Fwd/Bwd toggle shows
    let mut sel = SelectorUi::default();
    let mut plot = PlotCache::default();
    let mut edits = paramex_gui::state::EditBuffers::default();
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(800.0, 1400.0))
        .build_ui_state(
            move |ui, session: &mut Session| {
                paramex_gui::workspaces::transfer::selector::show(
                    ui, session, &mut sel, &mut plot, &mut edits,
                );
            },
            session,
        );
    harness.run();
    // Both graphs start in Auto (no manual window).
    let er0 = projected_ranges(harness.state(), &id);
    assert_eq!(er0.vt_range, None);
    assert_eq!(er0.vt_range_bwd, None);

    // Focus the VT "V_G min" field (its box sits just right of the "VG min" label in the
    // horizontal row, 56px wide). A press+release at one point is a focusing click.
    let vmin = harness.get_all_by_label("VG min").next().unwrap().rect();
    let pos = egui::pos2(vmin.right() + 28.0, vmin.center().y);
    harness.drag_at(pos);
    harness.drop_at(pos);
    harness.run();

    // Click the VT "Backward" toggle: it steals the field's focus, and the field's lost_focus
    // would (unfixed) commit the full-axis display value as a Window pin the same frame.
    harness.get_all_by_label("Backward").next().unwrap().click();
    harness.run();

    let er = projected_ranges(harness.state(), &id);
    assert_eq!(
        er.vt_range_bwd, None,
        "direction toggle pinned a spurious backward window from a focused V_G field"
    );
    assert_eq!(
        er.vt_range, None,
        "direction toggle pinned a spurious forward window from a focused V_G field"
    );
    assert_eq!(er.ss_range, None);
    assert_eq!(er.ss_range_bwd, None);
}

#[test]
fn strip_drag_commits_at_real_window_size() {
    let (session, id) = seed();
    let mut sel = SelectorUi::default();
    let mut plot = PlotCache::default();
    let mut edits = paramex_gui::state::EditBuffers::default();
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(564.0, 438.0))
        .build_ui_state(
            move |ui, session: &mut Session| {
                paramex_gui::workspaces::transfer::selector::show(
                    ui, session, &mut sel, &mut plot, &mut edits,
                );
            },
            session,
        );

    harness.get_all_by_label("Forward").next().unwrap().click();
    harness.run();

    let before = projected_ranges(harness.state(), &id).vt_range;
    // The range strip sits just above the "Vg min" field.
    let vmin = harness.get_all_by_label("VG min").next().unwrap().rect();
    let y = vmin.top() - 14.0;
    harness.drag_at(egui::pos2(160.0, y));
    harness.hover_at(egui::pos2(210.0, y));
    harness.run();
    harness.drop_at(egui::pos2(210.0, y));
    harness.run();

    let after = projected_ranges(harness.state(), &id).vt_range;
    assert_ne!(
        before, after,
        "strip drag did not commit a changed VT range"
    );
}

#[test]
fn strip_drag_does_not_move_selector_layout() {
    let (session, _id) = seed_corpus();
    let mut sel = SelectorUi::default();
    let mut plot = PlotCache::default();
    let mut edits = paramex_gui::state::EditBuffers::default();
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(564.0, 438.0))
        .build_ui_state(
            move |ui, session: &mut Session| {
                paramex_gui::workspaces::transfer::selector::show(
                    ui, session, &mut sel, &mut plot, &mut edits,
                );
            },
            session,
        );
    harness.run();

    let before = selector_layout_rects(&harness);
    let vmin = harness.get_all_by_label("VG min").next().unwrap().rect();
    let y = vmin.top() - 14.0;
    harness.drag_at(egui::pos2(220.0, y));
    harness.hover_at(egui::pos2(160.0, y));
    harness.run();
    let dragging = selector_layout_rects(&harness);
    harness.drop_at(egui::pos2(160.0, y));
    harness.run();
    let committed = selector_layout_rects(&harness);
    let pixels_per_point = harness.ctx.pixels_per_point();

    assert_selector_layout_static("during strip drag", &before, &dragging, pixels_per_point);
    assert_selector_layout_static("after strip release", &before, &committed, pixels_per_point);
}

fn selector_layout_rects(harness: &Harness<'_, Session>) -> Vec<(&'static str, egui::Rect)> {
    vec![
        ("FIT", harness.get_by_label("FIT").rect()),
        (
            "VTH fit range",
            harness.get_by_label("VTH fit range").rect(),
        ),
        ("SS fit range", harness.get_by_label("SS fit range").rect()),
        (
            "Forward[0]",
            harness.get_all_by_label("Forward").next().unwrap().rect(),
        ),
        (
            "VG min[0]",
            harness.get_all_by_label("VG min").next().unwrap().rect(),
        ),
        (
            "VG max[0]",
            harness.get_all_by_label("VG max").next().unwrap().rect(),
        ),
    ]
}

fn assert_selector_layout_static(
    phase: &str,
    before: &[(&'static str, egui::Rect)],
    after: &[(&'static str, egui::Rect)],
    pixels_per_point: f32,
) {
    for ((label, before), (_, after)) in before.iter().zip(after) {
        crate::common::assert_same_raster_rect(
            &format!("{label} moved {phase}"),
            *before,
            *after,
            pixels_per_point,
        );
    }
}

#[test]
fn strip_drag_commits_without_an_intermediate_drag_frame() {
    let (session, id) = seed_corpus();
    let mut sel = SelectorUi::default();
    let mut plot = PlotCache::default();
    let mut edits = paramex_gui::state::EditBuffers::default();
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(564.0, 438.0))
        .build_ui_state(
            move |ui, session: &mut Session| {
                paramex_gui::workspaces::transfer::selector::show(
                    ui, session, &mut sel, &mut plot, &mut edits,
                );
            },
            session,
        );

    harness.get_all_by_label("Forward").next().unwrap().click();
    harness.run();

    let before = projected_ranges(harness.state(), &id).vt_range;
    let vmin = harness.get_all_by_label("VG min").next().unwrap().rect();
    let y = vmin.top() - 14.0;
    harness.drag_at(egui::pos2(220.0, y));
    harness.drop_at(egui::pos2(160.0, y));
    harness.run();

    let after = projected_ranges(harness.state(), &id).vt_range;
    assert_ne!(
        before, after,
        "strip drag did not commit when drag start/drop were delivered together"
    );
}

#[test]
fn graph_band_drag_commits_at_real_window_size() {
    let (session, id) = seed();
    let mut sel = SelectorUi::default();
    let mut plot = PlotCache::default();
    let mut edits = paramex_gui::state::EditBuffers::default();
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(564.0, 438.0))
        .build_ui_state(
            move |ui, session: &mut Session| {
                paramex_gui::workspaces::transfer::selector::show(
                    ui, session, &mut sel, &mut plot, &mut edits,
                );
            },
            session,
        );

    harness.get_all_by_label("Forward").next().unwrap().click();
    harness.run();

    let before = projected_ranges(harness.state(), &id).vt_range;
    let fwd = harness.get_all_by_label("Forward").next().unwrap().rect();
    let graph_y = fwd.top() - 110.0;
    harness.drag_at(egui::pos2(fwd.left() + 175.0, graph_y));
    harness.hover_at(egui::pos2(fwd.left() + 210.0, graph_y));
    harness.run();
    harness.drop_at(egui::pos2(fwd.left() + 210.0, graph_y));
    harness.run();

    let after = projected_ranges(harness.state(), &id).vt_range;
    assert_ne!(
        before, after,
        "graph band drag did not commit a changed VT range"
    );
}

#[test]
fn graph_band_drag_commits_in_stacked_tall_layout() {
    let (session, id) = seed();
    let mut sel = SelectorUi::default();
    let mut plot = PlotCache::default();
    let mut edits = paramex_gui::state::EditBuffers::default();
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(800.0, 1400.0))
        .build_ui_state(
            move |ui, session: &mut Session| {
                paramex_gui::workspaces::transfer::selector::show(
                    ui, session, &mut sel, &mut plot, &mut edits,
                );
            },
            session,
        );

    harness.get_all_by_label("Forward").next().unwrap().click();
    harness.run();

    let before = projected_ranges(harness.state(), &id).vt_range;
    let fwd = harness.get_all_by_label("Forward").next().unwrap().rect();
    let mut after = before;
    'probe: for y_offset in [110.0, 260.0, 410.0] {
        for start_frac in [0.7, 0.8, 0.9] {
            let graph_y = fwd.top() - y_offset;
            let start_x = fwd.left() + fwd.width() * start_frac;
            let end_x = fwd.left() + fwd.width() * (start_frac - 0.12);
            harness.drag_at(egui::pos2(start_x, graph_y));
            harness.hover_at(egui::pos2(end_x, graph_y));
            harness.run();
            harness.drop_at(egui::pos2(end_x, graph_y));
            harness.run();
            after = projected_ranges(harness.state(), &id).vt_range;
            if after != before {
                break 'probe;
            }
        }
    }
    assert_ne!(
        before, after,
        "graph band drag should commit inside the stacked VTH plot"
    );
}

#[test]
fn graph_band_drag_commits_on_real_corpus_curve() {
    let (session, id) = seed_corpus();
    let mut sel = SelectorUi::default();
    let mut plot = PlotCache::default();
    let mut edits = paramex_gui::state::EditBuffers::default();
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(564.0, 438.0))
        .build_ui_state(
            move |ui, session: &mut Session| {
                paramex_gui::workspaces::transfer::selector::show(
                    ui, session, &mut sel, &mut plot, &mut edits,
                );
            },
            session,
        );

    harness.get_all_by_label("Forward").next().unwrap().click();
    harness.run();

    let before = projected_ranges(harness.state(), &id).vt_range;
    let fwd = harness.get_all_by_label("Forward").next().unwrap().rect();
    let graph_y = fwd.top() - 110.0;
    harness.drag_at(egui::pos2(fwd.left() + 175.0, graph_y));
    harness.hover_at(egui::pos2(fwd.left() + 220.0, graph_y));
    harness.run();
    harness.drop_at(egui::pos2(fwd.left() + 220.0, graph_y));
    harness.run();

    let after = projected_ranges(harness.state(), &id).vt_range;
    assert_ne!(
        before, after,
        "graph band drag did not commit a changed VT range on the real corpus curve"
    );
}
