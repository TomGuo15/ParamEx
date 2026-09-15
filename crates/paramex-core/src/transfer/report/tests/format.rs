use crate::transfer::report::format::{
    fmt, fmt_engineering_current, fmt_power_of_ten, format_count,
};
use crate::transfer::test_support::{load_reference_in, parse_f64};
use serde_json::Value;

fn opt(v: &Value) -> Option<f64> {
    if v.is_null() {
        None
    } else {
        Some(parse_f64(v))
    }
}

#[test]
fn report_formatters_match_reference() {
    let g = load_reference_in("report", "format");
    for case in g["cases"].as_array().unwrap() {
        let v = opt(&case["value"]);
        assert_eq!(fmt(v), case["fmt"].as_str().unwrap(), "fmt({v:?})");
        assert_eq!(
            fmt_engineering_current(v),
            case["eng"].as_str().unwrap(),
            "eng({v:?})"
        );
        assert_eq!(
            fmt_power_of_ten(v),
            case["pot_text"].as_str().unwrap(),
            "pot_text({v:?})"
        );
        assert_eq!(
            format_count(v),
            case["count"].as_str().unwrap(),
            "count({v:?})"
        );
    }
}

/// Non-finite input must return the lowercase words the reference encodes
/// instead of panicking on the missing 'e' (Rust formats non-finite floats
/// without one).
#[test]
fn format_sci_2e_non_finite_returns_reference_sentinels() {
    use crate::transfer::report::format::format_sci_2e;
    assert_eq!(format_sci_2e(f64::NAN), "nan");
    assert_eq!(format_sci_2e(f64::INFINITY), "inf");
    assert_eq!(format_sci_2e(f64::NEG_INFINITY), "-inf");
}
