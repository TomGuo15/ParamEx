use crate::transfer::report::schema::Cell;
use crate::transfer::report::table::{format_cell, results_to_report_sections};
use crate::transfer::report::ResultsTableColumn;
use crate::transfer::test_support::{load_reference_in, metric_result, parse_f64};
use serde_json::Value;

fn decode_cell(v: &Value) -> Cell {
    if v.is_null() {
        Cell::Null
    } else if let Some(s) = v.as_str() {
        // The reference tags non-finite floats as strings ("nan"/"inf"/"-inf",
        // harness-wide convention; see `parse_f64`). Decode those back to a
        // `Float`; any other string is a genuine text cell.
        match s {
            "nan" | "inf" | "-inf" => Cell::Float(parse_f64(v)),
            _ => Cell::Text(s.to_string()),
        }
    } else {
        Cell::Float(parse_f64(v))
    }
}

fn column_for_key(key: &str) -> ResultsTableColumn {
    ResultsTableColumn::ALL
        .into_iter()
        .find(|column| column.key() == key)
        .unwrap_or_else(|| panic!("unknown column key {key:?}"))
}

#[test]
fn report_sections_match_reference() {
    let g = load_reference_in("report", "sections");
    let results: Vec<_> = g["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(metric_result)
        .collect();
    let sections = results_to_report_sections(&results);
    let exp = g["sections"].as_array().unwrap();
    assert_eq!(sections.len(), exp.len(), "section count");
    for (si, (sec, e)) in sections.iter().zip(exp).enumerate() {
        assert_eq!(sec.title, e["title"].as_str().unwrap(), "title[{si}]");
        let header: Vec<String> = e["header"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        assert_eq!(sec.header, header, "header[{si}]");
        let rows = e["rows"].as_array().unwrap();
        assert_eq!(sec.rows.len(), rows.len(), "section {si} row count");
        for (ri, (row, er)) in sec.rows.iter().zip(rows).enumerate() {
            let exp_row: Vec<String> = er
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect();
            assert_eq!(row, &exp_row, "section {si} row {ri}");
        }
    }
}

#[test]
fn format_cell_matches_reference() {
    let g = load_reference_in("report", "sections");
    for c in g["format_cell"].as_array().unwrap() {
        let key = c["key"].as_str().unwrap();
        let cell = decode_cell(&c["value"]);
        assert_eq!(
            format_cell(column_for_key(key), &cell),
            c["plain"].as_str().unwrap(),
            "plain {key}"
        );
    }
}
