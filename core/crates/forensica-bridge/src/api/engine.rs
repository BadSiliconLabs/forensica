//! Engine entry points.

use std::path::Path;

use anyhow::{anyhow, Result};
use forensica_core::{report::FileInfo, Options, ProgressStage};
use forensica_engine::{self as engine, interactive};

use super::dto::viz_dto;
use super::dto::*;
use crate::frb_generated::StreamSink;

fn session(id: u32) -> Result<std::sync::Arc<engine::Session>> {
    engine::get(u64::from(id)).ok_or_else(|| anyhow!("session {id} not found (closed or evicted)"))
}

/// Opens a file read-only and returns its facts (incl. SHA-256 of the original).
pub fn open_session(path: String) -> Result<SessionInfo> {
    let s = engine::open(Path::new(&path))?;
    let info = FileInfo::from_input(&s.input);
    Ok(SessionInfo {
        id: s.id as u32,
        name: info.name,
        path,
        size: info.size as i64,
        sha256: info.sha256,
        format: format!("{:?}", info.format),
        mime: info.mime,
        width: info.width.unwrap_or(0),
        height: info.height.unwrap_or(0),
        decodable: s.input.format().decodable(),
        extension_mismatch: info.extension_mismatch,
    })
}

/// Frees a session.
#[flutter_rust_bridge::frb(sync)]
pub fn close_session(id: u32) {
    engine::close(u64::from(id));
}

/// RGBA preview (stored orientation) with the longest side ≤ `max_side`.
pub fn preview(id: u32, max_side: u32) -> Result<PreviewDto> {
    let p = interactive::preview(&*session(id)?, max_side)?;
    Ok(PreviewDto { width: p.width, height: p.height, scale: p.scale, rgba: p.rgba })
}

/// Pixel values at original coordinates.
pub fn pixel_at(id: u32, x: u32, y: u32) -> Result<PixelDto> {
    let p = interactive::pixel_at(&*session(id)?, x, y)?;
    Ok(PixelDto { x: p.x, y: p.y, r: p.r, g: p.g, b: p.b, luma: p.luma, block_mean: p.block_mean })
}

/// Lists registered modules.
#[flutter_rust_bridge::frb(sync)]
pub fn list_modules() -> Vec<ModuleInfo> {
    engine::pipeline()
        .modules()
        .iter()
        .map(|m| ModuleInfo {
            id: m.id().into(),
            name: m.name().into(),
            description: m.description().into(),
            version: m.version(),
        })
        .collect()
}

/// Engine version string.
#[flutter_rust_bridge::frb(sync)]
pub fn engine_version() -> String {
    forensica_core::ENGINE_VERSION.to_string()
}

fn to_options(o: &AnalysisOptions) -> Options {
    Options {
        max_pixels: u64::from(o.max_pixels),
        ela_quality: o.ela_quality.clamp(1, 100),
        modules: o.modules.clone(),
        extra: o.extra.iter().map(|kv| (kv.key.clone(), kv.value.clone())).collect(),
    }
}

/// Runs the analysis, streaming progress and finally the report.
pub fn analyze(id: u32, options: AnalysisOptions, sink: StreamSink<AnalysisEvent>) -> Result<()> {
    let s = session(id)?;
    let opts = to_options(&options);
    let report = s.analyze(&opts, &|e| {
        let stage = match e.stage {
            ProgressStage::Started => "started",
            ProgressStage::Running => "running",
            ProgressStage::Finished => "finished",
            ProgressStage::Failed => "failed",
            ProgressStage::Cancelled => "cancelled",
        };
        let _ = sink.add(AnalysisEvent {
            module_id: e.module_id,
            stage: stage.into(),
            completed: e.completed as u32,
            total: e.total as u32,
            module_fraction: e.module_fraction,
            report: None,
        });
    });
    let n = report.modules.len() as u32;
    let done = AnalysisEvent {
        module_id: String::new(),
        stage: "done".into(),
        completed: n,
        total: n,
        module_fraction: 1.0,
        report: Some(ReportDto::from(&report)),
    };
    sink.add(done).map_err(|e| anyhow!("{e:?}"))?;
    Ok(())
}

/// Requests cancellation of a running analysis.
#[flutter_rust_bridge::frb(sync)]
pub fn cancel_analysis(id: u32) {
    if let Some(s) = engine::get(u64::from(id)) {
        s.cancel();
    }
}

/// JSON of the last report (visualization bitmaps omitted).
pub fn report_json(id: u32) -> Result<String> {
    let s = session(id)?;
    s.last_report().map(|r| r.to_json()).ok_or_else(|| anyhow!("no analysis has been run yet"))
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

/// ELA at a user-chosen JPEG quality.
pub fn render_ela(id: u32, quality: u8) -> Result<VizDto> {
    Ok(viz_dto("tools", &interactive::ela(&*session(id)?, quality)?))
}

/// Channel view: luma, red, green, blue, cb, cr, hue, saturation.
pub fn render_channel(id: u32, channel: String) -> Result<VizDto> {
    Ok(viz_dto("tools", &interactive::channel(&*session(id)?, &channel)?))
}

/// Bit plane `bit` (0 = LSB) of a channel.
pub fn render_bitplane(id: u32, channel: String, bit: u8) -> Result<VizDto> {
    Ok(viz_dto("tools", &interactive::bitplane(&*session(id)?, &channel, bit)?))
}
