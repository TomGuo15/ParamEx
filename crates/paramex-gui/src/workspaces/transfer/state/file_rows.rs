//! Transfer's arrival-ordered loaded-file and persistent ingest-error rows.

#[derive(Debug, Clone, PartialEq, Eq)]
enum Row {
    File(String),
    Error {
        id: String,
        name: String,
        message: String,
    },
}

/// Render-ready row projection for the Transfer file list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileRow<'a> {
    File {
        id: &'a str,
    },
    Error {
        id: &'a str,
        name: &'a str,
        message: &'a str,
    },
}

/// Persistent Transfer file-list presentation state.
#[derive(Default)]
pub struct FileRows {
    next_error_id: u64,
    // Row counts are small, so linear scans over one Vec keep arrival order for free.
    rows: Vec<Row>,
}

impl FileRows {
    pub fn record_file(&mut self, file_id: String) {
        self.rows.push(Row::File(file_id));
    }

    pub fn record_error(&mut self, name: String, message: String) {
        let error_id = format!("ingest-error-{}", self.next_error_id);
        self.next_error_id += 1;
        self.rows.push(Row::Error {
            id: error_id,
            name,
            message,
        });
    }

    pub fn rows(&self) -> impl Iterator<Item = FileRow<'_>> + '_ {
        self.rows.iter().map(|row| match row {
            Row::File(id) => FileRow::File { id },
            Row::Error { id, name, message } => FileRow::Error { id, name, message },
        })
    }

    pub fn has_errors(&self) -> bool {
        self.rows.iter().any(|row| matches!(row, Row::Error { .. }))
    }

    pub fn clear_errors(&mut self) {
        self.rows.retain(|row| matches!(row, Row::File(_)));
    }

    pub fn dismiss_error(&mut self, error_id: &str) -> bool {
        let Some(index) = self
            .rows
            .iter()
            .position(|row| matches!(row, Row::Error { id, .. } if id == error_id))
        else {
            return false;
        };
        self.rows.remove(index);
        true
    }

    pub fn prune_file_rows(&mut self, file_exists: impl Fn(&str) -> bool) {
        self.rows.retain(|row| match row {
            Row::File(id) => file_exists(id),
            Row::Error { .. } => true,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_and_failures_keep_arrival_order() {
        let mut rows = FileRows::default();
        rows.record_file("first".to_owned());
        rows.record_error("bad.csv".to_owned(), "bad header".to_owned());
        rows.record_file("last".to_owned());

        assert!(matches!(
            rows.rows().collect::<Vec<_>>().as_slice(),
            [
                FileRow::File { id: "first" },
                FileRow::Error {
                    name: "bad.csv",
                    message: "bad header",
                    ..
                },
                FileRow::File { id: "last" }
            ]
        ));
    }

    #[test]
    fn dismiss_and_clear_remove_error_rows() {
        let mut rows = FileRows::default();
        rows.record_file("kept".to_owned());
        rows.record_error("bad.csv".to_owned(), "bad header".to_owned());

        assert!(rows.dismiss_error("ingest-error-0"));
        assert_eq!(
            rows.rows().collect::<Vec<_>>(),
            [FileRow::File { id: "kept" }]
        );

        rows.record_error("bad-again.csv".to_owned(), "bad header".to_owned());
        rows.clear_errors();
        assert!(!rows.has_errors());
        assert_eq!(
            rows.rows().collect::<Vec<_>>(),
            [FileRow::File { id: "kept" }]
        );
    }
}
