use crate::transfer::report::schema::{results_to_rows, Cell, COLUMNS};
use crate::transfer::report::ResultsTableColumn;
use crate::transfer::test_support::{load_reference_in, metric_result, parse_f64};
use serde_json::Value;

fn decode_cell(v: &Value) -> Cell {
    if v.is_null() {
        Cell::Null
    } else if v.is_string() {
        Cell::Text(v.as_str().unwrap().to_string())
    } else {
        Cell::Float(parse_f64(v))
    }
}

fn assert_cell_eq(got: &Cell, exp: &Cell, where_: &str) {
    match (got, exp) {
        (Cell::Float(a), Cell::Float(b)) => {
            assert!(a == b || (a.is_nan() && b.is_nan()), "{where_}: {a} != {b}");
        }
        _ => assert_eq!(got, exp, "{where_}"),
    }
}

#[test]
fn column_keys_match_reference() {
    let g = load_reference_in("report", "schema");
    let expected: Vec<&str> = g["column_keys"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let keys: Vec<&str> = COLUMNS.iter().map(|c| c.column.key()).collect();
    assert_eq!(keys, expected);
}

#[test]
fn column_specs_follow_canonical_column_order() {
    let specs: Vec<ResultsTableColumn> = COLUMNS.iter().map(|c| c.column).collect();
    assert_eq!(specs, ResultsTableColumn::ALL);
}

#[test]
fn results_to_rows_match_reference() {
    let g = load_reference_in("report", "schema");
    let results: Vec<_> = g["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(metric_result)
        .collect();
    let rows = results_to_rows(&results);
    let exp_rows = g["rows"].as_array().unwrap();
    assert_eq!(rows.len(), exp_rows.len(), "row count");
    for (ri, (row, exp_row)) in rows.iter().zip(exp_rows).enumerate() {
        let exp: Vec<Cell> = exp_row
            .as_array()
            .unwrap()
            .iter()
            .map(decode_cell)
            .collect();
        assert_eq!(row.len(), exp.len(), "row {ri} width");
        for (ci, (got, e)) in row.iter().zip(&exp).enumerate() {
            assert_cell_eq(got, e, &format!("row {ri} col {ci}"));
        }
    }
}
