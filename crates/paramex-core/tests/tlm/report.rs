use crate::common::tlm_fixture_dir;
use paramex_core::tlm::{analyze_dataset, load_dataset, result_csv, status_csv};

const UTF8_BOM: &str = "\u{feff}";

fn text(bytes: Vec<u8>) -> String {
    let text = String::from_utf8(bytes).expect("CSV is UTF-8");
    text.strip_prefix(UTF8_BOM)
        .expect("CSV starts with a UTF-8 BOM")
        .to_owned()
}

#[test]
fn result_csv_header_and_nan_empty() {
    let ds = load_dataset(&tlm_fixture_dir(), None).unwrap();
    let res = analyze_dataset(&ds, None);
    let csv = text(result_csv(&res));
    let header = csv.lines().next().unwrap();
    assert_eq!(
        header,
        "group,selected_vg,Rcontact_script_ohm,Rc_per_contact_ohm,slope_ohm_per_um,r_squared,Rcontact_median_ohm,Rc_per_contact_median_ohm,slope_median_ohm_per_um,r_squared_median,valid_lengths,warnings"
    );
    // single-length group -> NaN fit -> empty cells for the fit columns
    let row = csv.lines().nth(1).unwrap();
    assert!(row.starts_with("grp,")); // group name present
    assert!(row.contains(",,")); // NaN rendered as empty
}

#[test]
fn status_csv_header() {
    let ds = load_dataset(&tlm_fixture_dir(), None).unwrap();
    let res = analyze_dataset(&ds, None);
    let csv = text(status_csv(&res));
    assert_eq!(
        csv.lines().next().unwrap(),
        "file,group,length_um,status,message,vd_source"
    );
}

#[test]
fn tlm_exports_use_the_shared_bom_crlf_layout() {
    let ds = load_dataset(&tlm_fixture_dir(), None).unwrap();
    let res = analyze_dataset(&ds, None);
    let bytes = result_csv(&res);

    assert!(bytes.starts_with(&[0xEF, 0xBB, 0xBF]));
    let body = std::str::from_utf8(&bytes[3..]).unwrap();
    assert!(body.ends_with("\r\n"));
    assert!(!body.replace("\r\n", "").contains('\n'));
    // The single-length group carries the two-length warning, which contains a
    // comma-free sentence and no quotes, so every field stays unquoted.
    let warning_field = body.lines().nth(1).unwrap().rsplit(',').next().unwrap();
    assert!(warning_field.contains("At least two valid lengths"));
    assert!(!warning_field.starts_with('"'));
}
