// Shared integration-test helpers. Each test binary compiles this module
// independently and uses only a subset, so unused-helper warnings are expected.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use egui_kittest::Harness;
use paramex_core::tlm::AnalyzedDataset;
use paramex_core::tlm::{load_dataset, TlmDataset};
use paramex_core::transfer::{OutputCurve, OutputDataset, ParsedCurve, Session};
use paramex_gui::app::ParamExApp;
use paramex_gui::state::Workspace;
use paramex_gui::workspaces::tlm::state::TlmState;

const DOUBLE: &str = include_str!("../../../paramex-core/tests/fixtures/parse/corpus_double.csv");

pub const RASTER_TEST_SCALES: [f32; 5] = [1.0, 1.25, 1.5, 1.75, 2.0];

/// Serializes every headless GPU render in a test binary. The wgpu adapter and
/// device are process-global on Windows; two threads creating or driving them
/// at once crash the whole test process with an access violation, which the
/// test runner reports as a failure with no panic message.
static WGPU_HARNESS_LOCK: Mutex<()> = Mutex::new(());

/// Hold the returned guard for the lifetime of a wgpu-backed harness when a
/// test renders more than once and the frames must not interleave with other
/// tests (snapshot pairs). Single renders use [`render`] / [`snapshot`], which
/// take the lock themselves; do not hold this guard across those calls, since
/// the lock is not reentrant. A test that panicked while holding the lock must
/// not poison every later test, so a poisoned lock is recovered rather than
/// propagated.
pub fn wgpu_guard() -> MutexGuard<'static, ()> {
    WGPU_HARNESS_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Render `harness` to an image under the GPU lock. All non-snapshot renders in
/// the suite go through here so a test cannot forget the lock.
pub fn render<State>(harness: &mut Harness<'_, State>) -> image::RgbaImage {
    let _guard = wgpu_guard();
    harness.render().expect("headless wgpu render")
}

/// Compare `harness` against the committed snapshot `name` under the GPU lock.
pub fn snapshot<State>(harness: &mut Harness<'_, State>, name: &str) {
    let _guard = wgpu_guard();
    harness.snapshot(name);
}

pub fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

pub fn crate_file(path: impl AsRef<Path>) -> PathBuf {
    crate_root().join(path)
}

pub fn core_tlm_corpus() -> PathBuf {
    crate_file("../paramex-core/tests/reference/tlm/corpus")
}

pub fn load_tlm_corpus() -> TlmDataset {
    load_dataset(&core_tlm_corpus(), None).expect("TLM corpus loads")
}

pub fn loaded_tlm_state() -> TlmState {
    let mut tlm = TlmState::default();
    tlm.install_analyzed(AnalyzedDataset::analyze(load_tlm_corpus()));
    tlm
}

pub fn loaded_transfer_app() -> ParamExApp {
    let mut session = Session::new();
    let id = session
        .add_curve(parse_transfer_fixture(DOUBLE, "corpus_double.csv"))
        .expect("fixture adds");
    assert!(session.select_file(&id));

    ParamExApp::from_session(session)
}

pub fn loaded_transfer_output_app() -> ParamExApp {
    let mut session = Session::new();
    let id = session
        .add_curve(parse_transfer_fixture(DOUBLE, "corpus_double.csv"))
        .expect("fixture adds");
    assert!(session.select_file(&id));
    assert!(session
        .replace_output(
            &id,
            OutputDataset {
                name: "corpus_double_output.csv".to_string(),
                curves: vec![OutputCurve {
                    vg: 5.0,
                    vd: vec![0.0, 1.0, 2.0, 3.0],
                    id: vec![0.0, 1.0e-6, 1.7e-6, 2.5e-6],
                }],
                source_path: None,
            },
        )
        .is_ok());

    ParamExApp::from_session(session)
}

pub fn loaded_tlm_app() -> ParamExApp {
    let mut app = ParamExApp::from_session(Session::new());
    app.set_active_workspace(Workspace::Tlm);
    app.set_tlm_state(loaded_tlm_state());
    app
}

pub fn empty_workspace_app(workspace: Workspace) -> ParamExApp {
    let mut app = ParamExApp::from_session(Session::new());
    app.set_active_workspace(workspace);
    app
}

pub fn app_harness(app: ParamExApp) -> Harness<'static, ParamExApp> {
    app_harness_at_size(app, egui::Vec2::new(1280.0, 800.0))
}

pub fn app_harness_at_size(app: ParamExApp, size: egui::Vec2) -> Harness<'static, ParamExApp> {
    app_harness_with_runs(app, size, 2)
}

pub fn app_harness_with_runs(
    app: ParamExApp,
    size: egui::Vec2,
    runs: usize,
) -> Harness<'static, ParamExApp> {
    let mut harness = Harness::builder().with_size(size).build_eframe(|cc| {
        paramex_gui::theme::install(&cc.egui_ctx);
        app
    });
    for _ in 0..runs {
        harness.run();
    }
    harness
}

// AccessKit applies egui's pixels-per-point transform at the tree root, so
// kittest node rectangles already use physical-pixel coordinates.
pub fn raster_pixel(coordinate: f32) -> i32 {
    coordinate.round() as i32
}

fn raster_rect(rect: egui::Rect) -> [i32; 4] {
    [
        raster_pixel(rect.left()),
        raster_pixel(rect.top()),
        raster_pixel(rect.right()),
        raster_pixel(rect.bottom()),
    ]
}

#[track_caller]
pub fn assert_same_raster_rect(
    label: &str,
    actual: egui::Rect,
    expected: egui::Rect,
    pixels_per_point: f32,
) {
    assert_eq!(
        raster_rect(actual),
        raster_rect(expected),
        "{label} differs by a painted pixel: actual={actual:?}, expected={expected:?}, pixels_per_point={pixels_per_point}"
    );
}

#[track_caller]
pub fn assert_same_raster_edge(label: &str, actual: f32, expected: f32, pixels_per_point: f32) {
    assert_eq!(
        raster_pixel(actual),
        raster_pixel(expected),
        "{label} differs by a painted pixel: actual={actual:.3}, expected={expected:.3}, pixels_per_point={pixels_per_point}"
    );
}

#[track_caller]
pub fn assert_same_raster_span(
    label: &str,
    actual: (f32, f32),
    expected: (f32, f32),
    pixels_per_point: f32,
) {
    let span = |(min, max): (f32, f32)| raster_pixel(max) - raster_pixel(min);
    assert_eq!(
        span(actual),
        span(expected),
        "{label} differs by a painted pixel: actual={actual:?}, expected={expected:?}, pixels_per_point={pixels_per_point}"
    );
}

#[track_caller]
pub fn assert_raster_centers_aligned(
    label: &str,
    actual: f32,
    expected: f32,
    pixels_per_point: f32,
) {
    // Odd- and even-pixel bounds cannot always share an integer center. Half a
    // physical pixel is the exact parity-balanced case; a full-pixel drift fails.
    assert!(
        (actual - expected).abs() <= 0.5,
        "{label} centers differ by more than half a physical pixel: actual={actual:.3}, expected={expected:.3}, pixels_per_point={pixels_per_point}"
    );
}

pub fn read_crate_file(path: impl AsRef<Path>) -> String {
    read(&crate_file(path))
}

pub fn visit_rs_files(dir: impl AsRef<Path>, mut f: impl FnMut(&Path, &str)) {
    let dir = dir.as_ref();
    let mut files = Vec::new();
    collect_rs_files(dir, &mut files);
    files.sort();

    for path in files {
        let text = read(&path);
        f(&path, &text);
    }
}

fn collect_rs_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
        let path = entry.expect("read entry").path();
        if path.is_dir() {
            collect_rs_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
}

pub fn parse_transfer_fixture(text: &str, name: &str) -> ParsedCurve {
    let mut vg = Vec::new();
    let mut id_abs = Vec::new();
    for line in text.lines().skip(1).filter(|line| !line.trim().is_empty()) {
        let mut it = line.split(',');
        let v: f64 = it.next().unwrap().trim().parse().unwrap();
        let i: f64 = it.next().unwrap().trim().parse().unwrap();
        vg.push(v);
        id_abs.push(i.abs());
    }
    ParsedCurve {
        name: name.to_string(),
        vg,
        id_abs,
        source_path: None,
    }
}

pub fn partial_transfer_curve(name: &str) -> ParsedCurve {
    let base = 1.0e-9_f64;
    let mut id_abs = vec![base; 12];
    id_abs[4..8].fill(f64::from_bits(base.to_bits() + 1));
    id_abs[8..].fill(f64::from_bits(base.to_bits() + 2));
    ParsedCurve {
        name: name.to_string(),
        vg: (0..12).map(f64::from).collect(),
        id_abs,
        source_path: None,
    }
}
