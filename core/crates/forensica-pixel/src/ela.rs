//! Error Level Analysis: difference between the image and a re-compressed copy.

use image::RgbImage;
use serde_json::json;

use forensica_core::viz::{heatmap, Colormap};
use forensica_core::{AnalysisModule, Context, Error, ImageInput, ModuleResult, Result, Visualization};

/// Computes the per-pixel ELA map (max channel abs difference) at `quality`.
pub fn ela_map(img: &RgbImage, quality: u8) -> Result<Vec<f32>> {
    let mut buf = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality.clamp(1, 100))
        .encode_image(img)
        .map_err(|e| Error::Encode(e.to_string()))?;
    let re = image::load_from_memory_with_format(&buf, image::ImageFormat::Jpeg)
        .map_err(|e| Error::Decode(e.to_string()))?
        .to_rgb8();
    Ok(img
        .pixels()
        .zip(re.pixels())
        .map(|(a, b)| (0..3).map(|c| (i16::from(a.0[c]) - i16::from(b.0[c])).unsigned_abs()).max().unwrap_or(0) as f32)
        .collect())
}

/// Renders an ELA visualization.
pub fn render(img: &RgbImage, quality: u8) -> Result<Visualization> {
    let map = ela_map(img, quality)?;
    let hi = forensica_core::viz::percentile(&map, 0.995).max(4.0);
    heatmap(
        &format!("ela.q{quality}"),
        &format!("Error level analysis (q={quality})"),
        "Each pixel shows how much it changes when re-saved as JPEG. Regions that were pasted or edited after \
         the last compression often re-compress differently from their surroundings. Edges and textures are \
         naturally brighter; compare similar surfaces only.",
        &map,
        img.width(),
        img.height(),
        0.0,
        hi,
        Colormap::Inferno,
        "stable",
        "changes a lot",
    )
}

/// ELA module (default quality from options).
pub struct ElaModule;

impl AnalysisModule for ElaModule {
    fn id(&self) -> &'static str {
        "ela"
    }
    fn name(&self) -> &'static str {
        "Error level analysis"
    }
    fn description(&self) -> &'static str {
        "Re-compression difference map (adjustable quality in Tools)"
    }
    fn applies_to(&self, input: &ImageInput) -> bool {
        input.format().decodable()
    }
    fn run(&self, input: &ImageInput, ctx: &mut Context<'_>) -> Result<ModuleResult> {
        let (img, scale) = input.working_rgb(ctx.options.max_pixels)?;
        ctx.check_cancel()?;
        let q = ctx.options.ela_quality;
        let v = render(&img, q)?;
        Ok(ModuleResult { findings: Vec::new(), visualizations: vec![v], raw: json!({ "quality": q, "scale": scale }) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pasted_uncompressed_patch_stands_out() {
        // A JPEG-compressed background with a pristine (never compressed) noisy patch.
        let mut img = RgbImage::from_fn(128, 128, |x, y| image::Rgb([(x * 2) as u8, (y * 2) as u8, 90]));
        let mut buf = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 75).encode_image(&img).unwrap();
        img = image::load_from_memory(&buf).unwrap().to_rgb8();
        let mut seed = 7u32;
        for y in 40..80 {
            for x in 40..80 {
                seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
                let v = (seed >> 16) as u8;
                img.put_pixel(x, y, image::Rgb([v, v / 2, 255 - v]));
            }
        }
        let m = ela_map(&img, 75).unwrap();
        let inside: f32 =
            (40..80).flat_map(|y| (40..80).map(move |x| (x, y))).map(|(x, y)| m[y * 128 + x]).sum::<f32>() / 1600.0;
        let outside: f32 =
            (0..30).flat_map(|y| (0..30).map(move |x| (x, y))).map(|(x, y)| m[y * 128 + x]).sum::<f32>() / 900.0;
        assert!(inside > outside * 3.0, "inside {inside} outside {outside}");
        let v = render(&img, 90).unwrap();
        assert!(v.png.starts_with(b"\x89PNG"));
    }
}
