//! Interactive helpers used by the viewer: previews and the pixel inspector.

use image::imageops::FilterType;
use serde::Serialize;

use crate::Session;
use forensica_core::{Error, Result};

/// RGBA preview for display.
#[derive(Debug, Clone)]
pub struct Preview {
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
    /// Scale relative to the original (preview / original).
    pub scale: f32,
    /// RGBA8 pixels.
    pub rgba: Vec<u8>,
}

/// Renders an RGBA preview whose longest side is at most `max_side` (0 = original size).
pub fn preview(s: &Session, max_side: u32) -> Result<Preview> {
    let img = s.input.rgb()?;
    let (w, h) = img.dimensions();
    let m = w.max(h);
    let (img, scale) = if max_side > 0 && m > max_side {
        let k = max_side as f32 / m as f32;
        let nw = ((w as f32 * k).round() as u32).max(1);
        let nh = ((h as f32 * k).round() as u32).max(1);
        (image::imageops::resize(img.as_ref(), nw, nh, FilterType::Triangle), nw as f32 / w as f32)
    } else {
        ((*img).clone(), 1.0)
    };
    let rgba = image::DynamicImage::ImageRgb8(img).to_rgba8();
    Ok(Preview { width: rgba.width(), height: rgba.height(), scale, rgba: rgba.into_raw() })
}

/// Values at one pixel of the original.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct PixelInfo {
    /// X.
    pub x: u32,
    /// Y.
    pub y: u32,
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
    /// Rec.601 luma.
    pub luma: f32,
    /// Mean of the 8×8 JPEG block containing the pixel (luma).
    pub block_mean: f32,
}

/// Reads one pixel of the full-resolution original.
pub fn pixel_at(s: &Session, x: u32, y: u32) -> Result<PixelInfo> {
    let img = s.input.rgb()?;
    if x >= img.width() || y >= img.height() {
        return Err(Error::InvalidArgument(format!("({x},{y}) outside image")));
    }
    let luma = |p: &image::Rgb<u8>| 0.299 * f32::from(p.0[0]) + 0.587 * f32::from(p.0[1]) + 0.114 * f32::from(p.0[2]);
    let p = img.get_pixel(x, y);
    let (bx, by) = (x - x % 8, y - y % 8);
    let mut sum = 0.0;
    let mut n = 0.0;
    for yy in by..(by + 8).min(img.height()) {
        for xx in bx..(bx + 8).min(img.width()) {
            sum += luma(img.get_pixel(xx, yy));
            n += 1.0;
        }
    }
    Ok(PixelInfo { x, y, r: p.0[0], g: p.0[1], b: p.0[2], luma: luma(p), block_mean: sum / n })
}

/// Interactive pixels are capped to keep the UI responsive on phones.
const INTERACTIVE_MAX_PIXELS: u64 = 24_000_000;

/// ELA at a custom quality.
pub fn ela(s: &Session, quality: u8) -> Result<forensica_core::Visualization> {
    let (img, _) = s.input.working_rgb(INTERACTIVE_MAX_PIXELS)?;
    forensica_pixel::ela::render(&img, quality)
}

/// Single channel view.
pub fn channel(s: &Session, name: &str) -> Result<forensica_core::Visualization> {
    let (img, _) = s.input.working_rgb(INTERACTIVE_MAX_PIXELS)?;
    forensica_pixel::channels::render_channel(&img, name)
}

/// Bit-plane view.
pub fn bitplane(s: &Session, name: &str, bit: u8) -> Result<forensica_core::Visualization> {
    let (img, _) = s.input.working_rgb(INTERACTIVE_MAX_PIXELS)?;
    forensica_pixel::channels::render_bitplane(&img, name, bit)
}

/// PCE above which a PRNU match is usually declared.
pub const PRNU_THRESHOLD: f32 = forensica_pixel::prnu::PCE_THRESHOLD;

/// PRNU comparison of the session image against reference photos from one camera.
pub fn prnu(s: &Session, references: &[String]) -> Result<forensica_pixel::prnu::PrnuResult> {
    use rayon::prelude::*;
    let img = s.input.rgb()?;
    let refs: Vec<image::RgbImage> =
        references.par_iter().filter_map(|p| image::open(p).ok().map(|i| i.to_rgb8())).collect();
    let failed = references.len() - refs.len();
    let mut r = forensica_pixel::prnu::compare(&img, &refs)?;
    r.skipped += failed;
    Ok(r)
}

/// Image files in `dir` (recursing up to 3 levels, at most 5000 files).
pub fn list_images(dir: &std::path::Path) -> Vec<String> {
    const EXT: &[&str] = &["jpg", "jpeg", "png", "webp", "tif", "tiff", "bmp", "gif"];
    let mut out = Vec::new();
    let mut stack = vec![(dir.to_path_buf(), 0)];
    while let Some((d, depth)) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                if depth < 3 {
                    stack.push((p, depth + 1));
                }
            } else if p
                .extension()
                .map(|x| x.to_string_lossy().to_ascii_lowercase())
                .is_some_and(|x| EXT.contains(&x.as_str()))
            {
                out.push(p.to_string_lossy().into_owned());
                if out.len() >= 5000 {
                    return out;
                }
            }
        }
    }
    out.sort();
    out
}

/// Near-duplicates of the session image among `paths`.
pub fn similar(s: &Session, paths: &[String], max_distance: u32) -> Result<Vec<forensica_pixel::phash::Similar>> {
    let (img, _) = s.input.working_rgb(4_000_000)?;
    let own = std::fs::canonicalize(&s.path).ok();
    let paths: Vec<String> =
        paths.iter().filter(|p| own.is_none() || std::fs::canonicalize(p).ok() != own).cloned().collect();
    Ok(forensica_pixel::phash::find_similar(&img, &paths, max_distance, &|| false))
}

/// Compares the session image with another version of the same picture.
pub fn compare(s: &Session, other: &str) -> Result<(forensica_core::Visualization, Vec<forensica_core::Finding>)> {
    let a = s.input.rgb()?;
    let b = image::open(other).map_err(|e| forensica_core::Error::Decode(e.to_string()))?.to_rgb8();
    let r = forensica_pixel::pair::compare(&a, &b);
    let hi = forensica_core::viz::percentile(&r.map, 0.99).max(1.0);
    let viz = forensica_core::viz::heatmap(
        "compare.map",
        "Difference with the other version",
        "Blocks whose content differs between this image and the other version, after resizing and alignment. A compact bright region is what was added, removed or changed.",
        &r.map,
        r.map_w as u32,
        r.map_h as u32,
        0.0,
        hi,
        forensica_core::viz::Colormap::Inferno,
        "same",
        "different",
    )?;
    Ok((viz, forensica_pixel::pair::indicators(&r)))
}
