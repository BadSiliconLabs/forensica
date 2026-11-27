//! Plain data types crossing the bridge.

use forensica_core::{Finding, ModuleRun, ModuleStatus, Report, Visualization, VizKind};

/// File/session facts.
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: u32,
    pub name: String,
    pub path: String,
    pub size: i64,
    pub sha256: String,
    pub format: String,
    pub mime: String,
    pub width: u32,
    pub height: u32,
    pub decodable: bool,
    pub extension_mismatch: bool,
}

/// RGBA preview.
#[derive(Debug, Clone)]
pub struct PreviewDto {
    pub width: u32,
    pub height: u32,
    pub scale: f32,
    pub rgba: Vec<u8>,
}

/// Pixel inspector result.
#[derive(Debug, Clone)]
pub struct PixelDto {
    pub x: u32,
    pub y: u32,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub luma: f32,
    pub block_mean: f32,
}

/// Key/value evidence.
#[derive(Debug, Clone)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
}

/// A finding.
#[derive(Debug, Clone)]
pub struct FindingDto {
    pub module_id: String,
    pub code: String,
    /// info | low | medium | high
    pub severity: String,
    pub confidence: f32,
    pub title: String,
    pub explanation: String,
    pub limitations: String,
    pub evidence: Vec<KeyValue>,
}

/// A line overlay in source pixel coordinates.
#[derive(Debug, Clone)]
pub struct LineDto {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

/// A visualization.
#[derive(Debug, Clone)]
pub struct VizDto {
    pub id: String,
    pub module_id: String,
    pub title: String,
    pub description: String,
    /// heatmap | aligned | standalone
    pub kind: String,
    pub width: u32,
    pub height: u32,
    pub png: Vec<u8>,
    pub legend_low: String,
    pub legend_high: String,
    pub colormap: String,
    pub lines: Vec<LineDto>,
}

/// One module's outcome.
#[derive(Debug, Clone)]
pub struct ModuleRunDto {
    pub id: String,
    pub name: String,
    pub version: u32,
    /// ok | failed | cancelled
    pub status: String,
    pub error: Option<String>,
    pub duration_ms: i64,
    pub cached: bool,
    pub findings: Vec<FindingDto>,
    pub visualizations: Vec<VizDto>,
    pub raw_json: String,
}

/// Full report.
#[derive(Debug, Clone)]
pub struct ReportDto {
    pub sha256: String,
    pub file_name: String,
    pub engine_version: String,
    pub started_at: String,
    pub duration_ms: i64,
    pub modules: Vec<ModuleRunDto>,
}

/// Streamed analysis event: progress updates, then one event with `report` set.
#[derive(Debug, Clone)]
pub struct AnalysisEvent {
    pub module_id: String,
    /// started | running | finished | failed | cancelled | done
    pub stage: String,
    pub completed: u32,
    pub total: u32,
    pub module_fraction: f32,
    pub report: Option<ReportDto>,
}

/// Module metadata for settings.
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: u32,
}

/// User options.
#[derive(Debug, Clone)]
pub struct AnalysisOptions {
    /// 0 = full resolution.
    pub max_pixels: u32,
    pub ela_quality: u8,
    /// Empty = all.
    pub modules: Vec<String>,
    pub extra: Vec<KeyValue>,
}

pub(crate) fn severity_str(s: forensica_core::Severity) -> String {
    format!("{s:?}").to_lowercase()
}

impl From<&Finding> for FindingDto {
    fn from(f: &Finding) -> Self {
        Self {
            module_id: f.module_id.clone(),
            code: f.code.clone(),
            severity: severity_str(f.severity),
            confidence: f.confidence,
            title: f.title.clone(),
            explanation: f.explanation.clone(),
            limitations: f.limitations.clone(),
            evidence: f.evidence.iter().map(|(k, v)| KeyValue { key: k.clone(), value: v.clone() }).collect(),
        }
    }
}

pub(crate) fn viz_dto(module_id: &str, v: &Visualization) -> VizDto {
    VizDto {
        id: v.id.clone(),
        module_id: module_id.to_string(),
        title: v.title.clone(),
        description: v.description.clone(),
        kind: match v.kind {
            VizKind::Heatmap => "heatmap",
            VizKind::Aligned => "aligned",
            VizKind::Standalone => "standalone",
        }
        .to_string(),
        width: v.width,
        height: v.height,
        png: v.png.clone(),
        legend_low: v.legend.low.clone(),
        legend_high: v.legend.high.clone(),
        colormap: v.legend.colormap.clone(),
        lines: v.lines.iter().map(|l| LineDto { x1: l.x1, y1: l.y1, x2: l.x2, y2: l.y2 }).collect(),
    }
}

impl From<&ModuleRun> for ModuleRunDto {
    fn from(m: &ModuleRun) -> Self {
        Self {
            id: m.id.clone(),
            name: m.name.clone(),
            version: m.version,
            status: match m.status {
                ModuleStatus::Ok => "ok",
                ModuleStatus::Failed => "failed",
                ModuleStatus::Cancelled => "cancelled",
            }
            .to_string(),
            error: m.error.clone(),
            duration_ms: m.duration_ms as i64,
            cached: m.cached,
            findings: m.result.findings.iter().map(FindingDto::from).collect(),
            visualizations: m.result.visualizations.iter().map(|v| viz_dto(&m.id, v)).collect(),
            raw_json: serde_json::to_string(&m.result.raw).unwrap_or_else(|_| "null".into()),
        }
    }
}

impl From<&Report> for ReportDto {
    fn from(r: &Report) -> Self {
        Self {
            sha256: r.file.sha256.clone(),
            file_name: r.file.name.clone(),
            engine_version: r.engine_version.clone(),
            started_at: r.started_at.clone(),
            duration_ms: r.duration_ms as i64,
            modules: r.modules.iter().map(ModuleRunDto::from).collect(),
        }
    }
}

/// PRNU comparison result.
#[derive(Debug, Clone)]
pub struct PrnuDto {
    /// Peak-to-correlation energy.
    pub pce: f32,
    pub peak_x: i32,
    pub peak_y: i32,
    pub references: u32,
    pub skipped: u32,
    pub crop: u32,
    /// Usual decision threshold (60).
    pub threshold: f32,
}

/// A near-duplicate.
#[derive(Debug, Clone)]
pub struct SimilarDto {
    pub path: String,
    pub phash_distance: u32,
    pub dhash_distance: u32,
    pub width: u32,
    pub height: u32,
}

/// Result of comparing with another version.
#[derive(Debug, Clone)]
pub struct CompareDto {
    pub viz: VizDto,
    pub findings: Vec<FindingDto>,
}
