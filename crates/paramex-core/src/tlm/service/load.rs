//! TLM dataset loading orchestration.

use std::path::Path;

use crate::tlm::format::fmt_g;
use crate::tlm::parse::{
    dir_group_length, discover_workbooks, parse_workbook, path_group_length, rel_os, UnreadableDir,
    WorkbookDiscovery,
};
use crate::tlm::types::{FileStatus, Status, TlmCurve, TlmDataset, TlmParseError, VdSource};

/// Load all TLM workbooks under `root` in discovery order. Every discovered
/// workbook and every folder that could not be listed gets a status row, so a
/// permission problem in one subfolder never silently shrinks the dataset.
pub fn load_dataset(root: &Path, fallback_vd: Option<f64>) -> Result<TlmDataset, TlmParseError> {
    let discovery = discover_workbooks(root)?;
    load_discovered(root, fallback_vd, discovery)
}

fn load_discovered(
    root: &Path,
    fallback_vd: Option<f64>,
    mut discovery: WorkbookDiscovery,
) -> Result<TlmDataset, TlmParseError> {
    discovery.unreadable_dirs.sort_by_key(|dir| {
        let relative = rel_os(&dir.path, root);
        (!relative.is_empty(), relative.to_lowercase())
    });
    let mut curves: Vec<TlmCurve> = Vec::new();
    let mut statuses: Vec<FileStatus> = discovery
        .unreadable_dirs
        .iter()
        .map(|dir| unreadable_dir_status(dir, root))
        .collect();
    for wb in &discovery.workbooks {
        match parse_workbook(wb, root, fallback_vd) {
            Ok(curve) => {
                let message = if curve.vd_source() == VdSource::Fallback {
                    format!("Loaded with fallback V_D={} V", fmt_g(curve.vd()))
                } else {
                    "Loaded".to_string()
                };
                statuses.push(FileStatus {
                    file: rel_os(wb, root),
                    group: curve.group().to_string(),
                    length_um: Some(curve.length_um()),
                    status: Status::Ok,
                    message,
                    vd_source: curve.vd_source(),
                });
                curves.push(curve);
            }
            Err(exc) => {
                let (group, length_um) = path_group_length(wb, root);
                statuses.push(FileStatus {
                    file: rel_os(wb, root),
                    group,
                    length_um,
                    status: Status::Error,
                    message: exc.0,
                    vd_source: VdSource::Unread,
                });
            }
        }
    }
    // Workbook discovery is path-sorted; sort folder failures into the same
    // order so status.csv stays deterministic and follows discovery order.
    statuses.sort_by_key(|status| status.file.to_lowercase());
    if curves.is_empty() {
        if let Some(dir) = discovery.unreadable_dirs.first() {
            // With no curve to admit, the dataset constructor's generic error
            // would hide the listing failure; name it instead.
            return Err(TlmParseError(format!(
                "No valid TLM workbooks were found. {}",
                unreadable_dir_message(dir, root)
            )));
        }
    }
    TlmDataset::try_new(root.display().to_string(), curves, statuses)
}

/// Error status row for a folder whose listing failed. The root itself is
/// reported under its own name because a status identity must be a non-empty
/// relative path.
fn unreadable_dir_status(dir: &UnreadableDir, root: &Path) -> FileStatus {
    let (group, length_um) = dir_group_length(&dir.path, root);
    FileStatus {
        file: unreadable_dir_identity(dir, root),
        group,
        length_um,
        status: Status::Error,
        message: unreadable_dir_message(dir, root),
        vd_source: VdSource::Unread,
    }
}

fn unreadable_dir_identity(dir: &UnreadableDir, root: &Path) -> String {
    let rel = rel_os(&dir.path, root);
    if rel.is_empty() {
        root.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.display().to_string())
    } else {
        rel
    }
}

fn unreadable_dir_message(dir: &UnreadableDir, root: &Path) -> String {
    format!(
        "{} could not be listed: {}",
        unreadable_dir_identity(dir, root),
        dir.error
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tlm")
    }

    #[test]
    fn unlistable_folder_becomes_an_error_status_row_beside_loaded_curves() {
        let root = fixture_root();
        let discovery = WorkbookDiscovery {
            workbooks: vec![root.join("grp").join("50").join("with_setup.xlsx")],
            unreadable_dirs: vec![UnreadableDir {
                path: root.join("grp").join("80"),
                error: "access is denied".to_string(),
            }],
        };

        let dataset = load_discovered(&root, None, discovery).expect("readable curve admits");

        assert_eq!(dataset.curves().len(), 1);
        assert_eq!(
            dataset
                .statuses()
                .iter()
                .map(|status| status.file.as_str())
                .collect::<Vec<_>>(),
            vec![
                PathBuf::from("grp")
                    .join("50")
                    .join("with_setup.xlsx")
                    .to_str()
                    .unwrap(),
                PathBuf::from("grp").join("80").to_str().unwrap(),
            ]
        );
        let failed = dataset
            .statuses()
            .iter()
            .find(|status| status.status == Status::Error)
            .expect("unreadable folder is visible");
        assert_eq!(
            failed.file,
            PathBuf::from("grp").join("80").display().to_string()
        );
        assert_eq!(failed.group, "grp");
        assert_eq!(failed.length_um, Some(80.0));
        assert_eq!(failed.vd_source, VdSource::Unread);
        assert_eq!(
            failed.message,
            format!(
                "{} could not be listed: access is denied",
                PathBuf::from("grp").join("80").display()
            )
        );
    }

    #[test]
    fn unlistable_root_without_curves_names_the_listing_failure() {
        let root = fixture_root();
        let discovery = WorkbookDiscovery {
            workbooks: Vec::new(),
            unreadable_dirs: vec![UnreadableDir {
                path: root.clone(),
                error: "access is denied".to_string(),
            }],
        };

        let error = load_discovered(&root, None, discovery).unwrap_err();

        assert_eq!(
            error.0,
            "No valid TLM workbooks were found. tlm could not be listed: access is denied"
        );
    }

    #[test]
    fn no_curve_error_uses_the_first_unreadable_path() {
        let root = fixture_root();
        let discovery = WorkbookDiscovery {
            workbooks: Vec::new(),
            unreadable_dirs: vec![
                UnreadableDir {
                    path: root.join("grp").join("z"),
                    error: "z failure".to_string(),
                },
                UnreadableDir {
                    path: root.join("grp").join("a"),
                    error: "a failure".to_string(),
                },
            ],
        };

        let error = load_discovered(&root, None, discovery).unwrap_err();

        assert_eq!(
            error.0,
            format!(
                "No valid TLM workbooks were found. {} could not be listed: a failure",
                PathBuf::from("grp").join("a").display()
            )
        );
    }
}
