use std::env;
use std::fs;
use std::io::{self, Write};

use paramex_core::transfer::{ParsedCurve, Session};
use serde_json::Value;

fn parse_f64(value: &Value) -> Result<f64, String> {
    if let Some(n) = value.as_f64() {
        return Ok(n);
    }
    match value.as_str() {
        Some("nan") => Ok(f64::NAN),
        Some("inf") => Ok(f64::INFINITY),
        Some("-inf") => Ok(f64::NEG_INFINITY),
        other => Err(format!("bad float encoding: {other:?}")),
    }
}

fn vec_f64(value: &Value) -> Result<Vec<f64>, String> {
    value
        .as_array()
        .ok_or_else(|| "expected float array".to_string())?
        .iter()
        .map(parse_f64)
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = env::args().nth(1) else {
        return Err(
            "usage: export_transfer_csv_from_reference <session-end-to-end-reference.json>".into(),
        );
    };
    let text = fs::read_to_string(path)?;
    let payload: Value = serde_json::from_str(&text)?;

    let mut session = Session::new();
    let curves = payload["curves"]
        .as_array()
        .ok_or("reference payload must contain a curves array")?;
    for curve in curves {
        let parsed = ParsedCurve {
            name: curve["name"]
                .as_str()
                .ok_or("curve name must be a string")?
                .to_string(),
            vg: vec_f64(&curve["vg"])?,
            id_abs: vec_f64(&curve["id_abs"])?,
            source_path: None,
        };
        session
            .add_curve(parsed)
            .ok_or("reference corpus unexpectedly contained a duplicate curve")?;
    }

    io::stdout().write_all(&session.report_bytes())?;
    Ok(())
}
