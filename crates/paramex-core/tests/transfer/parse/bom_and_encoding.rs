//! Regression guards for delimited-text ingest robustness:
//! - a leading UTF-8 BOM (Excel "Save As CSV UTF-8") must not corrupt the first
//!   header cell (which silently swapped Vg/Id or rejected a valid file);
//! - a stray non-UTF-8 byte must not silently drop the whole row.

use paramex_core::transfer::parse_transfer_bytes;

const BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

#[test]
fn utf8_bom_csv_id_vg_order_is_not_axis_swapped() {
    // Column order "Id,Vg". Without BOM stripping the first header "Id" becomes
    // "\u{FEFF}Id", the labeled path fails to find Id, and the numeric fallback
    // picks (col0,col1) = (Id,Vg) -> Vg/Id axes SWAPPED with no error.
    let mut s = String::from("Id,Vg\n");
    for k in 1..=12u32 {
        s.push_str(&format!("{:e},{}\n", 1.0e-12 * k as f64, k));
    }
    let mut bytes = BOM.to_vec();
    bytes.extend_from_slice(s.as_bytes());

    let curve = parse_transfer_bytes("dev.csv", &bytes).expect("BOM-prefixed CSV should parse");
    assert_eq!(curve.vg.len(), 12, "row count");
    // Vg is the 1..=12 column; if axes were swapped, vg would be the ~1e-12 column.
    for (i, &v) in curve.vg.iter().enumerate() {
        assert!(
            (v - (i as f64 + 1.0)).abs() < 1e-9,
            "vg[{i}]={v} should be {} (axes must not be swapped)",
            i + 1
        );
    }
    assert!(
        curve.id_abs.iter().all(|&i| i < 1e-9),
        "id_abs should be the small-current (~1e-12) column"
    );
}

#[test]
fn utf8_bom_csv_vg_id_ig_is_not_rejected() {
    // Column order "Vg,Id,Ig" (Vg first). With a BOM the first header "Vg"
    // becomes "\u{FEFF}Vg", the labeled path returns None, and the 3-column
    // fallback (needs exactly 2) rejects a perfectly valid transfer curve.
    let mut s = String::from("Vg,Id,Ig\n");
    for k in 1..=12u32 {
        s.push_str(&format!(
            "{},{:e},{:e}\n",
            k,
            1.0e-12 * k as f64,
            1.0e-13 * k as f64
        ));
    }
    let mut bytes = BOM.to_vec();
    bytes.extend_from_slice(s.as_bytes());

    let curve = parse_transfer_bytes("dev.csv", &bytes)
        .expect("BOM-prefixed Vg,Id,Ig CSV should parse, not be rejected");
    assert_eq!(curve.vg.len(), 12);
    for (i, &v) in curve.vg.iter().enumerate() {
        assert!(
            (v - (i as f64 + 1.0)).abs() < 1e-9,
            "vg[{i}] should be {}",
            i + 1
        );
    }
}

#[test]
fn non_utf8_byte_preserves_the_row_instead_of_dropping_it() {
    // 13 rows; row 5 carries a stray Latin-1 byte (0xB5 'µ') in a 3rd "Tag"
    // column. The old `records().flatten()` dropped the whole record on a UTF-8
    // error; the lossy-decode fix keeps the row, so all 13 Vg/Id points survive.
    let mut bytes = b"Vg,Id,Tag\n".to_vec();
    for k in 1..=13u32 {
        bytes.extend_from_slice(format!("{},{:e},", k, 1.0e-12 * k as f64).as_bytes());
        if k == 5 {
            bytes.push(0xB5); // invalid UTF-8 byte
        } else {
            bytes.push(b't');
        }
        bytes.push(b'\n');
    }

    let curve =
        parse_transfer_bytes("dev.csv", &bytes).expect("CSV with one non-UTF-8 byte should parse");
    assert_eq!(
        curve.vg.len(),
        13,
        "all 13 rows should survive (row 5 must not be dropped)"
    );
    assert!(
        curve.vg.iter().any(|&v| (v - 5.0).abs() < 1e-9),
        "the Vg=5 row must be present"
    );
}

#[test]
fn iv_sweep_csv_misnamed_as_xls_parses() {
    // The dominant real-world format: the "I/V Sweep" instrument exporter saves a
    // BOM+comma CSV under an `.xls` name, with a metadata preamble above the real
    // `vg,vd,id,…,abs_id` header. calamine can't open it (no OLE2/ZIP magic), so
    // every such file (a third of the corpus) used to fail with "Cannot detect
    // file format". read_grids now falls back to the text reader.
    let mut bytes = BOM.to_vec();
    let mut s = String::from(
        "I/V Sweep,Id-Vg-low\nRecordTime,11/18/2024 14:04:02\nDevice ID,\n\
         Count,5\nFlag,\nRemarks,\nvg,vd,id,is,ig,sqrt_id,abs_id\n",
    );
    for k in 0..14u32 {
        let vg = -5.0 + k as f64;
        let id = 1.0e-9 * (k as f64 + 1.0);
        s.push_str(&format!("{vg},0.1,{id:e},{id:e},1e-13,1e-4,{id:e}\n"));
    }
    bytes.extend_from_slice(s.as_bytes());

    let curve = parse_transfer_bytes("Id-Vg-low [(5) ; 11_18_2024].xls", &bytes)
        .expect("BOM CSV misnamed .xls must parse via the text fallback");
    assert_eq!(curve.vg.len(), 14, "all 14 sweep rows");
    assert!(
        curve.id_abs.iter().all(|&i| i > 0.0),
        "currents are positive |Id|"
    );
}
