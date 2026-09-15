// tests/selector_render.rs
use paramex_core::transfer::{ParsedCurve, Session};
use paramex_gui::workspaces::transfer::selector::bands::band_rect_points;
use paramex_gui::workspaces::transfer::state::{PlotCache, SelectorUi};

struct SelectorHarnessApp {
    session: Session,
    sel: SelectorUi,
    plot: PlotCache,
    edits: paramex_gui::state::EditBuffers,
}

impl eframe::App for SelectorHarnessApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        paramex_gui::workspaces::transfer::selector::show(
            ui,
            &mut self.session,
            &mut self.sel,
            &mut self.plot,
            &mut self.edits,
        );
    }
}

#[test]
fn band_rect_is_full_height_four_corners() {
    let pts = band_rect_points(0.5, 1.5, -3.0, 2.0);
    assert_eq!(pts, vec![[0.5, -3.0], [1.5, -3.0], [1.5, 2.0], [0.5, 2.0]]);
}

#[test]
fn selector_panel_renders_without_panic() {
    use egui_kittest::Harness;
    let vg: Vec<f64> = (0..40).map(|i| -2.0 + i as f64 * 0.1).collect();
    let id_abs: Vec<f64> = vg
        .iter()
        .map(|v| 1e-9 * (10f64).powf(v.max(0.0) * 2.0))
        .collect();
    let mut session = Session::new();
    let id = session
        .add_curve(ParsedCurve {
            name: "a.csv".into(),
            vg,
            id_abs,
            source_path: None,
        })
        .unwrap();
    assert!(session.select_file(&id));

    let mut sel = SelectorUi::default();
    let mut plot = PlotCache::default();
    let mut edits = paramex_gui::state::EditBuffers::default();
    let mut harness = Harness::new_ui_state(
        move |ui, session: &mut Session| {
            paramex_gui::workspaces::transfer::selector::show(
                ui, session, &mut sel, &mut plot, &mut edits,
            );
        },
        session,
    );
    harness.run(); // must not panic; the two plots build from a seeded session
}

#[test]
fn selector_default_state_exposes_readable_plot_and_range_labels() {
    use egui_kittest::{
        kittest::{NodeT, Queryable},
        Harness,
    };
    let mut session = Session::new();
    let id = session
        .add_curve(ParsedCurve {
            name: "double.csv".into(),
            vg: (0..80).map(|i| -2.0 + i as f64 * 0.1).collect(),
            id_abs: (0..80)
                .map(|i| 1e-12 * 10f64.powf(i as f64 / 16.0))
                .collect(),
            source_path: None,
        })
        .unwrap();
    assert!(session.select_file(&id));

    let mut harness = Harness::builder().build_eframe(|cc| {
        paramex_gui::theme::install(&cc.egui_ctx);
        SelectorHarnessApp {
            session,
            sel: SelectorUi::default(),
            plot: PlotCache::default(),
            edits: paramex_gui::state::EditBuffers::default(),
        }
    });
    harness.run();

    assert!(harness.get_by_label("FIT").rect().is_positive());
    assert!(harness
        .get_by_label("Reset to Auto")
        .accesskit_node()
        .is_disabled());
    assert!(harness.get_by_label("VTH fit range").rect().is_positive());
    assert!(harness.get_by_label("SS fit range").rect().is_positive());
    // Axis titles ("Gate voltage VG (V)", "√|ID| (A^1/2)") render INSIDE the plot via
    // egui_plot AxisHints (painted text, not accesskit nodes) — their visibility
    // is covered by the app_real/app_tall snapshot baselines.
    assert!(harness
        .get_all_by_label("VG min")
        .any(|node| node.rect().is_positive()));
    assert!(harness
        .get_all_by_label("VG max")
        .any(|node| node.rect().is_positive()));
    assert!(harness
        .get_all_by_label("Forward")
        .any(|node| node.rect().is_positive()));
}

#[test]
fn selector_range_pair_inputs_share_exact_vertical_bounds() {
    use egui_kittest::{kittest::Queryable, Harness};
    let mut harness = Harness::builder()
        .with_size(egui::vec2(650.0, 450.0))
        .with_pixels_per_point(1.5)
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            SelectorHarnessApp {
                session: Session::new(),
                sel: SelectorUi::default(),
                plot: PlotCache::default(),
                edits: paramex_gui::state::EditBuffers::default(),
            }
        });
    harness.run();
    harness.run();

    let mut inputs: Vec<_> = harness
        .get_all_by_role(egui::accesskit::Role::TextInput)
        .map(|node| node.rect())
        .collect();
    inputs.sort_by(|a, b| a.left().total_cmp(&b.left()));
    assert_eq!(inputs.len(), 4);
    for pair in inputs.chunks_exact(2) {
        crate::common::assert_same_raster_edge(
            "selector paired-input top at 150% DPI",
            pair[0].top(),
            pair[1].top(),
            harness.ctx.pixels_per_point(),
        );
        crate::common::assert_same_raster_edge(
            "selector paired-input bottom at 150% DPI",
            pair[0].bottom(),
            pair[1].bottom(),
            harness.ctx.pixels_per_point(),
        );
    }

    let image = crate::common::render(&mut harness);
    for rect in inputs {
        let x0 = (rect.left() + 8.0).ceil().max(0.0) as u32;
        let x1 = (rect.right() - 8.0)
            .floor()
            .min(image.width().saturating_sub(1) as f32) as u32;
        let bottom = rect.bottom().round() as i32;
        let border_pixels = ((bottom - 2).max(0)..=bottom + 1)
            .filter(|y| *y < image.height() as i32)
            .map(|y| {
                (x0..=x1)
                    .filter(|x| {
                        let pixel = image.get_pixel(*x, y as u32);
                        pixel[0] < 248 || pixel[1] < 248 || pixel[2] < 248
                    })
                    .count()
            })
            .max()
            .unwrap_or_default();
        assert!(
            border_pixels * 4 >= (x1 - x0 + 1) as usize * 3,
            "numeric input bottom border is clipped at 150% DPI: rect={rect:?}"
        );
    }
}

#[test]
fn partial_extraction_keeps_selector_range_controls_visible() {
    use egui_kittest::{kittest::Queryable, Harness};
    let mut session = Session::new();
    let id = session
        .add_curve(crate::common::partial_transfer_curve("partial.csv"))
        .unwrap();
    assert!(session.select_file(&id));

    let mut harness = Harness::builder().build_eframe(|cc| {
        paramex_gui::theme::install(&cc.egui_ctx);
        SelectorHarnessApp {
            session,
            sel: SelectorUi::default(),
            plot: PlotCache::default(),
            edits: paramex_gui::state::EditBuffers::default(),
        }
    });
    harness.run();

    assert!(harness
        .get_all_by_label("VG min")
        .any(|node| node.rect().is_positive()));
    assert!(harness
        .get_all_by_label("VG max")
        .any(|node| node.rect().is_positive()));
}
