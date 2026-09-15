//! Transfer Cox-panel transient state.

/// One dielectric-stack layer as edit text. Parsed to `(eps_r, thickness_nm)`
/// only when the estimator runs - layer edits never recompute, so empty/invalid
/// text parses to 0.0 (-> the estimator returns NaN).
#[derive(Debug, Clone)]
pub struct LayerRow {
    eps_text: String,
    th_text: String,
}

pub const COX_ESTIMATE_PENDING_LABEL: &str = "Estimate stack C<sub>ox</sub>.";

impl LayerRow {
    pub fn new(eps_text: impl Into<String>, th_text: impl Into<String>) -> Self {
        Self {
            eps_text: eps_text.into(),
            th_text: th_text.into(),
        }
    }

    pub fn eps_text(&self) -> &str {
        &self.eps_text
    }

    pub fn th_text(&self) -> &str {
        &self.th_text
    }

    pub fn texts_mut(&mut self) -> (&mut String, &mut String) {
        (&mut self.eps_text, &mut self.th_text)
    }

    fn initial() -> Self {
        Self::new("3.9", "300")
    }

    fn added_default() -> Self {
        Self::new("3.9", "10")
    }
}

/// Cox-panel transient state: the dielectric layers + the latest estimate label/value.
/// The committed Cox value lives behind `Session::cox_nf_per_cm2`.
#[derive(Debug)]
pub struct CoxUi {
    layers: Vec<LayerRow>,
    estimate_label: String,
    estimate_value: Option<f64>,
}

impl Default for CoxUi {
    fn default() -> Self {
        CoxUi {
            // Default first layer: thermal SiO2, eps_r 3.9, 300 nm.
            layers: vec![LayerRow::initial()],
            // Sub/superscripts are `<sub>` markup rendered via `richtext`.
            estimate_label: COX_ESTIMATE_PENDING_LABEL.to_string(),
            estimate_value: None,
        }
    }
}

impl CoxUi {
    pub fn layers(&self) -> &[LayerRow] {
        &self.layers
    }

    pub fn layers_mut(&mut self) -> &mut [LayerRow] {
        &mut self.layers
    }

    pub fn can_remove_layer(&self) -> bool {
        self.layers.len() > 1
    }

    pub fn add_default_layer(&mut self) {
        // Added layers default to a thin eps_r 3.9, 10 nm film.
        self.layers.push(LayerRow::added_default());
    }

    pub fn add_layer(&mut self, layer: LayerRow) {
        self.layers.push(layer);
    }

    pub fn remove_layer(&mut self, index: usize) -> bool {
        if self.can_remove_layer() && index < self.layers.len() {
            self.layers.remove(index);
            true
        } else {
            false
        }
    }

    pub fn estimate_label(&self) -> &str {
        &self.estimate_label
    }

    pub fn estimate_value(&self) -> Option<f64> {
        self.estimate_value
    }

    pub fn set_estimate(&mut self, label: String, value: f64) {
        self.estimate_label = label;
        self.estimate_value = Some(value);
    }

    pub fn set_estimate_label(&mut self, label: String) {
        self.estimate_label = label;
    }

    /// Parse the layer edit-text into `(eps_r, thickness_nm)` for the estimator.
    /// Empty/invalid text -> 0.0, so the estimator returns NaN for any
    /// non-positive layer instead of silently skipping it.
    pub fn layers_data(&self) -> Vec<(f64, f64)> {
        self.layers
            .iter()
            .map(|r| (parse_or_zero(r.eps_text()), parse_or_zero(r.th_text())))
            .collect()
    }

    pub fn clear_estimate(&mut self) {
        self.estimate_value = None;
        self.estimate_label = COX_ESTIMATE_PENDING_LABEL.to_string();
    }
}

/// Parse a trimmed numeric input; empty/invalid -> 0.0.
pub fn parse_or_zero(text: &str) -> f64 {
    text.trim().parse::<f64>().unwrap_or(0.0)
}
