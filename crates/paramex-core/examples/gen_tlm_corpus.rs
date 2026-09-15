//! Synthetic TLM corpus + oracle generator. ALL data produced here is synthetic.
//!
//! Run: `cargo run -p paramex-core --example gen_tlm_corpus`

use std::fs;
use std::path::{Path, PathBuf};

use rust_xlsxwriter::{DocProperties, ExcelDateTime, Workbook};

use paramex_core::tlm::{
    analyze_dataset, analyze_sweep, length_points_csv, load_dataset, result_csv, status_csv,
    sweep_csv,
};

/// (group name, R_total in ohm at L = [50, 80, 120, 160] µm).
/// process_a / process_c are deliberately scattered (TLM fit R² < 0.95 -> warning);
/// process_b / process_d are collinear (R² = 1.0 -> no warning). Verified by hand.
const GROUPS: [(&str, [f64; 4]); 4] = [
    ("process_a", [120_000.0, 360_000.0, 250_000.0, 520_000.0]),
    ("process_b", [140_000.0, 200_000.0, 280_000.0, 360_000.0]),
    ("process_c", [150_000.0, 140_000.0, 360_000.0, 300_000.0]),
    ("process_d", [180_000.0, 270_000.0, 390_000.0, 510_000.0]),
];
const LENGTHS: [f64; 4] = [50.0, 80.0, 120.0, 160.0];
const DEVICES: [&str; 2] = ["d1", "d2"];
/// Drain bias written into every Setup sheet (V). |VD| drives R_total = |VD|/I.
const VD: f64 = 0.5;
const VG_STEPS: usize = 81;
const VG_MIN: f64 = -40.0;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn corpus_root() -> PathBuf {
    manifest_dir().join("tests/reference/tlm/corpus")
}
fn oracle_dir() -> PathBuf {
    manifest_dir().join("tests/reference/tlm/oracle")
}

fn vg_grid() -> Vec<f64> {
    (0..VG_STEPS).map(|i| VG_MIN + 0.5 * i as f64).collect()
}

/// abs_id(vg): peaks at vg = -40 (= `peak`), ramps down toward vg = 0, stays > 0
/// so default_selected_vg lands on -40 (max median current) and every sweep point
/// has finite current.
fn id_profile(peak: f64, vg: f64) -> f64 {
    let frac = (vg / VG_MIN).clamp(0.0, 1.0); // |vg|/40 in [0,1]; 1.0 at vg = -40
    peak * (0.2 + 0.8 * frac)
}

/// One device workbook: `Setup` sheet (drain bias) + `List` sheet (vg/abs_id/abs_is).
fn write_workbook(path: &Path, peak_id: f64) {
    let mut wb = Workbook::new();
    // Fixed creation timestamp -> byte-reproducible output (no wall-clock churn).
    let created = ExcelDateTime::from_ymd(2026, 6, 15).unwrap();
    wb.set_properties(&DocProperties::new().set_creation_datetime(&created));

    let setup = wb.add_worksheet();
    setup.set_name("Setup").unwrap();
    setup.write_string(0, 0, "ParamEx synthetic setup").unwrap();
    setup.write_string(1, 0, "Channel.VName").unwrap();
    setup.write_string(1, 1, "vg").unwrap();
    setup.write_string(1, 2, "vd").unwrap();
    setup.write_string(1, 3, "is").unwrap();
    setup.write_string(2, 0, "Measurement.Bias.Source").unwrap();
    setup.write_number(2, 1, 0.0).unwrap();
    setup.write_number(2, 2, VD).unwrap(); // value under the "vd" channel
    setup.write_number(2, 3, 0.0).unwrap();

    let list = wb.add_worksheet();
    list.set_name("List").unwrap();
    list.write_string(0, 0, "vg").unwrap();
    list.write_string(0, 1, "abs_id").unwrap();
    list.write_string(0, 2, "abs_is").unwrap();
    for (i, vg) in vg_grid().into_iter().enumerate() {
        let row = (i + 1) as u32;
        let abs_id = id_profile(peak_id, vg);
        list.write_number(row, 0, vg).unwrap();
        list.write_number(row, 1, abs_id).unwrap();
        list.write_number(row, 2, abs_id * 1.1).unwrap(); // min(|id|,|is|) = abs_id
    }

    fs::create_dir_all(path.parent().unwrap()).unwrap();
    wb.save(path).unwrap();
}

fn main() {
    let root = corpus_root();
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }

    for (group, rtotals) in GROUPS {
        for (li, &length) in LENGTHS.iter().enumerate() {
            let rtotal = rtotals[li];
            let peak1 = VD / rtotal; // device 1 sits on the designed R_total (max fit)
            let peak2 = peak1 * 0.96; // device 2 lower -> median current != max
            let dir = root.join(group).join(format!("{}", length as i64));
            write_workbook(&dir.join(format!("{}.xlsx", DEVICES[0])), peak1);
            write_workbook(&dir.join(format!("{}.xlsx", DEVICES[1])), peak2);
        }
    }

    // Malformed path: workbook directly in a group folder (no numeric length
    // subfolder) -> "non-numeric channel length folder" error status row.
    write_workbook(&root.join("process_a").join("orphan.xlsx"), VD / 300_000.0);

    // Single-workbook fixture for the parse/service/report smoke tests: group
    // "grp", one length -> < 2 lengths -> NaN fit. Lives outside the corpus tree.
    let fixture = manifest_dir().join("tests/fixtures/tlm/grp/50/with_setup.xlsx");
    write_workbook(&fixture, VD / 200_000.0);

    let ds = load_dataset(&root, None).expect("loads synthetic corpus");
    let res = analyze_dataset(&ds, None);
    let swp = analyze_sweep(&ds);

    let out = oracle_dir();
    fs::create_dir_all(&out).unwrap();
    fs::write(out.join("result.csv"), result_csv(&res)).unwrap();
    fs::write(out.join("sweep.csv"), sweep_csv(&swp)).unwrap();
    fs::write(out.join("length_points.csv"), length_points_csv(&res)).unwrap();
    fs::write(out.join("status.csv"), status_csv(&res)).unwrap();

    println!("generated {} groups under {}", GROUPS.len(), root.display());
}
