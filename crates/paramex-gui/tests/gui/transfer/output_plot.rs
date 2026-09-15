use egui_kittest::{
    kittest::{NodeT, Queryable},
    Harness,
};
use paramex_core::transfer::{OutputCurve, OutputDataset, Session};
use paramex_gui::app::ParamExApp;
use paramex_gui::state::EditBuffers;
use paramex_gui::workspaces::transfer::panels::output_plot;
use paramex_gui::workspaces::transfer::selector::strip::thumb_centers;

use crate::{attach_output, partial_output_dataset, transfer_curve as curve};

fn output_dataset(name: &str) -> OutputDataset {
    OutputDataset {
        name: name.to_string(),
        curves: vec![
            OutputCurve {
                vg: 5.0,
                vd: vec![0.0, 1.0, 2.0, 3.0],
                id: vec![0.0, 1.0e-6, 1.7e-6, 2.5e-6],
            },
            OutputCurve {
                vg: 1.0,
                vd: vec![0.0, 1.0, 2.0, 3.0],
                id: vec![0.0, 0.2e-6, 0.34e-6, 0.5e-6],
            },
            OutputCurve {
                vg: 3.0,
                vd: vec![0.0, 1.0, 2.0, 3.0],
                id: vec![0.0, 0.6e-6, 1.02e-6, 1.5e-6],
            },
        ],
        source_path: Some(std::path::PathBuf::from(name)),
    }
}

fn no_finite_output_dataset(name: &str) -> OutputDataset {
    OutputDataset {
        name: name.to_string(),
        curves: vec![OutputCurve {
            vg: 5.0,
            vd: vec![0.0, 1.0, 2.0],
            id: vec![f64::NAN, f64::NAN, f64::NAN],
        }],
        source_path: Some(std::path::PathBuf::from(name)),
    }
}

fn pchannel_far_output_dataset(name: &str) -> OutputDataset {
    OutputDataset {
        name: name.to_string(),
        curves: vec![OutputCurve {
            vg: -5.0,
            vd: vec![-10.0, -9.8, -9.6, 0.0],
            id: vec![-4.100e-6, -4.098e-6, -4.096e-6, 0.0],
        }],
        source_path: Some(std::path::PathBuf::from(name)),
    }
}

fn session_with_output() -> Session {
    let mut session = Session::new();
    session.add_curve(curve("device_a.csv"));
    assert!(attach_output(&mut session, output_dataset("device_a_output.csv")).is_none());
    session
}

fn selected_fit_range(session: &Session) -> Option<(f64, f64)> {
    session
        .selected_output_file()
        .and_then(|selected| selected.selected_fit_range)
}

struct OutputPlotHarnessApp {
    session: Session,
    edits: EditBuffers,
    cache: output_plot::OutputPlotCache,
}

impl eframe::App for OutputPlotHarnessApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.allocate_ui(egui::vec2(620.0, 320.0), |ui| {
            output_plot::show(ui, &mut self.session, &mut self.edits, &mut self.cache);
        });
    }
}

struct TallOutputPlotHarnessApp {
    session: Session,
    edits: EditBuffers,
    cache: output_plot::OutputPlotCache,
}

impl eframe::App for TallOutputPlotHarnessApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.allocate_ui(egui::vec2(620.0, 1070.0), |ui| {
            output_plot::show(ui, &mut self.session, &mut self.edits, &mut self.cache);
        });
    }
}

#[test]
fn plot_model_shows_signed_and_magnitude_pchannel_current_identically() {
    let signed = OutputDataset {
        name: "pch_signed_output.csv".to_string(),
        curves: vec![OutputCurve {
            vg: -5.0,
            vd: vec![0.0, -1.0, -2.0, f64::NAN],
            id: vec![0.0, -1.0e-6, -2.0e-6, -3.0e-6],
        }],
        source_path: None,
    };
    let mut magnitude = signed.clone();
    magnitude.name = "pch_abs_output.csv".to_string();
    magnitude.curves[0].id = vec![0.0, 1.0e-6, 2.0e-6, 3.0e-6];

    let signed_model = output_plot::plot_model(&signed).expect("finite signed points");
    let magnitude_model = output_plot::plot_model(&magnitude).expect("finite magnitude points");

    assert_eq!(signed_model, magnitude_model);
    assert_eq!(
        signed_model.series[0].points,
        vec![[0.0, 0.0], [-1.0, 1.0e-6], [-2.0, 2.0e-6]]
    );
    assert!(
        signed_model.x_bounds.0 <= -2.0,
        "{:?}",
        signed_model.x_bounds
    );
    assert!(
        signed_model.x_bounds.1 >= 0.0,
        "{:?}",
        signed_model.x_bounds
    );
    assert_eq!(signed_model.y_bounds.0, 0.0);
    assert!(
        signed_model.y_bounds.1 >= 2.0e-6,
        "{:?}",
        signed_model.y_bounds
    );
}

#[test]
fn output_curves_panel_warns_once_when_output_has_no_finite_points() {
    let mut session = Session::new();
    session.add_curve(curve("device_a.csv"));
    assert!(attach_output(
        &mut session,
        no_finite_output_dataset("device_a_output.csv")
    )
    .is_none());

    let mut harness = Harness::builder()
        .with_size(egui::vec2(650.0, 350.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            OutputPlotHarnessApp {
                session,
                edits: EditBuffers::default(),
                cache: output_plot::OutputPlotCache::default(),
            }
        });
    harness.run();
    harness.run();

    harness.get_by_label("VD min");
    harness.get_by_label("VD max");
    assert_eq!(
        harness.get_all_by_label("No finite Id-Vd points").count(),
        1
    );
    assert!(
        harness
            .get_by_label("VD min")
            .accesskit_node()
            .is_disabled(),
        "an unusable loaded output keeps its fit-range controls inert"
    );
    assert!(harness.query_by_label("Output fit unavailable").is_none());
    assert!(harness
        .query_by_label("Output data is loaded, but no finite Id-Vd points were found.")
        .is_none());
}

#[test]
fn output_curves_panel_reports_partial_fit_coverage() {
    let mut session = Session::new();
    session.add_curve(curve("device_a.csv"));
    assert!(attach_output(&mut session, partial_output_dataset("device_a_output.csv")).is_none());

    let mut harness = Harness::builder()
        .with_size(egui::vec2(650.0, 350.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            OutputPlotHarnessApp {
                session,
                edits: EditBuffers::default(),
                cache: output_plot::OutputPlotCache::default(),
            }
        });
    harness.run();
    harness.run();

    harness.get_by_label("1 of 2 output lines unavailable");
    harness.get_by_label("WARN");
}

#[test]
fn output_curves_panel_renders_attached_output_and_range_controls() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(650.0, 350.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            OutputPlotHarnessApp {
                session: session_with_output(),
                edits: EditBuffers::default(),
                cache: output_plot::OutputPlotCache::default(),
            }
        });
    harness.run();
    harness.run();

    harness.get_by_label("OUTPUT");
    assert!(harness.query_by_label("VD fit range").is_none());
    assert!(harness
        .get_by_label("Reset to Auto")
        .accesskit_node()
        .is_disabled());
    harness.get_by_label("VD min");
    harness.get_by_label("VD max");
    harness.get_by_label("VG 1 \u{2192} 5 V");
    assert!(
        harness.query_by_label("OK").is_none(),
        "successful output metrics belong in RESULTS, not the plot card"
    );
}

fn is_output_range_strip_pixel<P>(pixel: &P) -> bool
where
    P: std::ops::Index<usize, Output = u8>,
{
    let is_rail_gray = pixel[3] > 180
        && pixel[0] >= 110
        && pixel[0] <= 160
        && (pixel[0] as i16 - pixel[1] as i16).abs() <= 6
        && (pixel[1] as i16 - pixel[2] as i16).abs() <= 6;
    let is_primary_blue = pixel[3] > 180 && pixel[0] < 30 && pixel[1] < 110 && pixel[2] > 190;
    is_rail_gray || is_primary_blue
}

fn output_range_strip_span<State>(harness: &mut Harness<'_, State>) -> (u32, u32, u32) {
    let vmin = harness.get_by_label("VD min").rect();
    let image = crate::common::render(harness);
    let y0 = (vmin.top() - 22.0).round().max(0.0) as u32;
    let y1 = (vmin.top() - 8.0)
        .round()
        .min(image.height().saturating_sub(1) as f32) as u32;
    (y0..=y1)
        .filter_map(|y| {
            let xs: Vec<u32> = (0..image.width())
                .filter(|x| is_output_range_strip_pixel(image.get_pixel(*x, y)))
                .collect();
            Some((xs.iter().copied().min()?, xs.iter().copied().max()?, y))
        })
        .max_by_key(|(left, right, _)| right - left)
        .expect("range rail pixels")
}

#[test]
fn tall_output_plot_groups_stack_and_keep_range_strip_interactive() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(650.0, 1100.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            TallOutputPlotHarnessApp {
                session: session_with_output(),
                edits: EditBuffers::default(),
                cache: output_plot::OutputPlotCache::default(),
            }
        });
    harness.run();
    harness.run();

    let transfer = harness.get_by_label("Transfer").rect();
    let output = harness.get_by_label("Output").rect();
    let gate_span = harness.get_by_label("VG 1 \u{2192} 5 V").rect();
    crate::common::assert_same_raster_edge(
        "tall Output shared left title rail",
        transfer.left(),
        output.left(),
        harness.ctx.pixels_per_point(),
    );
    assert!(
        output.top() > transfer.bottom(),
        "tall Output plots should stack vertically: transfer={transfer:?}, output={output:?}"
    );
    assert!(
        gate_span.center().y >= output.top() && gate_span.center().y <= output.bottom(),
        "gate-voltage color key should share the Output caption rail: output={output:?}, gate_span={gate_span:?}"
    );
    let vmin = harness.get_by_label("VD min").rect();
    assert!(
        vmin.top() > output.bottom() && vmin.bottom() < 1070.0,
        "stacked plots should leave the V_D range controls reachable: output={output:?}, vmin={vmin:?}"
    );

    let (left, right, y) = output_range_strip_span(&mut harness);
    let at = egui::pos2(left as f32 + (right - left) as f32 * 0.35, y as f32);
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

    assert!(
        selected_fit_range(&harness.state().session).is_some(),
        "the V_D range strip should still commit below stacked plots"
    );
}

#[test]
fn output_range_strip_spans_the_output_card_body() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(650.0, 350.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            OutputPlotHarnessApp {
                session: session_with_output(),
                edits: EditBuffers::default(),
                cache: output_plot::OutputPlotCache::default(),
            }
        });
    harness.run();
    harness.run();

    let (left, right, _) = output_range_strip_span(&mut harness);

    assert!(
        right - left >= 500,
        "output V_D range rail should span the output card body, got span {left}..{right}"
    );
    assert!(
        left >= 20 && right <= 630,
        "output V_D range rail should keep normal card insets, got span {left}..{right}"
    );
}

#[test]
fn output_plot_layout_does_not_shift_when_range_becomes_manual() {
    let auto_rects = |session: Session| {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(650.0, 350.0))
            .build_eframe(|cc| {
                paramex_gui::theme::install(&cc.egui_ctx);
                OutputPlotHarnessApp {
                    session,
                    edits: EditBuffers::default(),
                    cache: output_plot::OutputPlotCache::default(),
                }
            });
        harness.run();
        harness.run();
        (
            harness.ctx.pixels_per_point(),
            [
                ("OUTPUT", harness.get_by_label("OUTPUT").rect()),
                ("VD min", harness.get_by_label("VD min").rect()),
                ("VD max", harness.get_by_label("VD max").rect()),
            ],
        )
    };

    let (pixels_per_point, auto) = auto_rects(session_with_output());
    let mut manual_session = session_with_output();
    let id = manual_session.file_ids().next().unwrap().to_string();
    assert!(manual_session.set_output_fit_range(&id, Some((0.0, 1.0))));
    let (manual_pixels_per_point, manual) = auto_rects(manual_session);
    assert_eq!(pixels_per_point, manual_pixels_per_point);

    for ((label, auto_rect), (_, manual_rect)) in auto.into_iter().zip(manual) {
        crate::common::assert_same_raster_rect(
            &format!("{label} moved when output range became manual"),
            auto_rect,
            manual_rect,
            pixels_per_point,
        );
    }
}

#[test]
fn output_range_pair_inputs_share_exact_vertical_bounds_at_fractional_dpi() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(650.0, 350.0))
        .with_pixels_per_point(1.5)
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            OutputPlotHarnessApp {
                session: session_with_output(),
                edits: EditBuffers::default(),
                cache: output_plot::OutputPlotCache::default(),
            }
        });
    harness.run();
    harness.run();

    let mut inputs: Vec<_> = harness
        .get_all_by_role(egui::accesskit::Role::TextInput)
        .map(|node| node.rect())
        .collect();
    inputs.sort_by(|a, b| a.left().total_cmp(&b.left()));
    assert_eq!(inputs.len(), 2);
    crate::common::assert_same_raster_edge(
        "Output range-input top at 150% DPI",
        inputs[0].top(),
        inputs[1].top(),
        harness.ctx.pixels_per_point(),
    );
    crate::common::assert_same_raster_edge(
        "Output range-input bottom at 150% DPI",
        inputs[0].bottom(),
        inputs[1].bottom(),
        harness.ctx.pixels_per_point(),
    );
}

#[test]
fn output_curves_panel_keeps_range_controls_inside_the_card() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(650.0, 350.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            OutputPlotHarnessApp {
                session: session_with_output(),
                edits: EditBuffers::default(),
                cache: output_plot::OutputPlotCache::default(),
            }
        });
    harness.run();
    harness.run();

    let vds_max = harness.get_by_label("VD max").rect();
    assert!(
        vds_max.bottom() <= 320.0,
        "output range controls are clipped below the card: {vds_max:?}"
    );
}

#[test]
fn output_curves_panel_renders_no_selected_file_empty_state() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(650.0, 350.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            OutputPlotHarnessApp {
                session: Session::new(),
                edits: EditBuffers::default(),
                cache: output_plot::OutputPlotCache::default(),
            }
        });
    harness.run();

    harness.get_by_label("VD min");
    harness.get_by_label("VD max");
    assert!(harness
        .query_by_label("Load or select a transfer curve to see output fit.")
        .is_none());
}

#[test]
fn output_curves_panel_renders_no_output_empty_state_for_selected_file() {
    let mut session = Session::new();
    session.add_curve(curve("device_a.csv"));

    let mut harness = Harness::builder()
        .with_size(egui::vec2(650.0, 350.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            OutputPlotHarnessApp {
                session,
                edits: EditBuffers::default(),
                cache: output_plot::OutputPlotCache::default(),
            }
        });
    harness.run();

    harness.get_by_label("OUTPUT");
    harness.get_by_label("VD min");
    harness.get_by_label("VD max");
    assert!(harness.query_by_label("device_a.csv").is_none());
    assert!(harness
        .query_by_label("Load matching output data to see output fit.")
        .is_none());
}

#[test]
fn full_app_output_segment_routes_top_center_to_output_curves() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            ParamExApp::from_session(session_with_output())
        });
    harness.run();
    harness.run();

    harness.get_by_label("FIT");
    assert!(harness.query_by_label("VD min").is_none());

    harness.get_by_label("Output Fit").click();
    harness.run_steps(1);

    harness.get_by_label("FIT");
    harness.get_by_label("Output file");

    harness.run_steps(1);
    harness.get_by_label("VD min");
    assert!(harness
        .get_by_label("Reset to Auto")
        .accesskit_node()
        .is_disabled());
}

#[test]
fn output_fit_range_refreshes_report_and_reset_button_restores_default() {
    let mut session = session_with_output();
    let id = session.file_ids().next().unwrap().to_string();
    assert!(session.set_output_fit_range(&id, Some((0.0, 1.0))));

    let mut harness = Harness::builder()
        .with_size(egui::vec2(650.0, 350.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            OutputPlotHarnessApp {
                session,
                edits: EditBuffers::default(),
                cache: output_plot::OutputPlotCache::default(),
            }
        });
    harness.run();
    harness.run();

    harness.get_by_label("VD min");
    harness.get_by_label("VD max");
    harness.get_by_label("Reset to Auto");
    let ranged_report = harness.state().session.output_report_bytes();
    assert!(
        String::from_utf8_lossy(&ranged_report).contains("0.000000e0,1.000000e0"),
        "report should use the manual output fit range"
    );

    harness.get_by_label("Reset to Auto").click();
    harness.run();
    harness.run();

    assert_eq!(selected_fit_range(&harness.state().session), None);
    assert!(harness
        .get_by_label("Reset to Auto")
        .accesskit_node()
        .is_disabled());
    let default_report = harness.state().session.output_report_bytes();
    assert!(
        String::from_utf8_lossy(&default_report).contains("2.000000e0,3.000000e0"),
        "report should return to the default output fit range"
    );
}

#[test]
fn output_range_strip_drag_commits_manual_range() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(650.0, 350.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            OutputPlotHarnessApp {
                session: session_with_output(),
                edits: EditBuffers::default(),
                cache: output_plot::OutputPlotCache::default(),
            }
        });
    harness.run();
    harness.run();

    assert_eq!(selected_fit_range(&harness.state().session), None);

    let (left, right, y) = output_range_strip_span(&mut harness);
    harness.drag_at(egui::pos2(right as f32 - 10.0, y as f32));
    harness.hover_at(egui::pos2(
        left as f32 + (right - left) as f32 * 0.55,
        y as f32,
    ));
    harness.run();
    harness.drop_at(egui::pos2(
        left as f32 + (right - left) as f32 * 0.55,
        y as f32,
    ));
    harness.run();

    assert!(
        selected_fit_range(&harness.state().session).is_some(),
        "dragging the output range strip should pin a manual Vd range"
    );
    harness.get_by_label("Reset to Auto");
}

#[test]
fn output_range_strip_drag_pins_range_before_release() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(650.0, 350.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            OutputPlotHarnessApp {
                session: session_with_output(),
                edits: EditBuffers::default(),
                cache: output_plot::OutputPlotCache::default(),
            }
        });
    harness.run();
    harness.run();

    let (left, right, y) = output_range_strip_span(&mut harness);
    harness.drag_at(egui::pos2(right as f32 - 10.0, y as f32));
    harness.hover_at(egui::pos2(
        left as f32 + (right - left) as f32 * 0.55,
        y as f32,
    ));
    harness.run();

    assert!(
        selected_fit_range(&harness.state().session).is_some(),
        "dragging the output range strip should pin a manual Vd range before pointer release"
    );
}

#[test]
fn output_range_strip_click_commits_manual_range() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(650.0, 350.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            OutputPlotHarnessApp {
                session: session_with_output(),
                edits: EditBuffers::default(),
                cache: output_plot::OutputPlotCache::default(),
            }
        });
    harness.run();
    harness.run();

    let (left, right, y) = output_range_strip_span(&mut harness);
    let at = egui::pos2(left as f32 + (right - left) as f32 * 0.35, y as f32);
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

    let range = selected_fit_range(&harness.state().session);
    assert!(
        range.is_some(),
        "clicking the output range strip should pin a manual Vd range"
    );
    assert!(
        !harness
            .get_by_label("Reset to Auto")
            .accesskit_node()
            .is_disabled(),
        "Reset to Auto should become available after a strip click pins the range"
    );
}

#[test]
fn output_range_strip_dragging_full_rail_moves_tiny_pchannel_window() {
    let mut session = Session::new();
    session.add_curve(curve("device_a.csv"));
    assert!(attach_output(
        &mut session,
        pchannel_far_output_dataset("device_a_output.csv")
    )
    .is_none());

    let mut harness = Harness::builder()
        .with_size(egui::vec2(650.0, 350.0))
        .build_eframe(|cc| {
            paramex_gui::theme::install(&cc.egui_ctx);
            OutputPlotHarnessApp {
                session,
                edits: EditBuffers::default(),
                cache: output_plot::OutputPlotCache::default(),
            }
        });
    harness.run();
    harness.run();

    assert_eq!(selected_fit_range(&harness.state().session), None);

    let (left, _, y) = output_range_strip_span(&mut harness);
    harness.drag_at(egui::pos2(left as f32 + 100.0, y as f32));
    harness.hover_at(egui::pos2(left as f32 + 110.0, y as f32));
    harness.run();
    harness.drop_at(egui::pos2(left as f32 + 110.0, y as f32));
    harness.run();

    let range = selected_fit_range(&harness.state().session)
        .expect("dragging the full rail should pin a manual Vd range");
    assert!(
        range.0 > -10.0,
        "tiny p-channel default window should slide when dragged from the full rail: {range:?}"
    );
}

#[test]
fn tight_output_range_keeps_both_overlapping_thumbs_draggable_without_jump() {
    for lower_thumb in [true, false] {
        let mut session = session_with_output();
        let id = session.file_ids().next().unwrap().to_string();
        session.set_output_fit_range(&id, Some((1.49, 1.51)));
        let mut harness = Harness::builder()
            .with_size(egui::vec2(650.0, 350.0))
            .build_eframe(|cc| {
                paramex_gui::theme::install(&cc.egui_ctx);
                OutputPlotHarnessApp {
                    session,
                    edits: EditBuffers::default(),
                    cache: output_plot::OutputPlotCache::default(),
                }
            });
        harness.run();
        harness.run();

        let before = selected_fit_range(&harness.state().session).unwrap();
        let (left, right, rail_y) = output_range_strip_span(&mut harness);
        let to_x = |value: f64| left as f32 + (right - left) as f32 * (value as f32 / 3.0);
        let (lower, upper) = thumb_centers(
            to_x(before.0),
            to_x(before.1),
            left as f32,
            right as f32,
            rail_y as f32,
        );
        let start = if lower_thumb { lower } else { upper };
        let end = egui::pos2(start.x + if lower_thumb { -24.0 } else { 24.0 }, start.y);
        harness.drag_at(start);
        harness.run();
        assert_eq!(
            selected_fit_range(&harness.state().session),
            Some(before),
            "pressing a separated thumb must not jump its true value"
        );
        harness.hover_at(end);
        harness.run();
        harness.drop_at(end);
        harness.run();

        let after = selected_fit_range(&harness.state().session).unwrap();
        if lower_thumb {
            assert!(after.0 < before.0 && (after.1 - before.1).abs() < 1.0e-9);
        } else {
            assert!(after.1 > before.1 && (after.0 - before.0).abs() < 1.0e-9);
        }
    }
}
