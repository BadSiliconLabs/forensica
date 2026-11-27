//! Module: double JPEG compression (aligned, DCT domain) and block-grid consistency (pixel domain).

use serde_json::json;

use crate::dct::decode_info;
use crate::double::{analyse as analyse_dq, largest_region};
use crate::grid::{analyse as analyse_grid, PRESENT};
use crate::structure::parse;
use forensica_core::viz::{heatmap, Colormap};
use forensica_core::{AnalysisModule, Context, FileFormat, Finding, ImageInput, ModuleResult, Result, Severity};

const MODULE: &str = "double_jpeg";

/// Double-compression and grid module.
pub struct DoubleCompressionModule;

impl AnalysisModule for DoubleCompressionModule {
    fn id(&self) -> &'static str {
        MODULE
    }
    fn name(&self) -> &'static str {
        "Double compression & JPEG grid"
    }
    fn version(&self) -> u32 {
        1
    }
    fn description(&self) -> &'static str {
        "DCT-histogram double quantisation, primary quality, localisation map, 8×8 grid alignment"
    }
    fn applies_to(&self, input: &ImageInput) -> bool {
        input.format().decodable()
    }
    fn run(&self, input: &ImageInput, ctx: &mut Context<'_>) -> Result<ModuleResult> {
        let mut findings = Vec::new();
        let mut viz = Vec::new();
        let mut raw = json!({});
        if input.format() == FileFormat::Jpeg {
            let info = parse(input.bytes())?;
            match decode_info(input.bytes(), &info) {
                Ok(coef) => {
                    ctx.report(0.3);
                    if let Some(y) = coef.components.first() {
                        let dq = analyse_dq(y);
                        raw["double"] = json!({ "evidence": dq.evidence, "primary_quality": dq.primary_quality, "modes": dq.modes });
                        if !dq.map.is_empty() {
                            let mut f = Finding::new(MODULE, "djpeg.aligned", Severity::Medium, 0.8, "JPEG compressed at least twice")
                                .explain(format!(
                                    "DCT coefficient histograms show the gaps/peaks left by an earlier JPEG compression ({} of {} frequencies). The image was saved as JPEG, decoded, and saved again — edits may have happened in between.",
                                    dq.evidence.0, dq.evidence.1
                                ))
                                .limits("Re-saving is extremely common (messaging apps, social networks, simple edits like exposure). This alone says nothing about content changes.")
                                .with("frequencies_with_evidence", format!("{}/{}", dq.evidence.0, dq.evidence.1));
                            if let Some(q) = dq.primary_quality {
                                f = f.with("estimated_primary_quality", q);
                            }
                            findings.push(f);
                            let blocks = dq.map_w * dq.map_h;
                            let region = largest_region(&dq.map, dq.map_w, dq.map_h, 0.85);
                            let frac = region as f32 / blocks.max(1) as f32;
                            if region >= 24 && (0.004..=0.6).contains(&frac) {
                                findings.push(
                                    Finding::new(MODULE, "djpeg.local_inconsistency", Severity::High, (0.6 + frac).min(0.85), "Region without the double-compression traces found elsewhere")
                                        .explain(format!(
                                            "A connected area of {region} blocks ({:.1}% of the image) does not follow the earlier quantisation that the rest of the image shows. This is the classic trace of content pasted in before the last save.",
                                            frac * 100.0
                                        ))
                                        .limits("Very flat or saturated areas carry little evidence and can light up; so can areas edited with filters before the last save. Check the map.")
                                        .with("blocks", region),
                                );
                            }
                            viz.push(heatmap(
                                "djpeg.map",
                                "Double-compression inconsistency",
                                "Per 8×8 block probability that the block does NOT carry the earlier JPEG compression seen in the rest of the image. Bright areas were likely inserted or regenerated before the final save.",
                                &dq.map,
                                dq.map_w as u32,
                                dq.map_h as u32,
                                0.0,
                                1.0,
                                Colormap::Inferno,
                                "consistent",
                                "inconsistent",
                            )?);
                        }
                    }
                }
                Err(e) => raw["double_error"] = json!(e.to_string()),
            }
        }
        ctx.check_cancel()?;
        ctx.report(0.6);
        // Grid analysis needs full resolution: scaling destroys the 8-pixel period.
        let img = input.rgb()?;
        let g = analyse_grid(&img);
        raw["grid"] = json!(g);
        let present = g.global.strength() >= PRESENT;
        if input.format() != FileFormat::Jpeg && present {
            let shifted = g.global.dx != 0 || g.global.dy != 0;
            findings.push(
                Finding::new(
                    MODULE,
                    "grid.previous_jpeg",
                    Severity::Low,
                    (0.4 + 0.05 * g.global.strength()).min(0.85),
                    "Traces of an earlier JPEG compression",
                )
                .explain(format!(
                    "An 8×8 block grid is visible although the file is {:?}: it was a JPEG before being converted{}.",
                    input.format(),
                    if shifted {
                        format!(", and was cropped or shifted afterwards (grid offset {},{})", g.global.dx, g.global.dy)
                    } else {
                        String::new()
                    }
                ))
                .limits("Screenshots of JPEGs and some upscalers also show a grid.")
                .with("offset", format!("{},{}", g.global.dx, g.global.dy))
                .with("strength", format!("{:.1}", g.global.strength())),
            );
        }
        if input.format() == FileFormat::Jpeg && present && (g.global.dx != 0 || g.global.dy != 0) {
            findings.push(
                Finding::new(MODULE, "grid.misaligned", Severity::Medium, 0.7, "Dominant JPEG grid is not aligned with the image")
                    .explain(format!("The strongest 8×8 grid is offset by ({},{}): an earlier, stronger JPEG compression was followed by cropping and a light re-save.", g.global.dx, g.global.dy))
                    .limits("Requires the last save to be high quality; heavy textures can confuse the estimate.")
                    .with("offset", format!("{},{}", g.global.dx, g.global.dy)),
            );
        }
        let mism = g.mismatch.iter().filter(|v| **v > 0.0).count();
        let cluster = largest_region(
            &g.mismatch.iter().map(|v| if *v > 0.0 { 1.0 } else { 0.0 }).collect::<Vec<_>>(),
            g.map_w,
            g.map_h,
            0.5,
        );
        if present && cluster >= 4 && (mism as f32) < 0.2 * (g.map_w * g.map_h) as f32 {
            findings.push(
                Finding::new(MODULE, "grid.local_mismatch", Severity::Medium, 0.6, "Local block grid differs from the rest")
                    .explain(format!("{cluster} adjacent 64×64 windows carry an 8×8 grid at a different position than the image's: that content may come from another JPEG."))
                    .limits("Textures with 8-pixel periodicity (tiles, fabrics, text) can mimic a grid.")
                    .with("windows", cluster),
            );
        }
        if present && g.map_w > 0 && g.map_h > 0 {
            viz.push(heatmap(
                "grid.mismatch",
                "Block-grid mismatch",
                "64×64 windows whose 8×8 JPEG grid is at a different position than the global one (brightness = grid strength). Isolated windows are common; compact clusters are suspicious.",
                &g.mismatch,
                g.map_w as u32,
                g.map_h as u32,
                0.0,
                PRESENT * 3.0,
                Colormap::Inferno,
                "agrees",
                "different grid",
            )?);
        }
        Ok(ModuleResult { findings, visualizations: viz, raw })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::double::tests::natural;
    use crate::testutil::encode;

    fn run(bytes: Vec<u8>, name: &str) -> ModuleResult {
        DoubleCompressionModule.run(&ImageInput::from_bytes(name, bytes), &mut Context::simple()).unwrap()
    }

    #[test]
    fn spliced_jpeg_gets_high_finding_and_map() {
        let mut first = image::load_from_memory(&encode(&natural(512, 512, 3), 60)).unwrap().to_rgb8();
        let other = natural(512, 512, 77);
        for y in 160..320 {
            for x in 160..320 {
                first.put_pixel(x, y, *other.get_pixel(x, y));
            }
        }
        let r = run(encode(&first, 92), "s.jpg");
        let codes: Vec<&str> = r.findings.iter().map(|f| f.code.as_str()).collect();
        assert!(codes.contains(&"djpeg.aligned"), "{codes:?}");
        assert!(codes.contains(&"djpeg.local_inconsistency"), "{codes:?}");
        assert!(r.visualizations.iter().any(|v| v.id == "djpeg.map"));
    }

    #[test]
    fn png_from_jpeg_is_noticed() {
        let d = image::load_from_memory(&encode(&natural(256, 256, 4), 70)).unwrap().to_rgb8();
        let mut buf = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(d).write_to(&mut buf, image::ImageFormat::Png).unwrap();
        let r = run(buf.into_inner(), "x.png");
        assert!(r.findings.iter().any(|f| f.code == "grid.previous_jpeg"), "{}", r.raw["grid"]["global"]);
    }

    #[test]
    fn single_jpeg_is_quiet() {
        let r = run(encode(&natural(512, 512, 5), 90), "x.jpg");
        assert!(r.findings.iter().all(|f| f.severity < Severity::Medium), "{:?}", r.findings);
    }
}
