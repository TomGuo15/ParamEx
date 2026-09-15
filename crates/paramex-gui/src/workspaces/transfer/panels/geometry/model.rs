//! Geometry card commit policy and deferred commands.

use egui_notify::Toasts;
use paramex_core::transfer::{GeometryError, Session};

use crate::format_ui::{global_wl_message, WL_NUMERIC_MESSAGE, WL_POSITIVE_MESSAGE};
use crate::state::EditBuffers;

/// Commit one per-file W/L edit through `Session`: preserve the unedited
/// dimension (`None`), set the source to manual, and recompute just this file.
/// Returns [`GeometryError`] without mutating on failure.
pub fn commit_row_geometry(
    session: &mut Session,
    file_id: &str,
    width: Option<f64>,
    length: Option<f64>,
) -> Result<(), GeometryError> {
    session
        .set_manual_geometry(file_id, width, length)
        .map(|_| ())
}

/// Deferred geometry-card actions, collected during render and applied after.
pub(super) enum Cmd {
    /// One per-file cell commit; the unedited dimension stays `None`.
    RowGeometry {
        file_id: String,
        width: Option<f64>,
        length: Option<f64>,
    },
    /// "Apply W/L to All Files" with both global inputs parsed.
    ApplyGlobalWl { width: f64, length: f64 },
    /// A committed field held text that is not a number.
    RejectNonNumeric,
}

/// "Apply W/L to All Files" is authoritative for its frame. Clicking it steals
/// focus from a per-file field, whose lost_focus then commits its stale text the
/// same frame; the shared changed-text guard cannot catch that, because the stale
/// buffer does differ from the just-applied value. Drop those row commits.
fn drop_rows_superseded_by_global_apply(cmds: &mut Vec<Cmd>) {
    if cmds
        .iter()
        .any(|cmd| matches!(cmd, Cmd::ApplyGlobalWl { .. }))
    {
        cmds.retain(|cmd| !matches!(cmd, Cmd::RowGeometry { .. }));
    }
}

pub(super) fn apply_commands(
    session: &mut Session,
    edits: &mut EditBuffers,
    toasts: &mut Toasts,
    mut cmds: Vec<Cmd>,
) {
    drop_rows_superseded_by_global_apply(&mut cmds);
    for cmd in cmds {
        match cmd {
            Cmd::RowGeometry {
                file_id,
                width,
                length,
            } => {
                if commit_row_geometry(session, &file_id, width, length).is_err() {
                    toasts.warning(WL_POSITIVE_MESSAGE);
                }
            }
            Cmd::ApplyGlobalWl { width, length } => match session.set_global_wl(width, length) {
                Ok(count) => {
                    // The apply changed every file's committed W/L; drop the per-file
                    // buffers so every field re-seeds from the new value next frame.
                    edits.forget_prefix("geom:");
                    toasts.success(global_wl_message(count));
                }
                Err(_message) => {
                    toasts.warning(WL_POSITIVE_MESSAGE);
                }
            },
            Cmd::RejectNonNumeric => {
                toasts.warning(WL_NUMERIC_MESSAGE);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_apply_drops_same_frame_row_commits() {
        let mut cmds = vec![
            Cmd::ApplyGlobalWl {
                width: 700.0,
                length: 50.0,
            },
            Cmd::RowGeometry {
                file_id: "file-1".to_string(),
                width: Some(1500.0),
                length: None,
            },
        ];
        drop_rows_superseded_by_global_apply(&mut cmds);
        assert_eq!(cmds.len(), 1);
        assert!(matches!(cmds[0], Cmd::ApplyGlobalWl { .. }));
    }

    #[test]
    fn row_commits_without_a_global_apply_are_kept() {
        let mut cmds = vec![
            Cmd::RowGeometry {
                file_id: "file-1".to_string(),
                width: Some(1500.0),
                length: None,
            },
            Cmd::RejectNonNumeric,
        ];
        drop_rows_superseded_by_global_apply(&mut cmds);
        assert_eq!(cmds.len(), 2);
    }
}
