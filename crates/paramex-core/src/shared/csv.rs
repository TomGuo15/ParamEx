//! Byte-exact CSV writing shared by every product export.
//!
//! Exports are opened in Excel on Windows, so every non-empty file starts with
//! a UTF-8 BOM (otherwise Excel decodes `µ` and `²` as ANSI) and uses CRLF row
//! terminators. Fields are quoted only when they contain the delimiter, a
//! quote, or a line break; embedded quotes are doubled.

/// The UTF-8 byte-order mark written at the start of every non-empty export.
pub(crate) const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// Write one CSV record: comma-joined minimally quoted fields plus CRLF. An
/// empty `fields` slice writes just the terminator (a blank separator row).
pub(crate) fn write_row<S: AsRef<str>>(out: &mut Vec<u8>, fields: &[S]) {
    let joined = fields
        .iter()
        .map(|f| quote_minimal(f.as_ref()))
        .collect::<Vec<_>>()
        .join(",");
    out.extend_from_slice(joined.as_bytes());
    out.extend_from_slice(b"\r\n");
}

/// Write a header row followed by data rows as a complete BOM-prefixed CSV
/// document. Returns empty bytes when there are no rows, so an empty export
/// is an empty file rather than a lone BOM.
pub(crate) fn write_table<S: AsRef<str>>(headers: &[&str], rows: &[Vec<S>]) -> Vec<u8> {
    if rows.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    out.extend_from_slice(&UTF8_BOM);
    write_row(&mut out, headers);
    for row in rows {
        write_row(&mut out, row);
    }
    out
}

fn quote_minimal(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\r') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_are_minimally_quoted_and_crlf_terminated() {
        let mut out = Vec::new();
        write_row(&mut out, &["a", "b,c", "d\"e", "f\ng"]);
        assert_eq!(out, b"a,\"b,c\",\"d\"\"e\",\"f\ng\"\r\n");
    }

    #[test]
    fn empty_row_writes_only_the_terminator() {
        let mut out = Vec::new();
        write_row::<&str>(&mut out, &[]);
        assert_eq!(out, b"\r\n");
    }

    #[test]
    fn table_starts_with_bom_and_is_empty_without_rows() {
        assert!(write_table::<String>(&["h"], &[]).is_empty());
        let bytes = write_table(&["h1", "h2"], &[vec!["1", "2"]]);
        assert!(bytes.starts_with(&UTF8_BOM));
        assert_eq!(&bytes[3..], b"h1,h2\r\n1,2\r\n");
    }
}
