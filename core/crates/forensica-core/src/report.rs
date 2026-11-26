//! Analysis report assembled by the pipeline.

use std::io::Cursor;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::{FileFormat, Finding, ImageInput, ModuleResult, Options};

/// Basic facts about the analysed file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileInfo {
    /// File name.
    pub name: String,
    /// Size in bytes.
    pub size: u64,
    /// SHA-256 of the original bytes.
    pub sha256: String,
    /// Sniffed format.
    pub format: FileFormat,
    /// MIME type for the sniffed format.
    pub mime: String,
    /// True when the extension disagrees with the magic bytes.
    pub extension_mismatch: bool,
    /// Pixel width, when known.
    pub width: Option<u32>,
    /// Pixel height, when known.
    pub height: Option<u32>,
}

impl FileInfo {
    /// Collects file info without fully decoding the image.
    pub fn from_input(input: &ImageInput) -> Self {
        let dims = image::ImageReader::new(Cursor::new(input.bytes()))
            .with_guessed_format()
            .ok()
            .and_then(|r| r.into_dimensions().ok());
        Self {
            name: input.name().to_string(),
            size: input.bytes().len() as u64,
            sha256: input.sha256().to_string(),
            format: input.format(),
            mime: input.format().mime().to_string(),
            extension_mismatch: input.extension_mismatch(),
            width: dims.map(|d| d.0),
            height: dims.map(|d| d.1),
        }
    }
}

/// Outcome of one module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModuleStatus {
    /// Completed.
    Ok,
    /// Errored; see `error`.
    Failed,
    /// Cancelled by the user.
    Cancelled,
}

/// One module's execution record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleRun {
    /// Module id.
    pub id: String,
    /// Module name.
    pub name: String,
    /// Module version.
    pub version: u32,
    /// Status.
    pub status: ModuleStatus,
    /// Error message for failed modules.
    pub error: Option<String>,
    /// Wall time.
    pub duration_ms: u64,
    /// Whether the result came from the cache.
    pub cached: bool,
    /// The module output.
    pub result: ModuleResult,
}

/// Full analysis report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    /// File facts incl. SHA-256 of the original.
    pub file: FileInfo,
    /// Engine version.
    pub engine_version: String,
    /// Start timestamp (RFC 3339, UTC).
    pub started_at: String,
    /// Total wall time.
    pub duration_ms: u64,
    /// Options used.
    pub options: Options,
    /// Per-module results.
    pub modules: Vec<ModuleRun>,
}

impl Report {
    /// All findings across modules, most severe first.
    pub fn findings(&self) -> Vec<&Finding> {
        let mut v: Vec<&Finding> = self.modules.iter().flat_map(|m| m.result.findings.iter()).collect();
        v.sort_by(|a, b| {
            b.severity
                .cmp(&a.severity)
                .then(b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal))
        });
        v
    }

    /// Pretty JSON (visualization bitmaps are omitted).
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    }
}

/// Current UTC time formatted as RFC 3339 (second precision).
pub fn now_rfc3339() -> String {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    format_unix(secs)
}

/// Formats seconds since the Unix epoch as `YYYY-MM-DDTHH:MM:SSZ`.
pub fn format_unix(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, (rem % 3600) / 60, rem % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_epoch() {
        assert_eq!(format_unix(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_unix(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(format_unix(1_700_000_000), "2023-11-14T22:13:20Z");
    }
}
