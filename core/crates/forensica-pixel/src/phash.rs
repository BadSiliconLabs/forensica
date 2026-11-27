//! Perceptual hashes (pHash, dHash) and near-duplicate search.

use image::imageops::FilterType;
use rayon::prelude::*;
use serde::Serialize;
use serde_json::json;

use forensica_core::{AnalysisModule, Context, ImageInput, ModuleResult, Result};

/// 64-bit DCT perceptual hash (32×32 grey → 8×8 lowest frequencies, median threshold).
pub fn phash(img: &image::RgbImage) -> u64 {
    let g =
        image::imageops::resize(&image::DynamicImage::ImageRgb8(img.clone()).to_luma8(), 32, 32, FilterType::Triangle);
    let px: Vec<f32> = g.pixels().map(|p| f32::from(p.0[0])).collect();
    let n = 32;
    let cos: Vec<f32> = (0..8 * n)
        .map(|i| {
            let (u, x) = (i / n, i % n);
            (((2 * x + 1) as f32 * u as f32 * std::f32::consts::PI) / (2.0 * n as f32)).cos()
        })
        .collect();
    let mut rows = vec![0f32; n * 8];
    for y in 0..n {
        for u in 0..8 {
            rows[y * 8 + u] = (0..n).map(|x| cos[u * n + x] * px[y * n + x]).sum();
        }
    }
    let mut coef = [0f32; 64];
    for v in 0..8 {
        for u in 0..8 {
            coef[v * 8 + u] = (0..n).map(|y| cos[v * n + y] * rows[y * 8 + u]).sum();
        }
    }
    let mut sorted: Vec<f32> = coef[1..].to_vec();
    sorted.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let med = sorted[sorted.len() / 2];
    coef.iter().enumerate().fold(0u64, |h, (i, c)| if i > 0 && *c > med { h | (1 << i) } else { h })
}

/// 64-bit difference hash (9×8 grey, horizontal gradient signs).
pub fn dhash(img: &image::RgbImage) -> u64 {
    let g =
        image::imageops::resize(&image::DynamicImage::ImageRgb8(img.clone()).to_luma8(), 9, 8, FilterType::Triangle);
    let mut h = 0u64;
    for y in 0..8 {
        for x in 0..8 {
            if g.get_pixel(x, y).0[0] > g.get_pixel(x + 1, y).0[0] {
                h |= 1 << (y * 8 + x);
            }
        }
    }
    h
}

/// Hamming distance.
pub fn distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

/// A near-duplicate candidate.
#[derive(Debug, Clone, Serialize)]
pub struct Similar {
    /// File path.
    pub path: String,
    /// pHash distance (0 = identical, ≤ 10 = very similar).
    pub phash_distance: u32,
    /// dHash distance.
    pub dhash_distance: u32,
    /// Pixel size.
    pub width: u32,
    /// Pixel height.
    pub height: u32,
}

/// Hashes every file in `paths` (in parallel) and returns those within `max_distance` of `query`.
pub fn find_similar(
    query: &image::RgbImage,
    paths: &[String],
    max_distance: u32,
    cancel: &(dyn Fn() -> bool + Sync),
) -> Vec<Similar> {
    let (qp, qd) = (phash(query), dhash(query));
    let mut out: Vec<Similar> = paths
        .par_iter()
        .filter_map(|p| {
            if cancel() {
                return None;
            }
            let img = image::open(p).ok()?.to_rgb8();
            let (pd, dd) = (distance(qp, phash(&img)), distance(qd, dhash(&img)));
            (pd <= max_distance).then(|| Similar {
                path: p.clone(),
                phash_distance: pd,
                dhash_distance: dd,
                width: img.width(),
                height: img.height(),
            })
        })
        .collect();
    out.sort_by_key(|s| (s.phash_distance, s.dhash_distance));
    out
}

/// Module that records perceptual hashes in the report.
pub struct HashModule;

impl AnalysisModule for HashModule {
    fn id(&self) -> &'static str {
        "hashes"
    }
    fn name(&self) -> &'static str {
        "Perceptual hashes"
    }
    fn description(&self) -> &'static str {
        "pHash and dHash for near-duplicate search"
    }
    fn applies_to(&self, input: &ImageInput) -> bool {
        input.format().decodable()
    }
    fn run(&self, input: &ImageInput, _ctx: &mut Context<'_>) -> Result<ModuleResult> {
        // Hashes are computed on ≤4 MP regardless of options so they are comparable across runs.
        let (img, _) = input.working_rgb(4_000_000)?;
        let (p, d) = (phash(&img), dhash(&img));
        let entries = vec![
            json!({"key": "SHA-256 (file)", "value": input.sha256()}),
            json!({"key": "pHash (DCT)", "value": format!("{p:016x}")}),
            json!({"key": "dHash (gradient)", "value": format!("{d:016x}")}),
        ];
        Ok(ModuleResult {
            findings: Vec::new(),
            visualizations: Vec::new(),
            raw: json!({ "groups": [{ "name": "Hashes", "entries": entries }], "phash": format!("{p:016x}"), "dhash": format!("{d:016x}") }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resampling::tests::natural;

    fn other(w: u32, h: u32) -> image::RgbImage {
        image::RgbImage::from_fn(w, h, |x, y| {
            let v = if (x / 40 + y / 30) % 2 == 0 { 200 } else { 40 };
            image::Rgb([v, v, (x % 255) as u8])
        })
    }

    #[test]
    fn hashes_are_robust_to_resize_and_jpeg_but_not_content() {
        let a = natural(320, 240, 1);
        let small = image::imageops::resize(&a, 200, 150, FilterType::Triangle);
        let mut j = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut j, 60).encode_image(&small).unwrap();
        let b = image::load_from_memory(&j).unwrap().to_rgb8();
        let c = other(320, 240);
        assert!(distance(phash(&a), phash(&b)) <= 6, "{}", distance(phash(&a), phash(&b)));
        assert!(distance(dhash(&a), dhash(&b)) <= 8);
        assert!(distance(phash(&a), phash(&c)) > 15, "{}", distance(phash(&a), phash(&c)));
    }

    #[test]
    fn search_finds_copy() {
        let dir = std::env::temp_dir().join(format!("forensica-phash-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let a = natural(160, 120, 2);
        let pa = dir.join("a.png");
        let pb = dir.join("b.png");
        a.save(&pa).unwrap();
        other(160, 120).save(&pb).unwrap();
        let paths =
            vec![pa.to_string_lossy().into_owned(), pb.to_string_lossy().into_owned(), "/nonexistent.jpg".into()];
        let r = find_similar(&a, &paths, 10, &|| false);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].phash_distance, 0);
    }
}
