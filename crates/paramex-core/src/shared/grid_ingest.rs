//! Raw workbook/delimited ingest for parser modules.
//!
//! This module owns the external file-format adapters (`csv` and `calamine`) and
//! returns string grids. Domain parsers decide what those grids mean.

use std::io::Cursor;
use std::path::Path;

use calamine::{Data, Reader};

pub(crate) const MEASUREMENT_EXTENSIONS: [&str; 5] = [".csv", ".tsv", ".txt", ".xlsx", ".xls"];

/// Instrument exports may prepend long setup/analysis sections before their
/// real header.
pub(crate) const HEADER_SCAN_LIMIT: usize = 300;

/// Lower-cased extension including its leading dot, or an empty string.
pub(crate) fn normalized_extension(path: &Path) -> String {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| format!(".{}", extension.to_lowercase()))
        .unwrap_or_default()
}

/// One sheet of raw string cells. Rows may be ragged (a delimited-text reader
/// can emit variable-width rows); column accessors must bounds-check.
pub(crate) type Grid = Vec<Vec<String>>;

/// Lenient scalar coercion for domain parsers that consume raw workbook grids:
/// anything that is not a parseable number becomes NaN instead of an error.
pub(crate) fn coerce_numeric(s: &str) -> f64 {
    // Only ASCII whitespace is stripped, so a number wrapped in NBSP stays NaN;
    // the reference parses were produced under that rule.
    let t = s.trim_matches(|c: char| c.is_ascii_whitespace());
    if t.is_empty() {
        return f64::NAN;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GridReadError(pub(crate) String);

pub(crate) struct NamedGrid {
    pub(crate) name: String,
    pub(crate) grid: Grid,
}

/// Split a one-column grid whose cells hold delimited text into columns.
///
/// The first cell selects comma, tab, or collapsed-whitespace splitting. Grids
/// that already have multiple columns are cloned unchanged.
pub(crate) fn split_single_column(grid: &Grid) -> Grid {
    let one_column = !grid.is_empty() && grid.iter().all(|row| row.len() == 1);
    if !one_column {
        return grid.clone();
    }

    let first = grid
        .first()
        .and_then(|row| row.first())
        .map(String::as_str)
        .unwrap_or("");
    if first.contains(',') && !first.contains('\t') {
        return grid
            .iter()
            .map(|row| split_delimited_cell(&row[0], ','))
            .collect();
    }
    if first.contains('\t') {
        return grid
            .iter()
            .map(|row| split_delimited_cell(&row[0], '\t'))
            .collect();
    }
    grid.iter()
        .map(|row| row[0].split_whitespace().map(str::to_owned).collect())
        .collect()
}

fn split_delimited_cell(cell: &str, delimiter: char) -> Vec<String> {
    cell.split(delimiter).map(str::to_owned).collect()
}

/// True when the bytes begin with a real spreadsheet container magic: OLE2
/// (legacy `.xls`) or ZIP (`.xlsx`/`.ods`). Instrument tools routinely save a
/// plain CSV under an `.xls` name (e.g. the "I/V Sweep" exporter writes a
/// BOM+comma file as `Id-Vg-low ….xls`); those carry neither magic and must be
/// read as delimited text, not handed to calamine.
fn looks_like_workbook(content: &[u8]) -> bool {
    content.starts_with(b"\xD0\xCF\x11\xE0") // OLE2 compound file (.xls)
        || content.starts_with(b"PK\x03\x04") // ZIP local-file header (.xlsx/.ods)
        || content.starts_with(b"PK\x05\x06") // empty ZIP
        || content.starts_with(b"PK\x07\x08") // spanned ZIP
}

/// Read the ordered grids for `suffix` from a byte buffer.
pub(crate) fn read_grids(content: &[u8], suffix: &str) -> Result<Vec<Grid>, GridReadError> {
    let suffix = suffix.to_ascii_lowercase();
    if (suffix == ".xlsx" || suffix == ".xls") && looks_like_workbook(content) {
        let sheets = read_named_excel_sheets(content, |_| true)?;
        return Ok(sheets.into_iter().map(|sheet| sheet.grid).collect());
    }
    // A `.xls`/`.xlsx` whose bytes are not a real workbook is a delimited-text
    // export misnamed by the instrument; fall back to the comma reader.
    Ok(vec![read_delimited(content, &suffix)?])
}

/// Read named Excel sheets into grids, preserving workbook sheet order.
pub(crate) fn read_named_excel_sheets(
    content: &[u8],
    keep: impl Fn(&str) -> bool,
) -> Result<Vec<NamedGrid>, GridReadError> {
    let cursor = Cursor::new(content);
    let mut workbook = calamine::open_workbook_auto_from_rs(cursor)
        .map_err(|e| GridReadError(format!("Could not open workbook: {e}")))?;
    let sheet_names = workbook.sheet_names().to_vec();

    let mut grids = Vec::new();
    for name in sheet_names {
        if !keep(&name) {
            continue;
        }
        // A kept sheet that is listed but whose range fails to read is a hard error
        // (corrupt/encrypted sheet); the caller surfaces the cause. Dropping it
        // silently would misreport a present-but-unreadable sheet as a missing one.
        // (Non-kept sheets are skipped above, so an unreadable sheet the caller
        // never wanted cannot fail the read.)
        let range = workbook
            .worksheet_range(&name)
            .map_err(|e| GridReadError(format!("Could not read sheet {name}: {e}")))?;
        let grid = range
            .rows()
            .map(|row| row.iter().map(cell_to_string).collect())
            .collect();
        grids.push(NamedGrid { name, grid });
    }
    Ok(grids)
}

/// Read a delimited-text buffer into a single-sheet grid. `,` for csv/txt, `\t`
/// for tsv; rows are flexible-width and unparsed verbatim.
///
/// Blank lines are dropped: the `csv` crate already skips truly-empty lines,
/// and we additionally drop any row whose every cell is whitespace-only, so a
/// blank separator line never becomes a data row.
fn read_delimited(content: &[u8], suffix: &str) -> Result<Grid, GridReadError> {
    let delimiter = if suffix == ".tsv" { b'\t' } else { b',' };
    // Strip a leading UTF-8 BOM (Excel "Save As CSV UTF-8" writes EF BB BF), then
    // decode lossily: a stray non-UTF-8 byte (e.g. a Latin-1 `µ`/`°`/`±` in an
    // instrument export's units) becomes U+FFFD instead of making the `csv`
    // reader error on that record; an errored record would drop a data point
    // and shift the curve with no warning. `from_utf8_lossy` is borrow-only
    // (no allocation) for already-valid UTF-8, so normal files are unchanged.
    let content = content.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(content);
    let text = String::from_utf8_lossy(content);
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .delimiter(delimiter)
        .from_reader(text.as_bytes());
    let mut grid = Vec::new();
    for record in reader.records() {
        let record =
            record.map_err(|e| GridReadError(format!("Could not read delimited data: {e}")))?;
        let row: Vec<String> = record.iter().map(str::to_string).collect();
        if row.iter().all(|c| c.trim().is_empty()) {
            continue;
        }
        grid.push(row);
    }
    Ok(grid)
}

/// Convert a calamine cell to its grid string.
fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(s) | Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
        Data::Float(f) => format!("{f}"),
        Data::Int(i) => format!("{i}"),
        Data::Bool(b) => b.to_string(),
        // A numeric cell that merely carries a date/time/duration number format
        // arrives as DateTime/DateTimeIso/DurationIso; read its serial/ISO text
        // (never a Debug dump) so the Transfer and TLM readers coerce identical
        // cells the same way. A Debug dump would coerce to NaN and silently drop
        // a date-styled List/Setup value.
        Data::DateTime(dt) => format!("{}", dt.as_f64()),
        Data::Error(e) => format!("{e:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_styled_cells_read_as_values_not_debug_dumps() {
        assert_eq!(cell_to_string(&Data::Float(45000.0)), "45000");
        assert_eq!(cell_to_string(&Data::Int(7)), "7");
        assert_eq!(
            cell_to_string(&Data::DateTimeIso("2023-11-30".into())),
            "2023-11-30"
        );
        // A date/time-styled numeric cell arrives as DateTime(serial); it must read
        // as the serial number, not a Debug dump that would coerce to NaN.
        let dt = Data::DateTime(calamine::ExcelDateTime::new(
            45000.0,
            calamine::ExcelDateTimeType::DateTime,
            false,
        ));
        assert_eq!(cell_to_string(&dt), "45000");
        assert_eq!(coerce_numeric(&cell_to_string(&dt)), 45000.0);
    }

    #[test]
    fn xls_named_csv_export_falls_back_to_delimited_text() {
        // The "I/V Sweep" exporter writes a BOM+comma CSV under an `.xls` name.
        // calamine can't open it (no OLE2/ZIP magic); read_grids must read it as
        // delimited text instead of erroring — otherwise a third of the corpus
        // (every misnamed `.xls`) fails to load.
        let bytes = b"\xEF\xBB\xBFI/V Sweep,Id-Vg-low\r\nvg,vd,id\r\n-1,0.1,1e-9\r\n0,0.1,2e-9\r\n";
        let grids = read_grids(bytes, ".xls").expect("misnamed-xls CSV falls back to text");
        assert_eq!(grids[0][0], vec!["I/V Sweep", "Id-Vg-low"]);
        assert_eq!(grids[0][1], vec!["vg", "vd", "id"]);
        assert_eq!(grids[0][2], vec!["-1", "0.1", "1e-9"]);
    }

    #[test]
    fn real_workbook_magic_still_routes_to_calamine() {
        // ZIP-magic bytes are NOT diverted to the text reader: they take the
        // workbook path and error in calamine (truncated archive). Guards against
        // the fallback swallowing genuine `.xlsx`/`.xls` files.
        assert!(read_grids(b"PK\x03\x04not-a-real-xlsx", ".xlsx").is_err());
        assert!(read_grids(b"\xD0\xCF\x11\xE0not-a-real-xls", ".xls").is_err());
    }

    #[test]
    fn malformed_delimited_data_is_preserved_instead_of_dropped() {
        // The csv reader keeps an unclosed quote as a field; the ingest layer
        // must preserve that row so later scientific validation can reject it.
        let bytes = b"vg,id\n0,1\n\"1,2\n";
        let grids = read_grids(bytes, ".csv").expect("delimited input reads");
        assert_eq!(grids[0].last(), Some(&vec!["1,2\n".to_owned()]));
    }
}
