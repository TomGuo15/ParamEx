use crate::transfer::report::stats::results_to_stats;
use crate::transfer::test_support::{assert_close, load_reference_in, metric_result, parse_f64};
use serde_json::Value;

fn opt_stat(v: &Value) -> Option<f64> {
    if v.is_null() {
        None
    } else {
        Some(parse_f64(v))
    }
}

fn assert_opt_close(got: Option<f64>, exp: Option<f64>, where_: &str) {
    match (got, exp) {
        (None, None) => {}
        (Some(a), Some(b)) => assert_close(a, b, 1e-9, 1e-12),
        _ => panic!("{where_}: option mismatch got={got:?} exp={exp:?}"),
    }
}

#[test]
fn results_to_stats_match_reference() {
    let g = load_reference_in("report", "stats");
    let results: Vec<_> = g["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(metric_result)
        .collect();
    let stats = results_to_stats(&results);
    let exp = g["stats"].as_array().unwrap();
    assert_eq!(stats.len(), exp.len(), "stat row count");
    for (i, (row, e)) in stats.iter().zip(exp).enumerate() {
        assert_eq!(row.scope, e["scope"].as_str().unwrap(), "scope[{i}]");
        assert_eq!(row.metric, e["metric"].as_str().unwrap(), "metric[{i}]");
        assert_eq!(
            row.count,
            e["count"].as_u64().unwrap() as usize,
            "count[{i}]"
        );
        assert_opt_close(row.mean, opt_stat(&e["mean"]), &format!("mean[{i}]"));
        assert_opt_close(row.std, opt_stat(&e["std"]), &format!("std[{i}]"));
        assert_opt_close(row.min, opt_stat(&e["min"]), &format!("min[{i}]"));
        assert_opt_close(row.median, opt_stat(&e["median"]), &format!("median[{i}]"));
        assert_opt_close(row.max, opt_stat(&e["max"]), &format!("max[{i}]"));
    }
}
