//! Result types: findings, visualizations and module results.

use serde::{Deserialize, Serialize};

/// How much attention a finding deserves. This is *not* a verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Neutral, informational observation.
    Info,
    /// Weak indicator; common in benign files.
    Low,
    /// Notable indicator worth a closer look.
    Medium,
    /// Strong indicator of processing or inconsistency.
    High,
}

/// A single observation produced by a module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    /// Module that produced the finding.
    pub module_id: String,
    /// Stable machine-readable code, e.g. `meta.software_tag`.
    pub code: String,
    /// Severity of the indicator.
    pub severity: Severity,
    /// Confidence that the *indicator itself* is present (0..=1), not that the image is manipulated.
    pub confidence: f32,
    /// Short title.
    pub title: String,
    /// Plain-language explanation.
    pub explanation: String,
    /// Known limitations / benign causes.
    pub limitations: String,
    /// Supporting evidence as key/value pairs.
    pub evidence: Vec<(String, String)>,
}

impl Finding {
    /// Starts building a finding.
    pub fn new(module_id: &str, code: &str, severity: Severity, confidence: f32, title: impl Into<String>) -> Self {
        Self {
            module_id: module_id.to_string(),
            code: code.to_string(),
            severity,
            confidence: confidence.clamp(0.0, 1.0),
            title: title.into(),
            explanation: String::new(),
            limitations: String::new(),
            evidence: Vec::new(),
        }
    }

    /// Sets the explanation.
    #[must_use]
    pub fn explain(mut self, text: impl Into<String>) -> Self {
        self.explanation = text.into();
        self
    }

    /// Sets the limitations.
    #[must_use]
    pub fn limits(mut self, text: impl Into<String>) -> Self {
        self.limitations = text.into();
        self
    }

    /// Adds a piece of evidence.
    #[must_use]
    pub fn with(mut self, key: impl Into<String>, value: impl ToString) -> Self {
        self.evidence.push((key.into(), value.to_string()));
        self
    }
}

/// What kind of visualization this is; drives how the viewer presents it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VizKind {
    /// Scalar map aligned with the image (overlay-able).
    Heatmap,
    /// An image aligned with the source (e.g. a channel, bit plane).
    Aligned,
    /// A standalone image not aligned with the source (e.g. spectrum, histogram, thumbnail).
    Standalone,
}

/// Legend for a heatmap.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Legend {
    /// Label for the low end of the scale.
    pub low: String,
    /// Label for the high end of the scale.
    pub high: String,
    /// Name of the colormap (`inferno`, `gray`, ...).
    pub colormap: String,
}

/// A line segment in source-image pixel coordinates (e.g. copy-move matches).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Line {
    /// Start x.
    pub x1: f32,
    /// Start y.
    pub y1: f32,
    /// End x.
    pub x2: f32,
    /// End y.
    pub y2: f32,
}

/// A rendered visualization (PNG encoded).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Visualization {
    /// Stable id, e.g. `ela.q90`.
    pub id: String,
    /// Title shown in the UI.
    pub title: String,
    /// How to read it.
    pub description: String,
    /// Presentation kind.
    pub kind: VizKind,
    /// PNG width in pixels.
    pub width: u32,
    /// PNG height in pixels.
    pub height: u32,
    /// PNG bytes. Skipped in JSON reports.
    #[serde(skip)]
    pub png: Vec<u8>,
    /// Legend (empty for non-heatmaps).
    pub legend: Legend,
    /// Optional line overlays in *source* image coordinates.
    pub lines: Vec<Line>,
}

/// Output of one [`crate::AnalysisModule`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ModuleResult {
    /// Findings, possibly empty.
    pub findings: Vec<Finding>,
    /// Visualizations, possibly empty.
    pub visualizations: Vec<Visualization>,
    /// Module-specific structured data (metadata tables, statistics...).
    pub raw: serde_json::Value,
}
