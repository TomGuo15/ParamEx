use paramex_core::transfer::{
    axis_bounds, is_supported_measurement_path, parse_transfer_bytes, split_double_sweep,
    ParsedCurve, SweepData, Transform, WindowedFitter, SUPPORTED_EXTENSIONS,
};
use std::path::Path;

#[test]
fn transfer_facade_owns_supported_measurement_path_policy() {
    assert!(is_supported_measurement_path(Path::new("device.CSV")));
    assert!(!is_supported_measurement_path(Path::new("device")));
    assert!(!is_supported_measurement_path(Path::new("device.png")));
}

#[test]
fn transfer_facade_exports_domain_named_analysis_api() {
    let csv = b"Vg,Id
0,1e-9
1,2e-9
2,3e-9
3,4e-9
4,5e-9
5,6e-9
6,7e-9
7,8e-9
8,9e-9
9,1e-8
10,1.1e-8
11,1.2e-8
";

    let curve: ParsedCurve = parse_transfer_bytes("device.csv", csv).expect("transfer parses");
    assert!(SUPPORTED_EXTENSIONS.contains(&".csv"));
    assert_eq!(axis_bounds(&curve.vg), (0.0, 11.0));

    let (forward, backward) = split_double_sweep(&curve.vg, &curve.id_abs);
    assert_eq!(forward.vg, curve.vg);
    assert_eq!(backward.vg, vec![11.0]);

    let sweep = SweepData {
        vg: curve.vg,
        id_abs: curve.id_abs,
    };
    let fit = WindowedFitter::new(&sweep, Transform::Sqrt).fit(None);
    assert_eq!(fit.points, 12);
    assert!(fit.r2.is_finite());
}
