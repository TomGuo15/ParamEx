//! TLM workbook path discovery and group/length derivation policy.

use std::path::{Path, PathBuf};

use crate::tlm::types::TlmParseError;

/// A folder under the TLM root whose contents could not be listed, together
/// with the operating-system error text. Every workbook beneath it is
/// unknown, so the caller must surface the folder rather than skip it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::tlm) struct UnreadableDir {
    pub(in crate::tlm) path: PathBuf,
    pub(in crate::tlm) error: String,
}

/// Outcome of walking a TLM root: the workbooks found plus the folders that
/// could not be listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::tlm) struct WorkbookDiscovery {
    /// `*.xlsx` paths sorted by lower-cased relative POSIX path.
    pub(in crate::tlm) workbooks: Vec<PathBuf>,
    /// Folders whose listing failed, in walk order.
    pub(in crate::tlm) unreadable_dirs: Vec<UnreadableDir>,
}

/// One directory listing: the entry paths that could be read and the first
/// listing error, if any. Both are kept so a partially listable folder still
/// contributes its readable workbooks while the failure stays visible.
struct Listing {
    paths: Vec<PathBuf>,
    error: Option<std::io::Error>,
}

/// Recursively collect `*.xlsx` under `root`, sorted by relative POSIX path,
/// lowercased. Folders that cannot be listed are reported, not skipped.
pub(in crate::tlm) fn discover_workbooks(root: &Path) -> Result<WorkbookDiscovery, TlmParseError> {
    if !root.exists() {
        return Err(TlmParseError(format!(
            "TLM data folder does not exist: {}",
            root.display()
        )));
    }
    if !root.is_dir() {
        return Err(TlmParseError(format!(
            "TLM data path is not a folder: {}",
            root.display()
        )));
    }
    let mut discovery = discover_with(root, &mut list_dir);
    discovery
        .workbooks
        .sort_by_key(|p| rel_posix(p, root).to_lowercase());
    Ok(discovery)
}

/// Walk `root` with an injectable directory lister so listing failures can be
/// exercised without an operating-system permission fixture.
fn discover_with(root: &Path, list: &mut impl FnMut(&Path) -> Listing) -> WorkbookDiscovery {
    let mut discovery = WorkbookDiscovery {
        workbooks: Vec::new(),
        unreadable_dirs: Vec::new(),
    };
    collect_xlsx(root, list, &mut discovery);
    discovery
}

fn list_dir(dir: &Path) -> Listing {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            return Listing {
                paths: Vec::new(),
                error: Some(error),
            }
        }
    };
    let mut listing = Listing {
        paths: Vec::new(),
        error: None,
    };
    for entry in entries {
        match entry {
            Ok(entry) => listing.paths.push(entry.path()),
            Err(error) => {
                if listing.error.is_none() {
                    listing.error = Some(error);
                }
            }
        }
    }
    listing
}

fn collect_xlsx(
    dir: &Path,
    list: &mut impl FnMut(&Path) -> Listing,
    discovery: &mut WorkbookDiscovery,
) {
    let listing = list(dir);
    if let Some(error) = listing.error {
        discovery.unreadable_dirs.push(UnreadableDir {
            path: dir.to_path_buf(),
            error: error.to_string(),
        });
    }
    for path in listing.paths {
        if path.is_dir() {
            collect_xlsx(&path, list, discovery);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("xlsx"))
            == Some(true)
        {
            discovery.workbooks.push(path);
        }
    }
}

/// Relative path with `/` separators: the discovery sort key.
fn rel_posix(path: &Path, root: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    path_parts(rel).join("/")
}

/// Relative path with the OS separator (backslash on Windows): error-message
/// prefixes here, and the status.csv `file` column via `tlm::service`.
pub(in crate::tlm) fn rel_os(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Parsed `(group, length_um)` for a workbook path under a TLM root.
pub(super) fn workbook_group_length(
    path: &Path,
    root: &Path,
) -> Result<(String, f64), TlmParseError> {
    let rel = path.strip_prefix(root).map_err(|_| {
        TlmParseError(format!(
            "{} is not under TLM root {}",
            path.display(),
            root.display()
        ))
    })?;
    let parts = path_parts(rel);
    if parts.len() < 2 {
        return Err(TlmParseError(format!(
            "{} is not under length/file folders",
            rel_os(path, root)
        )));
    }
    let (group, length_name) = group_length_from_parts(&parts, root);
    // A folder literally named "nan" or "inf" parses as a non-finite f64; it is
    // rejected like any non-numeric name so the workbook surfaces an error
    // instead of poisoning the fit.
    let length_um = parse_finite_length(&length_name).ok_or_else(|| {
        TlmParseError(format!(
            "{} has non-numeric channel length folder",
            rel_os(path, root)
        ))
    })?;
    Ok((group, length_um))
}

fn group_length_from_parts(parts: &[&str], root: &Path) -> (String, String) {
    if parts.len() >= 3 {
        (parts[0].to_string(), parts[1].to_string())
    } else if !parts.is_empty() {
        (root_name(root).to_string(), parts[0].to_string())
    } else {
        (root_name(root).to_string(), String::new())
    }
}

/// `(group, length|None)` for the status row of a workbook that failed to parse.
pub(in crate::tlm) fn path_group_length(path: &Path, root: &Path) -> (String, Option<f64>) {
    let Ok(rel) = path.strip_prefix(root) else {
        return (String::new(), None);
    };
    let parts = path_parts(rel);
    if parts.len() < 2 {
        return (parts.first().copied().unwrap_or("").to_string(), None);
    }
    let (group, length_name) = group_length_from_parts(&parts, root);
    (group, parse_finite_length(&length_name))
}

/// `(group, length|None)` for the status row of a folder that could not be
/// listed. A folder's own components are the group and length levels, so the
/// root maps to its own name, `root/group` to that group, and
/// `root/group/length` to both.
pub(in crate::tlm) fn dir_group_length(dir: &Path, root: &Path) -> (String, Option<f64>) {
    let Ok(rel) = dir.strip_prefix(root) else {
        return (String::new(), None);
    };
    match path_parts(rel).as_slice() {
        [] => (root_name(root).to_string(), None),
        [group] => (group.to_string(), None),
        [group, length, ..] => (group.to_string(), parse_finite_length(length)),
    }
}

fn parse_finite_length(name: &str) -> Option<f64> {
    name.parse::<f64>().ok().filter(|l| l.is_finite())
}

fn path_parts(path: &Path) -> Vec<&str> {
    path.components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect()
}

fn root_name(root: &Path) -> &str {
    root.file_name().and_then(|n| n.to_str()).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_finite_length_folder_is_rejected() {
        let root = Path::new("root");
        // A "nan"/"inf" length folder must not parse to Some(non-finite).
        assert_eq!(
            path_group_length(Path::new("root/proc/nan/d.xlsx"), root).1,
            None
        );
        assert_eq!(
            path_group_length(Path::new("root/proc/inf/d.xlsx"), root).1,
            None
        );
        // A numeric folder still parses.
        assert_eq!(
            path_group_length(Path::new("root/proc/120/d.xlsx"), root),
            ("proc".to_string(), Some(120.0))
        );
    }

    #[test]
    fn folder_group_length_follows_the_folder_depth() {
        let root = Path::new("root");
        assert_eq!(dir_group_length(root, root), ("root".to_string(), None));
        assert_eq!(
            dir_group_length(Path::new("root/proc"), root),
            ("proc".to_string(), None)
        );
        assert_eq!(
            dir_group_length(Path::new("root/proc/80"), root),
            ("proc".to_string(), Some(80.0))
        );
        assert_eq!(
            dir_group_length(Path::new("elsewhere/proc"), root),
            (String::new(), None)
        );
    }

    /// Temporary tree removed on drop so a failing assertion leaves nothing behind.
    struct TempTree(PathBuf);

    impl TempTree {
        fn new(label: &str) -> Self {
            let unique = format!(
                "paramex-tlm-{label}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );
            let root = std::env::temp_dir().join(unique);
            std::fs::create_dir_all(&root).unwrap();
            Self(root)
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    // Denying read access to a folder needs an ACL change that a unit test
    // cannot apply and undo reliably, so the listing failure is injected while
    // the surrounding tree is real (`is_dir` still drives recursion).
    #[test]
    fn unlistable_folder_is_reported_and_its_siblings_still_load() {
        let tree = TempTree::new("unreadable");
        let root = tree.0.as_path();
        let readable = root.join("proc").join("50");
        let denied = root.join("proc").join("80");
        std::fs::create_dir_all(&readable).unwrap();
        std::fs::create_dir_all(&denied).unwrap();
        std::fs::write(readable.join("d1.xlsx"), b"").unwrap();
        std::fs::write(denied.join("d2.xlsx"), b"").unwrap();

        let denied_path = denied.clone();
        let mut lister = |dir: &Path| {
            if dir == denied_path {
                Listing {
                    paths: Vec::new(),
                    error: Some(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "access is denied",
                    )),
                }
            } else {
                list_dir(dir)
            }
        };

        let discovery = discover_with(root, &mut lister);

        assert_eq!(discovery.workbooks, vec![readable.join("d1.xlsx")]);
        assert_eq!(discovery.unreadable_dirs.len(), 1);
        assert_eq!(discovery.unreadable_dirs[0].path, denied);
        assert!(discovery.unreadable_dirs[0]
            .error
            .contains("access is denied"));
    }

    #[test]
    fn real_listing_finds_nested_workbooks_case_insensitively() {
        let tree = TempTree::new("nested");
        let root = tree.0.as_path();
        std::fs::create_dir_all(root.join("b").join("50")).unwrap();
        std::fs::create_dir_all(root.join("A").join("80")).unwrap();
        std::fs::write(root.join("b").join("50").join("d.XLSX"), b"").unwrap();
        std::fs::write(root.join("A").join("80").join("d.xlsx"), b"").unwrap();
        std::fs::write(root.join("A").join("80").join("notes.txt"), b"").unwrap();

        let discovery = discover_workbooks(root).unwrap();

        assert!(discovery.unreadable_dirs.is_empty());
        assert_eq!(
            discovery.workbooks,
            vec![
                root.join("A").join("80").join("d.xlsx"),
                root.join("b").join("50").join("d.XLSX"),
            ]
        );
    }
}
