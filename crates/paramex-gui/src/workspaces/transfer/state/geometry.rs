//! Transfer geometry-panel transient state.

/// Geometry-panel transient state. Per-file W/L edits live behind `EditBuffers`;
/// these two are the persistent global-apply inputs (no committed mirror).
#[derive(Debug)]
pub struct GeometryUi {
    global_w: String,
    global_l: String,
}

impl Default for GeometryUi {
    fn default() -> Self {
        // Global W defaults to 1500 um and global L to 50 um (the extraction defaults).
        GeometryUi {
            global_w: "1500".to_string(),
            global_l: "50".to_string(),
        }
    }
}

impl GeometryUi {
    pub fn with_global_inputs(global_w: impl Into<String>, global_l: impl Into<String>) -> Self {
        Self {
            global_w: global_w.into(),
            global_l: global_l.into(),
        }
    }

    pub fn global_w(&self) -> &str {
        &self.global_w
    }

    pub fn global_l(&self) -> &str {
        &self.global_l
    }

    pub fn global_wl_mut(&mut self) -> (&mut String, &mut String) {
        (&mut self.global_w, &mut self.global_l)
    }

    pub fn parse_global_wl(&self) -> Option<(f64, f64)> {
        match (
            self.global_w.trim().parse::<f64>(),
            self.global_l.trim().parse::<f64>(),
        ) {
            (Ok(w), Ok(l)) => Some((w, l)),
            _ => None,
        }
    }
}
