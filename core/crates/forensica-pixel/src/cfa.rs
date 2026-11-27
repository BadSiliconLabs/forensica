//! Colour filter array (CFA / demosaicing) consistency.
//!
//! Camera sensors sample one colour per pixel (Bayer pattern) and interpolate the
//! rest. In the green channel, interpolated pixels are predictable from their
//! 4 neighbours while sampled pixels are not, giving a checkerboard of prediction
//! error variance. Per block we measure F = ln(var_even / var_odd) over the two
//! green lattices. Regions where the pattern disappears or flips were likely
//! resampled, pasted, retouched or synthesised.

use rayon::prelude::*;
use serde_json::json;

use forensica_core::container::jpeg_layout;
use forensica_core::viz::{heatmap, Colormap};
use forensica_core::{AnalysisModule, Context, FileFormat, Finding, ImageInput, ModuleResult, Result, Severity};

/// Block size (even).
pub const BLOCK: usize = 32;

/// Per-block CFA feature map.
pub fn feature_map(img: &image::RgbImage) -> (Vec<f32>, Vec<f32>, usize, usize) {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let g: Vec<f32> = img.pixels().map(|p| f32::from(p.0[1])).collect();
    let (mw, mh) = (w / BLOCK, h / BLOCK);
    let res: Vec<(f32, f32)> = (0..mw * mh)
        .into_par_iter()
        .map(|i| {
            let (bx, by) = (i % mw * BLOCK, i / mw * BLOCK);
            let (mut se, mut so, mut ne, mut no) = (0f64, 0f64, 0f64, 0f64);
            let mut tex = 0f64;
            for y in by.max(1)..(by + BLOCK).min(h - 1) {
                for x in bx.max(1)..(bx + BLOCK).min(w - 1) {
                    let c = g[y * w + x];
                    if !(4.0..=251.0).contains(&c) {
                        continue;
                    }
                    let pred = 0.25 * (g[y * w + x - 1] + g[y * w + x + 1] + g[(y - 1) * w + x] + g[(y + 1) * w + x]);
                    let e = f64::from(c - pred);
                    tex += e.abs();
                    if (x + y) % 2 == 0 {
                        se += e * e;
                        ne += 1.0;
                    } else {
                        so += e * e;
                        no += 1.0;
                    }
                }
            }
            if ne < 64.0 || no < 64.0 {
                return (0.0, 0.0);
            }
            let f = ((se / ne + 1e-3) / (so / no + 1e-3)).ln() as f32;
            (f, (tex / (ne + no)) as f32)
        })
        .collect();
    (res.iter().map(|r| r.0).collect(), res.iter().map(|r| r.1).collect(), mw, mh)
}

fn jpeg_luma_step(bytes: &[u8]) -> Option<f32> {
    let l = jpeg_layout(bytes).ok()?;
    let d = bytes.get(l.with_marker(0xDB).next()?.data.clone())?;
    let t = d.get(1..65)?;
    Some(t.iter().map(|v| f32::from(*v)).sum::<f32>() / 64.0)
}

/// CFA module.
pub struct CfaModule;

impl AnalysisModule for CfaModule {
    fn id(&self) -> &'static str {
        "cfa"
    }
    fn name(&self) -> &'static str {
        "CFA (demosaicing) consistency"
    }
    fn description(&self) -> &'static str {
        "Bayer-pattern interpolation traces and regions where they are missing"
    }
    fn applies_to(&self, input: &ImageInput) -> bool {
        input.format().decodable()
    }
    fn run(&self, input: &ImageInput, ctx: &mut Context<'_>) -> Result<ModuleResult> {
        if input.format() == FileFormat::Jpeg {
            if let Some(step) = jpeg_luma_step(input.bytes()) {
                if step > 4.0 {
                    return Ok(ModuleResult {
                        findings: vec![Finding::new("cfa", "cfa.not_applicable", Severity::Info, 1.0, "CFA analysis skipped: compression too strong")
                            .explain("JPEG compression at this level erases demosaicing traces, so their absence would mean nothing.")
                            .limits("CFA analysis needs high-quality JPEG (≈ q ≥ 90) or lossless images.")],
                        visualizations: Vec::new(),
                        raw: json!({ "mean_luma_step": step }),
                    });
                }
            }
        }
        // Full resolution only: any resizing destroys the 2-pixel lattice.
        let img = input.rgb()?;
        let (f, tex, mw, mh) = feature_map(&img);
        ctx.check_cancel()?;
        let textured: Vec<f32> = f.iter().zip(&tex).filter(|(_, t)| **t > 1.0).map(|(v, _)| *v).collect();
        let global = crate::gray::median(&textured);
        let strong = global.abs() > 0.35 && textured.len() >= 20;
        let mut findings = Vec::new();
        let missing: Vec<f32> = if strong {
            f.iter()
                .zip(&tex)
                .map(|(v, t)| if *t > 1.0 { (1.0 - v * global.signum() / global.abs()).clamp(0.0, 1.0) } else { 0.0 })
                .collect()
        } else {
            vec![0.0; f.len()]
        };
        if strong {
            findings.push(
                Finding::new("cfa", "cfa.present", Severity::Info, 0.8, "Camera demosaicing traces present")
                    .explain("The green channel shows the checkerboard interpolation pattern of a Bayer sensor across the image, consistent with an unresized camera image.")
                    .limits("Traces survive only without resizing and with light compression.")
                    .with("strength", format!("{global:.2}")),
            );
            let hot: Vec<f32> = missing.iter().map(|v| if *v >= 0.9 { 1.0 } else { 0.0 }).collect();
            let region = largest(&hot, mw, mh);
            let total = textured.len();
            if region >= 4 && (region as f32) < 0.3 * total as f32 {
                findings.push(
                    Finding::new("cfa", "cfa.local_missing", Severity::Medium, 0.6, "Region without demosaicing traces")
                        .explain(format!("{region} adjacent blocks lack the Bayer interpolation pattern present in the rest of the image: their pixels may not come from this camera capture (pasted, resampled, painted or generated)."))
                        .limits("Very dark/bright, out-of-focus or heavily denoised areas weaken the pattern; experimental with modern phone pipelines.")
                        .with("blocks", region),
                );
            }
        } else if input.format() != FileFormat::Jpeg || jpeg_luma_step(input.bytes()).is_some_and(|s| s <= 2.0) {
            findings.push(
                Finding::new("cfa", "cfa.absent", Severity::Info, 0.6, "No camera demosaicing traces")
                    .explain("The Bayer interpolation pattern is not visible. The image was probably resized, heavily processed, or not produced by a camera sensor (render, screenshot, AI generator).")
                    .limits("Many phones apply processing that removes the pattern; this is common and not by itself suspicious."),
            );
        }
        let viz = vec![heatmap(
            "cfa.map",
            "CFA consistency",
            "Per 32×32 block: how much the Bayer demosaicing pattern is missing compared with the image as a whole (bright = missing). Only meaningful when the image overall shows the pattern.",
            &missing,
            mw as u32,
            mh as u32,
            0.0,
            1.0,
            Colormap::Inferno,
            "consistent",
            "missing",
        )?];
        Ok(ModuleResult { findings, visualizations: viz, raw: json!({ "global": global, "blocks": textured.len() }) })
    }
}

fn largest(map: &[f32], w: usize, h: usize) -> usize {
    let mut seen = vec![false; map.len()];
    let mut best = 0;
    for s in 0..map.len() {
        if seen[s] || map[s] < 0.5 {
            continue;
        }
        let (mut stack, mut n) = (vec![s], 0);
        seen[s] = true;
        while let Some(i) = stack.pop() {
            n += 1;
            let (x, y) = (i % w, i / w);
            for (nx, ny) in [(x.wrapping_sub(1), y), (x + 1, y), (x, y.wrapping_sub(1)), (x, y + 1)] {
                if nx < w && ny < h {
                    let j = ny * w + nx;
                    if !seen[j] && map[j] >= 0.5 {
                        seen[j] = true;
                        stack.push(j);
                    }
                }
            }
        }
        best = best.max(n);
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resampling::tests::natural;

    /// Simulates a Bayer (RGGB) capture + bilinear demosaicing.
    pub fn demosaiced(src: &image::RgbImage) -> image::RgbImage {
        let (w, h) = src.dimensions();
        let raw = |x: u32, y: u32| -> f32 {
            let p = src.get_pixel(x.min(w - 1), y.min(h - 1)).0;
            match (x % 2, y % 2) {
                (0, 0) => f32::from(p[0]),
                (1, 1) => f32::from(p[2]),
                _ => f32::from(p[1]),
            }
        };
        image::RgbImage::from_fn(w, h, |x, y| {
            let g = if (x + y) % 2 == 1 {
                raw(x, y)
            } else {
                0.25 * (raw(x.saturating_sub(1), y) + raw(x + 1, y) + raw(x, y.saturating_sub(1)) + raw(x, y + 1))
            };
            let o = src.get_pixel(x, y).0;
            image::Rgb([o[0], g.round() as u8, o[2]])
        })
    }

    fn png(img: &image::RgbImage) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img.clone()).write_to(&mut buf, image::ImageFormat::Png).unwrap();
        buf.into_inner()
    }

    #[test]
    fn detects_pattern_and_pasted_region() {
        let mut cam = demosaiced(&natural(384, 384, 9));
        let other = natural(384, 384, 10);
        for y in 128..256 {
            for x in 128..256 {
                cam.put_pixel(x, y, *other.get_pixel(x, y));
            }
        }
        let r = CfaModule.run(&ImageInput::from_bytes("c.png", png(&cam)), &mut Context::simple()).unwrap();
        let codes: Vec<&str> = r.findings.iter().map(|f| f.code.as_str()).collect();
        assert!(codes.contains(&"cfa.present"), "{codes:?} {}", r.raw);
        assert!(codes.contains(&"cfa.local_missing"), "{codes:?} {}", r.raw);
    }

    #[test]
    fn no_pattern_in_synthetic() {
        let r = CfaModule
            .run(&ImageInput::from_bytes("s.png", png(&natural(256, 256, 11))), &mut Context::simple())
            .unwrap();
        assert!(r.findings.iter().any(|f| f.code == "cfa.absent"), "{}", r.raw);
    }
}
