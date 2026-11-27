//! Experimental illuminant-colour map (grey-edge estimate per block).
//!
//! Each block's illuminant chromaticity is estimated from the average gradient
//! magnitude of each colour channel (grey-edge hypothesis, van de Weijer et al.).
//! Objects pasted from photos taken under different light often stand out.
//! This is a visual aid only: no automatic finding is produced.

use serde_json::json;

use forensica_core::viz::{heatmap, image_viz, Colormap};
use forensica_core::{AnalysisModule, Context, ImageInput, ModuleResult, Result, VizKind};

const BLOCK: usize = 24;

/// Illuminant module (experimental).
pub struct IlluminantModule;

impl AnalysisModule for IlluminantModule {
    fn id(&self) -> &'static str {
        "illuminant"
    }
    fn name(&self) -> &'static str {
        "Lighting consistency (experimental)"
    }
    fn description(&self) -> &'static str {
        "Local illuminant colour map (grey-edge); visual aid, no automatic verdict"
    }
    fn applies_to(&self, input: &ImageInput) -> bool {
        input.format().decodable()
    }
    fn run(&self, input: &ImageInput, ctx: &mut Context<'_>) -> Result<ModuleResult> {
        let cap = if ctx.options.max_pixels == 0 { 4_000_000 } else { ctx.options.max_pixels.min(4_000_000) };
        let (img, _) = input.working_rgb(cap)?;
        let (w, h) = (img.width() as usize, img.height() as usize);
        let (mw, mh) = (w / BLOCK, h / BLOCK);
        let px = |x: usize, y: usize| img.get_pixel(x as u32, y as u32).0.map(f32::from);
        let mut chroma = vec![[0f32; 3]; mw * mh];
        let mut valid = vec![false; mw * mh];
        for by in 0..mh {
            for bx in 0..mw {
                let mut e = [0f32; 3];
                let mut n = 0;
                for y in by * BLOCK..(by + 1) * BLOCK - 1 {
                    for x in bx * BLOCK..(bx + 1) * BLOCK - 1 {
                        let (p, r, d) = (px(x, y), px(x + 1, y), px(x, y + 1));
                        if p.iter().any(|v| *v >= 250.0 || *v <= 5.0) {
                            continue;
                        }
                        for c in 0..3 {
                            e[c] += ((r[c] - p[c]).powi(2) + (d[c] - p[c]).powi(2)).sqrt();
                        }
                        n += 1;
                    }
                }
                let s = e[0] + e[1] + e[2];
                if n > BLOCK * BLOCK / 2 && s > (n as f32) * 3.0 {
                    chroma[by * mw + bx] = [e[0] / s, e[1] / s, e[2] / s];
                    valid[by * mw + bx] = true;
                }
            }
            ctx.check_cancel()?;
        }
        let vals: Vec<[f32; 3]> = chroma.iter().zip(&valid).filter(|(_, v)| **v).map(|(c, _)| *c).collect();
        let med = |k: usize| crate::gray::median(&vals.iter().map(|c| c[k]).collect::<Vec<_>>());
        let m = [med(0), med(1), med(2)];
        let dev: Vec<f32> =
            chroma
                .iter()
                .zip(&valid)
                .map(|(c, v)| {
                    if *v {
                        ((c[0] - m[0]).powi(2) + (c[1] - m[1]).powi(2) + (c[2] - m[2]).powi(2)).sqrt()
                    } else {
                        0.0
                    }
                })
                .collect();
        let color = image::RgbImage::from_fn(mw.max(1) as u32, mh.max(1) as u32, |x, y| {
            let i = y as usize * mw + x as usize;
            if i >= chroma.len() || !valid[i] {
                return image::Rgb([40, 40, 40]);
            }
            let c = chroma[i];
            let mx = c[0].max(c[1]).max(c[2]).max(1e-3);
            image::Rgb([(c[0] / mx * 255.0) as u8, (c[1] / mx * 255.0) as u8, (c[2] / mx * 255.0) as u8])
        });
        let viz = vec![
            image_viz(
                "illuminant.color",
                "Illuminant colour (experimental)",
                "Estimated light colour per block (grey-edge). Regions lit by a different light source than the rest may appear in a different hue. Grey blocks had too little texture. Strongly coloured surfaces bias the estimate.",
                color,
                VizKind::Aligned,
            )?,
            heatmap(
                "illuminant.deviation",
                "Illuminant deviation (experimental)",
                "Distance of each block's illuminant estimate from the image median. Highlights candidate inconsistent lighting; coloured objects also light up.",
                &dev,
                mw as u32,
                mh as u32,
                0.0,
                0.12,
                Colormap::Inferno,
                "typical",
                "different",
            )?,
        ];
        Ok(ModuleResult {
            findings: Vec::new(),
            visualizations: viz,
            raw: json!({ "median_chroma": m, "blocks": vals.len() }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_maps() {
        let img = crate::resampling::tests::natural(240, 240, 4);
        let mut buf = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img).write_to(&mut buf, image::ImageFormat::Png).unwrap();
        let r =
            IlluminantModule.run(&ImageInput::from_bytes("i.png", buf.into_inner()), &mut Context::simple()).unwrap();
        assert_eq!(r.visualizations.len(), 2);
        assert!(r.findings.is_empty());
    }
}
