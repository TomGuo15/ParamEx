use crate::transfer::report::csv::export_results_bytes;
use crate::transfer::test_support::{b64_decode, load_reference_in, metric_result};

#[test]
fn export_results_bytes_match_reference() {
    let g = load_reference_in("report", "csv");
    for case in g["cases"].as_array().unwrap() {
        let label = case["label"].as_str().unwrap();
        let results: Vec<_> = case["results"]
            .as_array()
            .unwrap()
            .iter()
            .map(metric_result)
            .collect();
        let got = export_results_bytes(&results);
        let expected = b64_decode(case["csv_b64"].as_str().unwrap());
        assert_eq!(
            got,
            expected,
            "CSV bytes differ for {label}\n got: {:?}\n exp: {:?}",
            String::from_utf8_lossy(&got),
            String::from_utf8_lossy(&expected),
        );
    }
}
