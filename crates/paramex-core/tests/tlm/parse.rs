use crate::common::tlm_fixture_dir;
use paramex_core::tlm::{parse_workbook, VdSource};

#[test]
fn parse_with_setup_reads_vd_and_sorted_curve() {
    let root = tlm_fixture_dir();
    let f = root.join("grp/50/with_setup.xlsx");
    let curve = parse_workbook(&f, &root, None).expect("parses");
    assert!(curve.vd().is_finite() && curve.vd() != 0.0);
    assert_eq!(curve.vd_source(), VdSource::Setup);
    assert!(curve
        .samples()
        .windows(2)
        .all(|window| window[0].vg() <= window[1].vg()));
    assert!(curve.samples().iter().all(|sample| {
        sample.vg().is_finite() && sample.abs_id().is_finite() && sample.abs_is().is_finite()
    }));
}
