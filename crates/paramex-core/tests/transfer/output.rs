use paramex_core::shared::numerics::FLOAT_EPSILON;
use paramex_core::transfer::{
    parse_output_bytes, parse_output_file, AttachOutputOutcome, OutputCurve, OutputDataset,
    OutputFitKind, OutputFitStatus, OutputSummary, Session,
};

use crate::{expect_attached, transfer_curve};

fn extract_output_summary(
    dataset: &OutputDataset,
    fit_range: Option<(f64, f64)>,
) -> Option<OutputSummary> {
    let mut session = Session::new();
    let file_id = session
        .add_curve(transfer_curve("__summary__.csv", 0.0))
        .expect("summary fixture adds");
    session
        .replace_output(&file_id, dataset.clone())
        .expect("fixture file exists");
    if let Some(range) = fit_range {
        assert!(session.set_output_fit_range(&file_id, Some(range)));
    }
    session
        .selected_output_file()
        .and_then(|selected| selected.summary)
}

#[test]
fn output_parser_reads_multiple_vg_curves() {
    let csv = b"Vd,Vg,Id\n0,1,0.0\n1,1,1e-6\n2,1,1.8e-6\n0,2,0.0\n1,2,2e-6\n2,2,3.6e-6\n";
    let out = parse_output_bytes("devA_id-vd.csv", csv).expect("output parses");
    assert_eq!(out.name, "devA_id-vd.csv");
    assert_eq!(out.curves.len(), 2);
    assert_eq!(out.curves[0].vg, 1.0);
    assert_eq!(out.curves[1].vg, 2.0);
    assert_eq!(out.curves[1].vd, vec![0.0, 1.0, 2.0]);
    assert_eq!(out.curves[1].id, vec![0.0, 2e-6, 3.6e-6]);
}

#[test]
fn output_parser_keeps_three_sample_admission_distinct_from_default_fit() {
    assert!(
        parse_output_bytes("two-point.csv", b"Vd,Vg,Id\n1,2,1e-6\n2,2,2e-6\n").is_err(),
        "Transfer requires at least three samples per output curve"
    );

    let mut session = Session::new();
    let id = session
        .add_curve(transfer_curve("device.csv", 0.5))
        .expect("transfer adds");
    assert!(session
        .replace_output(
            &id,
            OutputDataset {
                name: "manually-supplied.csv".to_owned(),
                source_path: None,
                curves: vec![OutputCurve {
                    vg: 2.0,
                    vd: vec![1.0, 2.0],
                    id: vec![1.0e-6, 2.0e-6],
                }],
            }
        )
        .is_ok());
    assert_eq!(
        session.output_report_rows()[1].status,
        OutputFitStatus::Ok,
        "the fitter independently accepts two distinct same-sign Vd points"
    );

    let duplicate = parse_output_bytes(
        "duplicate-vd.csv",
        b"Vd,Vg,Id\n1,2,1e-6\n1,2,2e-6\n1,2,3e-6\n",
    )
    .expect("three measured samples are admitted even when the fit is unavailable");
    assert_eq!(duplicate.curves.len(), 1);
    assert!(session.replace_output(&id, duplicate).is_ok());
    assert_eq!(
        session.output_report_rows()[1].status,
        OutputFitStatus::Unavailable
    );
}

#[test]
fn output_parser_keeps_nearly_equal_vg_curves_separate() {
    let csv = b"Vd,Vg,Id\n0,1.0000000000001,1e-6\n1,1.0000000000001,2e-6\n2,1.0000000000001,3e-6\n0,1.0000000000002,4e-6\n1,1.0000000000002,5e-6\n2,1.0000000000002,6e-6\n";
    let out = parse_output_bytes("nearly-equal-vg.csv", csv).expect("output parses");
    assert_eq!(out.curves.len(), 2);
    assert_eq!(out.curves[0].vg, 1.0000000000001);
    assert_eq!(out.curves[1].vg, 1.0000000000002);
    assert_eq!(out.curves[0].id, vec![1e-6, 2e-6, 3e-6]);
    assert_eq!(out.curves[1].id, vec![4e-6, 5e-6, 6e-6]);
}

#[test]
fn output_parser_groups_signed_zero_gate_voltages_together() {
    let csv = b"Vd,Vg,Id\n0,-0,0\n1,0,1e-6\n2,-0.0,2e-6\n";
    let out = parse_output_bytes("zero-gate.csv", csv).expect("one three-point sweep parses");
    assert_eq!(out.curves.len(), 1);
    assert_eq!(out.curves[0].vg, 0.0);
    assert_eq!(out.curves[0].vd, vec![0.0, 1.0, 2.0]);
    assert_eq!(out.curves[0].id, vec![0.0, 1e-6, 2e-6]);
}

#[test]
fn output_parser_rejects_missing_vd_id_or_vg() {
    let err = parse_output_bytes("bad.csv", b"Vd,Id\n0,0\n1,1e-6\n").unwrap_err();
    assert!(err.0.contains("No usable output curve found in bad.csv"));
}

#[test]
fn output_parser_reads_whitespace_single_column_txt() {
    let txt = b"Vd Vg Id\n0 1 0.0\n1 1 1e-6\n2 1 1.8e-6\n";
    let out = parse_output_bytes("devA.txt", txt).expect("output parses");
    assert_eq!(out.curves.len(), 1);
    assert_eq!(out.curves[0].vg, 1.0);
    assert_eq!(out.curves[0].vd, vec![0.0, 1.0, 2.0]);
    assert_eq!(out.curves[0].id, vec![0.0, 1e-6, 1.8e-6]);
}

#[test]
fn output_parser_preserves_signed_vd_and_id() {
    let csv = b"Vd,Vg,Id\n-1,1,-1e-6\n0,1,0.0\n1,1,1e-6\n";
    let out = parse_output_bytes("signed.csv", csv).expect("output parses");
    assert_eq!(out.curves[0].vd, vec![-1.0, 0.0, 1.0]);
    assert_eq!(out.curves[0].id, vec![-1e-6, 0.0, 1e-6]);
}

#[test]
fn output_parser_prefers_signed_id_over_abs_id() {
    let csv = b"Vd,Vg,abs_id,Id\n0,1,1e-6,-1e-6\n1,1,2e-6,-2e-6\n2,1,3e-6,-3e-6\n";
    let out = parse_output_bytes("signed-before-abs.csv", csv).expect("output parses");
    assert_eq!(out.curves[0].id, vec![-1e-6, -2e-6, -3e-6]);
}

#[test]
fn output_parser_retains_unfittable_gate_lines_in_the_canonical_report() {
    let csv = b"Vd,Vg,Id\n0,1,0.0\n1,1,1e-6\n2,1,1.8e-6\n0,2,0.0\n";
    let out = parse_output_bytes("short-group.csv", csv).expect("output parses");
    assert_eq!(out.curves.len(), 2);
    assert_eq!(out.curves[0].vg, 1.0);
    assert_eq!(out.curves[1].vg, 2.0);
    assert_eq!(out.curves[1].vd, vec![0.0]);

    let mut session = Session::new();
    let id = session
        .add_curve(transfer_curve("device.csv", 0.5))
        .expect("transfer adds");
    assert!(session.replace_output(&id, out).is_ok());
    assert!(session.set_output_fit_range(&id, Some((0.0, 2.0))));

    let rows = session.output_report_rows();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].fit, OutputFitKind::Family);
    assert_eq!(rows[0].status, OutputFitStatus::Partial);
    assert_eq!(rows[1].fit, OutputFitKind::Line);
    assert_eq!(rows[1].status, OutputFitStatus::Ok);
    assert_eq!(rows[2].fit, OutputFitKind::Line);
    assert_eq!(rows[2].status, OutputFitStatus::Unavailable);
    assert_eq!(rows[2].vg, 2.0);
    assert_eq!(rows[2].fit_range, Some((0.0, 2.0)));
}

#[test]
fn output_parser_finds_header_below_row_zero() {
    let csv = b"Instrument,metadata\nOperator,A\nVd,Vg,Id\n0,1,0.0\n1,1,1e-6\n2,1,1.8e-6\n";
    let out = parse_output_bytes("metadata.csv", csv).expect("output parses");
    assert_eq!(out.curves.len(), 1);
    assert_eq!(out.curves[0].vd, vec![0.0, 1.0, 2.0]);
}

#[test]
fn output_parser_finds_header_after_long_preamble() {
    let mut csv = String::new();
    for idx in 0..205 {
        csv.push_str(&format!("Preamble row {idx},metadata\n"));
    }
    csv.push_str("Vd,Vg,Id\n0,1,0.0\n1,1,1e-6\n2,1,1.8e-6\n");

    let out = parse_output_bytes("deep-header.csv", csv.as_bytes()).expect("output parses");
    assert_eq!(out.curves.len(), 1);
    assert_eq!(out.curves[0].vd, vec![0.0, 1.0, 2.0]);
}

#[test]
fn output_parser_file_sets_source_path() {
    let path =
        std::env::temp_dir().join(format!("paramex-output-source-{}.csv", std::process::id()));
    std::fs::write(&path, b"Vd,Vg,Id\n0,1,0.0\n1,1,1e-6\n2,1,1.8e-6\n")
        .expect("write output fixture");

    let out = parse_output_file(&path).expect("output parses");
    assert_eq!(out.source_path, Some(path.clone()));

    let _ = std::fs::remove_file(path);
}

#[test]
fn output_summary_aggregates_output_family_early_voltage() {
    let dataset = OutputDataset {
        name: "devA_id-vd.csv".to_string(),
        source_path: None,
        curves: vec![
            OutputCurve {
                vg: 1.0,
                vd: vec![0.0, 1.0, 2.0, 3.0],
                id: vec![0.0, 1.0e-6, 1.8e-6, 2.4e-6],
            },
            OutputCurve {
                vg: 2.0,
                vd: vec![0.0, 1.0, 2.0, 3.0],
                id: vec![0.0, 2.0e-6, 3.8e-6, 5.4e-6],
            },
        ],
    };

    let summary = extract_output_summary(&dataset, None).expect("summary");
    assert!((summary.vg_used - 1.5).abs() < 1e-12);
    assert_eq!(summary.fit_range, Some((2.0, 3.0)));
    assert!((summary.gds - 1.1e-6).abs() < 1e-12);
    assert!((summary.fit_intercept - 0.6e-6).abs() < 1e-12);
    assert!((summary.ro - (1.0 / 1.1e-6)).abs() < 1e-3);
    assert!((summary.early_voltage - 0.6875).abs() < 1e-12);
    assert!((summary.lambda - (1.0 / 0.6875)).abs() < 1e-12);
    assert_eq!(summary.fitted_lines, 2);
    assert_eq!(summary.total_lines, 2);
}

#[test]
fn output_summary_does_not_invent_one_auto_range_for_heterogeneous_lines() {
    let dataset = OutputDataset {
        name: "heterogeneous_ranges_id-vd.csv".to_string(),
        source_path: None,
        curves: vec![
            OutputCurve {
                vg: 1.0,
                vd: vec![0.0, 1.0, 2.0],
                id: vec![0.0, 1.0e-6, 2.0e-6],
            },
            OutputCurve {
                vg: 2.0,
                vd: vec![0.0, 10.0, 20.0],
                id: vec![0.0, 2.0e-6, 4.0e-6],
            },
        ],
    };

    let summary = extract_output_summary(&dataset, None).expect("both lines fit");
    assert_eq!(summary.fit_range, None);
    assert_eq!(summary.fitted_lines, 2);
    assert_eq!(summary.total_lines, 2);
}

#[test]
fn output_summary_does_not_average_incompatible_early_voltage_lines() {
    let dataset = OutputDataset {
        name: "scattered_va_id-vd.csv".to_string(),
        source_path: None,
        curves: vec![
            OutputCurve {
                vg: 1.0,
                vd: vec![0.0, 1.0, 2.0, 3.0],
                id: vec![0.0, 1.0e-6, 2.0e-6, 3.0e-6],
            },
            OutputCurve {
                vg: 2.0,
                vd: vec![0.0, 1.0, 2.0, 3.0],
                id: vec![99.0e-6, 100.0e-6, 101.0e-6, 102.0e-6],
            },
        ],
    };

    let summary = extract_output_summary(&dataset, None).expect("summary");

    assert!(
        summary.early_voltage.is_nan(),
        "a single Early voltage should not be reported when line intercepts disagree wildly"
    );
    assert!(summary.lambda.is_nan());
}

#[test]
fn output_summary_preserves_signed_idsat_and_gds() {
    let dataset = OutputDataset {
        name: "p_device_id-vd.csv".to_string(),
        source_path: None,
        curves: vec![OutputCurve {
            vg: -2.0,
            vd: vec![0.0, -1.0, -2.0, -3.0],
            id: vec![0.0, -2.0e-6, -3.8e-6, -5.4e-6],
        }],
    };

    let summary = extract_output_summary(&dataset, None).expect("summary");
    assert_eq!(summary.fit_range, Some((-2.0, -3.0)));
    assert!((summary.idsat + 5.4e-6).abs() < 1e-12);
    assert!((summary.gds - 1.6e-6).abs() < 1e-12);
    assert!((summary.fit_intercept + 0.6e-6).abs() < 1e-12);
    assert!((summary.early_voltage + 0.375).abs() < 1e-12);
}

#[test]
fn output_summary_treats_sub_epsilon_gds_as_nonzero() {
    let dataset = OutputDataset {
        name: "low_gds_id-vd.csv".to_string(),
        source_path: None,
        curves: vec![OutputCurve {
            vg: 1.0,
            vd: vec![0.0, 1.0, 2.0, 3.0],
            id: vec![1.0e-12, 1.5e-12, 2.0e-12, 2.5e-12],
        }],
    };

    let summary = extract_output_summary(&dataset, None).expect("summary");
    assert!(summary.gds.abs() < FLOAT_EPSILON);
    assert!((summary.gds - 5.0e-13).abs() < 1e-24);
    assert!((summary.ro - 2.0e12).abs() < 1.0);
    assert!((summary.early_voltage - 2.0).abs() < 1e-12);
    assert!((summary.lambda - 0.5).abs() < 1e-12);
    assert!(summary.r2.is_finite());
    assert!((summary.r2 - 1.0).abs() < 1e-12);
}

#[test]
fn output_summary_explicit_same_sign_range_stays_on_branch() {
    let dataset = OutputDataset {
        name: "mixed_sign_id-vd.csv".to_string(),
        source_path: None,
        curves: vec![OutputCurve {
            vg: -1.0,
            vd: vec![-3.0, -2.0, 2.0, 3.0],
            id: vec![-5.0e-6, -3.0e-6, 20.0e-6, 30.0e-6],
        }],
    };

    let summary = extract_output_summary(&dataset, Some((-3.0, -2.0))).expect("summary");
    assert_eq!(summary.fit_range, Some((-3.0, -2.0)));
    assert!(summary.idsat < 0.0);
    assert!((summary.idsat + 5.0e-6).abs() < 1e-12);
    assert!((summary.gds - 2.0e-6).abs() < 1e-12);
}

#[test]
fn output_summary_explicit_range_fits_distinct_vd_but_idsat_uses_duplicates() {
    let dataset = OutputDataset {
        name: "explicit_duplicate_endpoint_id-vd.csv".to_string(),
        source_path: None,
        curves: vec![OutputCurve {
            vg: 2.0,
            vd: vec![2.0, 3.0, 3.0],
            id: vec![4.0e-6, 6.0e-6, 7.0e-6],
        }],
    };

    let summary = extract_output_summary(&dataset, Some((2.0, 3.0))).expect("summary");
    assert_eq!(summary.fit_range, Some((2.0, 3.0)));
    assert!((summary.gds - 2.0e-6).abs() < 1e-12);
    assert!((summary.idsat - 7.0e-6).abs() < 1e-12);
}

#[test]
fn output_summary_uses_fitable_curve_without_claiming_a_family_auto_range() {
    let dataset = OutputDataset {
        name: "fallback_curve_id-vd.csv".to_string(),
        source_path: None,
        curves: vec![
            OutputCurve {
                vg: 9.0,
                vd: vec![3.0, 3.0, 3.0],
                id: vec![100.0e-6, 90.0e-6, 80.0e-6],
            },
            OutputCurve {
                vg: 2.0,
                vd: vec![0.0, 2.0, 3.0],
                id: vec![0.0, 4.0e-6, 6.0e-6],
            },
        ],
    };

    let summary = extract_output_summary(&dataset, None).expect("summary");
    assert_eq!(summary.vg_used, 2.0);
    assert_eq!(summary.fit_range, None);
    assert_eq!(summary.fitted_lines, 1);
    assert_eq!(summary.total_lines, 2);
    assert!(summary.gds.is_finite());
    assert!((summary.gds - 2.0e-6).abs() < 1e-12);
}

#[test]
fn output_summary_explicit_crossing_range_uses_signed_numeric_bounds() {
    let dataset = OutputDataset {
        name: "crossing_range_id-vd.csv".to_string(),
        source_path: None,
        curves: vec![OutputCurve {
            vg: 0.5,
            vd: vec![-3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0],
            id: vec![200.0e-6, 0.0, -1.0e-6, 0.0, 3.0e-6, 8.0e-6, -200.0e-6],
        }],
    };

    let summary = extract_output_summary(&dataset, Some((-2.0, 2.0))).expect("summary");
    assert_eq!(summary.fit_range, Some((-2.0, 2.0)));
    assert!((summary.gds - 2.0e-6).abs() < 1e-12);
    assert!((summary.idsat - 8.0e-6).abs() < 1e-12);
}

#[test]
fn output_summary_crossing_range_idsat_tie_break_is_order_independent() {
    let ascending = OutputDataset {
        name: "ascending_crossing_id-vd.csv".to_string(),
        source_path: None,
        curves: vec![OutputCurve {
            vg: 0.5,
            vd: vec![-2.0, -1.0, 0.0, 1.0, 2.0],
            id: vec![0.0, -1.0e-6, 0.0, 3.0e-6, 8.0e-6],
        }],
    };
    let descending = OutputDataset {
        name: "descending_crossing_id-vd.csv".to_string(),
        source_path: None,
        curves: vec![OutputCurve {
            vg: 0.5,
            vd: vec![2.0, 1.0, 0.0, -1.0, -2.0],
            id: vec![8.0e-6, 3.0e-6, 0.0, -1.0e-6, 0.0],
        }],
    };

    let asc = extract_output_summary(&ascending, Some((-2.0, 2.0))).expect("summary");
    let desc = extract_output_summary(&descending, Some((-2.0, 2.0))).expect("summary");
    assert!((asc.idsat - 8.0e-6).abs() < 1e-12);
    assert!((desc.idsat - asc.idsat).abs() < 1e-12);
    assert!((desc.early_voltage - asc.early_voltage).abs() < 1e-12);
}

#[test]
fn output_summary_default_fit_branch_is_order_independent_for_symmetric_sweeps() {
    // A bipolar sweep with exactly symmetric endpoints ties on |Vd|: the default
    // fit must land on the same (positive) branch whether the file lists the
    // sweep ascending or descending, not on whichever endpoint appears first.
    let vd: Vec<f64> = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
    let id: Vec<f64> = vec![-2.0e-6, -1.0e-6, 0.0, 3.0e-6, 8.0e-6];
    let make = |name: &str, vd: Vec<f64>, id: Vec<f64>| OutputDataset {
        name: name.to_string(),
        source_path: None,
        curves: vec![OutputCurve { vg: 0.5, vd, id }],
    };
    let ascending = make("ascending_symmetric_id-vd.csv", vd.clone(), id.clone());
    let descending = make(
        "descending_symmetric_id-vd.csv",
        vd.into_iter().rev().collect(),
        id.into_iter().rev().collect(),
    );

    let asc = extract_output_summary(&ascending, None).expect("summary");
    let desc = extract_output_summary(&descending, None).expect("summary");
    assert!((asc.idsat - 8.0e-6).abs() < 1e-12, "idsat={}", asc.idsat);
    assert!((desc.idsat - asc.idsat).abs() < 1e-12);
    assert!((desc.gds - asc.gds).abs() < 1e-18);
}

#[test]
fn output_summary_default_fit_uses_distinct_vd_when_max_endpoint_is_duplicated() {
    let dataset = OutputDataset {
        name: "duplicated_endpoint_id-vd.csv".to_string(),
        source_path: None,
        curves: vec![OutputCurve {
            vg: 2.0,
            vd: vec![0.0, 2.0, 3.0, 3.0],
            id: vec![0.0, 4.0e-6, 6.0e-6, 7.0e-6],
        }],
    };

    let summary = extract_output_summary(&dataset, None).expect("summary");
    assert_eq!(summary.fit_range, Some((2.0, 3.0)));
    assert!(summary.gds.is_finite());
    assert!((summary.gds - 2.0e-6).abs() < 1e-12);
    assert!((summary.idsat - 7.0e-6).abs() < 1e-12);
}

#[test]
fn output_csv_exports_canonical_family_and_line_rows() {
    let mut session = Session::new();
    let id = session
        .add_curve(transfer_curve("devA.csv", 0.5))
        .expect("transfer adds");
    assert!(session
        .replace_output(
            &id,
            OutputDataset {
                name: "devA_id-vd.csv".to_string(),
                source_path: None,
                curves: vec![OutputCurve {
                    vg: 2.0,
                    vd: vec![0.0, 1.0, 2.0, 3.0],
                    id: vec![0.0, 2.0e-6, 3.8e-6, 5.4e-6],
                }],
            }
        )
        .is_ok());

    let csv = String::from_utf8(session.output_report_bytes()).unwrap();
    assert!(csv.starts_with('\u{feff}'));
    assert!(csv.contains(
        "device,output_file,fit,status,Vg,Idsat,gds,ro,Early voltage,lambda,Vds fit min,Vds fit max,R2"
    ));
    assert!(csv.contains("devA.csv,devA_id-vd.csv,Family,ok"));
    assert!(csv.contains("devA.csv,devA_id-vd.csv,Line,ok,2.000000e0"));
    let mut rows = csv.lines().skip(1);
    let family: Vec<_> = rows.next().expect("family row").split(',').collect();
    let line: Vec<_> = rows.next().expect("line row").split(',').collect();
    assert!(!family[9].is_empty());
    assert_eq!(family[9], line[9]);
}

#[test]
fn output_csv_quotes_text_and_blanks_unavailable_values_through_session() {
    let mut session = Session::new();
    let id = session
        .add_curve(transfer_curve("dev,\"A\"\n", 0.5))
        .expect("transfer adds");
    assert!(session
        .replace_output(&id, unusable_output_dataset("out\rfile.csv"))
        .is_ok());

    let csv = String::from_utf8(session.output_report_bytes()).expect("UTF-8 report");
    assert!(csv.starts_with('\u{feff}'));
    assert!(csv.contains("\"dev,\"\"A\"\"\n\",\"out\rfile.csv\",Family,unavailable"));
    assert!(csv.contains("\"dev,\"\"A\"\"\n\",\"out\rfile.csv\",Line,unavailable"));
    assert_eq!(csv.matches("unavailable,,,,,,,,,\r\n").count(), 1);
    assert!(csv.contains("Line,unavailable,2.000000e0,,,,,,,,\r\n"));
    assert!(!csv.contains("NaN"));
    assert!(!csv.contains("inf"));
}

#[test]
fn output_csv_sorts_reversed_fit_range_columns() {
    let mut session = Session::new();
    let id = session
        .add_curve(transfer_curve("p_device.csv", -1.0))
        .expect("transfer adds");
    assert!(session
        .replace_output(
            &id,
            OutputDataset {
                name: "p_device_id-vd.csv".to_string(),
                source_path: None,
                curves: vec![OutputCurve {
                    vg: -2.0,
                    vd: vec![0.0, -1.0, -2.0, -3.0],
                    id: vec![0.0, -2.0e-6, -3.8e-6, -5.4e-6],
                }],
            }
        )
        .is_ok());
    assert!(session.set_output_fit_range(&id, Some((-2.0, -3.0))));
    assert_eq!(
        session.output_report_rows()[0].fit_range,
        Some((-2.0, -3.0))
    );

    let csv = String::from_utf8(session.output_report_bytes()).unwrap();
    let row = csv.lines().nth(1).expect("row");
    let cells: Vec<&str> = row.split(',').collect();
    assert_eq!(cells[10], "-3.000000e0");
    assert_eq!(cells[11], "-2.000000e0");
}

fn output_dataset(name: &str, scale: f64) -> OutputDataset {
    OutputDataset {
        name: name.to_string(),
        source_path: None,
        curves: vec![OutputCurve {
            vg: 2.0,
            vd: vec![0.0, 1.0, 2.0, 3.0],
            id: vec![0.0, 1.0e-6 * scale, 2.0e-6 * scale, 3.0e-6 * scale],
        }],
    }
}

fn unusable_output_dataset(name: &str) -> OutputDataset {
    OutputDataset {
        name: name.to_string(),
        source_path: None,
        curves: vec![OutputCurve {
            vg: 2.0,
            vd: vec![3.0, 3.0, 3.0],
            id: vec![1.0e-6, 2.0e-6, 3.0e-6],
        }],
    }
}

#[test]
fn output_attaches_to_unique_matching_transfer_file() {
    let mut s = Session::new();
    let dev_a = s.add_curve(transfer_curve("dev A.csv", 0.5)).unwrap();
    let dev_b = s.add_curve(transfer_curve("devB.csv", 1.0)).unwrap();
    let generation = s.generation();

    assert!(expect_attached(
        s.attach_output(output_dataset("dev_A output.csv", 1.0)),
        &dev_a,
    )
    .is_none());

    assert!(s.generation() > generation);
    assert!(s.select_file(&dev_a));
    let attached = s.selected_output_file().expect("selected transfer");
    assert_eq!(
        attached.output.expect("attached output").name,
        "dev_A output.csv"
    );
    assert_eq!(attached.selected_fit_range, None);
    assert!(s.select_file(&dev_b));
    assert!(s
        .selected_output_file()
        .expect("selected transfer")
        .output
        .is_none());
}

#[test]
fn output_o_suffix_attaches_to_numbered_transfer_file() {
    let mut s = Session::new();
    let id = s.add_curve(transfer_curve("2-6.xlsx", 0.5)).unwrap();

    assert!(expect_attached(s.attach_output(output_dataset("2-6o.xlsx", 1.0)), &id).is_none());
    assert_eq!(
        s.selected_output_file()
            .expect("selected transfer")
            .output
            .expect("attached output")
            .name,
        "2-6o.xlsx"
    );
}

#[test]
fn output_no_match_does_not_attach_or_mutate_generation() {
    let mut s = Session::new();
    let id = s.add_curve(transfer_curve("devA_repeat.csv", 0.5)).unwrap();
    let generation = s.generation();

    let output = output_dataset("devA_id-vd.csv", 1.0);
    assert_eq!(
        s.attach_output(output.clone()),
        AttachOutputOutcome::NoMatch { output }
    );

    assert_eq!(s.generation(), generation);
    assert_eq!(s.active_file_id(), Some(id.as_str()));
    assert!(s
        .selected_output_file()
        .expect("selected transfer")
        .output
        .is_none());
}

#[test]
fn output_ambiguous_match_does_not_attach_or_mutate_generation() {
    let mut s = Session::new();
    let a = s.add_curve(transfer_curve("dev A.csv", 0.5)).unwrap();
    let b = s.add_curve(transfer_curve("dev_A.csv", 1.0)).unwrap();
    let generation = s.generation();

    let output = output_dataset("dev-A_id-vd.csv", 1.0);
    assert_eq!(
        s.attach_output(output.clone()),
        AttachOutputOutcome::Ambiguous { output }
    );

    assert_eq!(s.generation(), generation);
    for id in [&a, &b] {
        assert!(s.select_file(id));
        assert!(s
            .selected_output_file()
            .expect("selected transfer")
            .output
            .is_none());
    }
}

#[test]
fn manual_output_attach_resolves_ambiguous_match() {
    let mut s = Session::new();
    let a = s.add_curve(transfer_curve("dev A.csv", 0.5)).unwrap();
    let b = s.add_curve(transfer_curve("dev_A.csv", 1.0)).unwrap();

    let output = match s.attach_output(output_dataset("dev-A_id-vd.csv", 1.0)) {
        AttachOutputOutcome::Ambiguous { output } => output,
        other => panic!("expected ambiguous output, got {other:?}"),
    };
    assert!(s.replace_output(&b, output).is_ok());

    assert!(s.select_file(&a));
    assert!(s
        .selected_output_file()
        .expect("selected transfer")
        .output
        .is_none());
    assert!(s.select_file(&b));
    assert_eq!(
        s.selected_output_file()
            .expect("selected transfer")
            .output
            .expect("attached output")
            .name,
        "dev-A_id-vd.csv"
    );
}

#[test]
fn reloading_output_replaces_only_matched_device() {
    let mut s = Session::new();
    let a = s.add_curve(transfer_curve("devA.csv", 0.5)).unwrap();
    let b = s.add_curve(transfer_curve("devB.csv", 1.0)).unwrap();

    assert!(expect_attached(s.attach_output(output_dataset("devA_id-vd.csv", 1.0)), &a,).is_none());
    assert!(expect_attached(s.attach_output(output_dataset("devB_id-vd.csv", 2.0)), &b,).is_none());
    let generation = s.generation();

    let displaced = expect_attached(
        s.attach_output(output_dataset("devA_output-curve.csv", 3.0)),
        &a,
    )
    .expect("different-source automatic replacement returns the prior output");
    assert_eq!(displaced.name, "devA_id-vd.csv");

    assert!(s.generation() > generation);
    assert!(s.select_file(&a));
    assert_eq!(
        s.selected_output_file()
            .expect("selected transfer")
            .output
            .expect("attached output")
            .name,
        "devA_output-curve.csv"
    );
    assert_eq!(
        s.selected_output_file()
            .expect("selected transfer")
            .selected_fit_range,
        None
    );
    assert!(s.select_file(&b));
    assert_eq!(
        s.selected_output_file()
            .expect("selected transfer")
            .output
            .expect("attached output")
            .name,
        "devB_id-vd.csv"
    );
}

#[test]
fn removing_transfer_file_removes_its_output() {
    let mut s = Session::new();
    let a = s.add_curve(transfer_curve("devA.csv", 0.5)).unwrap();
    let b = s.add_curve(transfer_curve("devB.csv", 1.0)).unwrap();
    assert!(expect_attached(s.attach_output(output_dataset("devA_id-vd.csv", 1.0)), &a,).is_none());
    assert!(expect_attached(s.attach_output(output_dataset("devB_id-vd.csv", 2.0)), &b,).is_none());

    assert!(s.select_file(&a));
    assert_eq!(s.remove_selected_or_checked(), 1);

    let rows = s.output_report_rows();
    assert_eq!(
        rows.iter()
            .filter(|row| row.fit == OutputFitKind::Family)
            .count(),
        1
    );
    assert!(rows.iter().all(|row| row.device == "devB.csv"));
}

#[test]
fn detaching_output_keeps_transfer_file_loaded() {
    let mut s = Session::new();
    let a = s.add_curve(transfer_curve("devA.csv", 0.5)).unwrap();
    assert!(expect_attached(s.attach_output(output_dataset("devA_id-vd.csv", 1.0)), &a,).is_none());
    assert!(s.set_output_fit_range(&a, Some((0.0, 1.0))));

    assert!(s.take_output(&a).is_some());

    let selected = s.selected_output_file().expect("selected transfer remains");
    assert!(selected.output.is_none());
    assert_eq!(selected.selected_fit_range, None);
    assert!(s.output_report_rows().is_empty());
    assert_eq!(s.file_count(), 1);
}

#[test]
fn clearing_files_clears_output_results() {
    let mut s = Session::new();
    let a = s.add_curve(transfer_curve("devA.csv", 0.5)).unwrap();
    assert!(expect_attached(s.attach_output(output_dataset("devA_id-vd.csv", 1.0)), &a,).is_none());

    assert_eq!(s.clear_files(), 1);

    assert!(s.output_report_rows().is_empty());
}

#[test]
fn output_report_rows_keep_unavailable_families_and_lines_in_file_order() {
    let mut s = Session::new();
    let a = s.add_curve(transfer_curve("devA.csv", 0.5)).unwrap();
    let b = s.add_curve(transfer_curve("devB.csv", 1.0)).unwrap();
    let c = s.add_curve(transfer_curve("devC.csv", 1.5)).unwrap();
    assert!(expect_attached(
        s.attach_output(unusable_output_dataset("devA_id-vd.csv")),
        &a,
    )
    .is_none());
    assert!(expect_attached(s.attach_output(output_dataset("devB_id-vd.csv", 2.0)), &b,).is_none());
    assert!(
        expect_attached(s.attach_output(output_dataset("devC output.csv", 3.0)), &c,).is_none()
    );

    let rows = s.output_report_rows();
    assert_eq!(rows.len(), 6);
    assert_eq!(rows[0].fit, OutputFitKind::Family);
    assert_eq!(rows[0].status, OutputFitStatus::Unavailable);
    assert_eq!(rows[0].device, "devA.csv");
    assert_eq!(rows[0].output_file, "devA_id-vd.csv");
    assert_eq!(rows[1].fit, OutputFitKind::Line);
    assert_eq!(rows[1].status, OutputFitStatus::Unavailable);
    assert_eq!(rows[2].fit, OutputFitKind::Family);
    assert_eq!(rows[2].status, OutputFitStatus::Ok);
    assert_eq!(rows[2].device, "devB.csv");
    assert_eq!(rows[2].output_file, "devB_id-vd.csv");
    assert_eq!(rows[3].fit, OutputFitKind::Line);
    assert_eq!(rows[4].fit, OutputFitKind::Family);
    assert_eq!(rows[4].device, "devC.csv");
    assert_eq!(rows[4].output_file, "devC output.csv");
    assert_eq!(rows[5].fit, OutputFitKind::Line);

    assert!(s.set_output_fit_range(&a, Some((0.0, 2.0))));
    let rows = s.output_report_rows();
    assert_eq!(rows[0].status, OutputFitStatus::Unavailable);
    assert_eq!(rows[0].fit_range, Some((0.0, 2.0)));
    assert_eq!(rows[1].fit_range, Some((0.0, 2.0)));
}

#[test]
fn output_report_rows_mark_partial_families_and_retain_the_failed_gate_line() {
    let mut session = Session::new();
    let id = session
        .add_curve(transfer_curve("devA.csv", 0.5))
        .expect("transfer adds");
    assert!(session
        .replace_output(
            &id,
            OutputDataset {
                name: "devA_id-vd.csv".to_string(),
                source_path: None,
                curves: vec![
                    OutputCurve {
                        vg: 1.0,
                        vd: vec![0.0, 1.0, 2.0],
                        id: vec![0.0, 1.0e-6, 2.0e-6],
                    },
                    OutputCurve {
                        vg: 2.0,
                        vd: vec![3.0, 3.0, 3.0],
                        id: vec![1.0e-6, 2.0e-6, 3.0e-6],
                    },
                ],
            }
        )
        .is_ok());

    let rows = session.output_report_rows();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].status, OutputFitStatus::Partial);
    assert_eq!(rows[0].fit_range, None);
    assert_eq!(rows[1].status, OutputFitStatus::Ok);
    assert_eq!(rows[2].status, OutputFitStatus::Unavailable);
    assert_eq!(rows[2].vg, 2.0);
    assert!(rows[2].idsat.is_nan());
    assert!(String::from_utf8(session.output_report_bytes())
        .unwrap()
        .contains("devA.csv,devA_id-vd.csv,Line,unavailable,2.000000e0"));

    assert!(session.set_output_fit_range(&id, Some((0.0, 2.0))));
    let rows = session.output_report_rows();
    assert_eq!(rows[0].status, OutputFitStatus::Partial);
    assert_eq!(rows[0].fit_range, Some((0.0, 2.0)));
    assert_eq!(rows[2].status, OutputFitStatus::Unavailable);
    assert_eq!(rows[2].fit_range, Some((0.0, 2.0)));
}
