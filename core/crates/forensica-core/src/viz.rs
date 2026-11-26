//! Helpers to render visualizations as PNG.

use image::codecs::png::{CompressionType, FilterType as PngFilter, PngEncoder};
use image::{imageops::FilterType, GrayImage, ImageEncoder, RgbImage};

use crate::{Error, Legend, Result, Visualization, VizKind};

/// Largest side of a rendered visualization; bigger maps are downscaled.
pub const MAX_VIZ_SIDE: u32 = 4096;

/// Supported colormaps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Colormap {
    /// Perceptually uniform dark-to-bright (inferno-like).
    Inferno,
    /// Grayscale.
    Gray,
    /// Blue-white-red diverging.
    Diverging,
}

impl Colormap {
    /// Name used in legends.
    pub fn name(self) -> &'static str {
        match self {
            Self::Inferno => "inferno",
            Self::Gray => "gray",
            Self::Diverging => "diverging",
        }
    }

    /// Maps `t` in 0..=1 to RGB.
    pub fn map(self, t: f32) -> [u8; 3] {
        let t = if t.is_finite() { t.clamp(0.0, 1.0) } else { 0.0 };
        match self {
            Self::Gray => {
                let v = (t * 255.0).round() as u8;
                [v, v, v]
            }
            Self::Inferno => lerp_stops(&INFERNO, t),
            Self::Diverging => lerp_stops(&DIVERGING, t),
        }
    }
}

const INFERNO: [[f32; 3]; 9] = [
    [0.0, 0.0, 4.0],
    [31.0, 12.0, 72.0],
    [85.0, 15.0, 109.0],
    [136.0, 34.0, 106.0],
    [186.0, 54.0, 85.0],
    [227.0, 89.0, 51.0],
    [249.0, 140.0, 10.0],
    [249.0, 201.0, 50.0],
    [252.0, 255.0, 164.0],
];

const DIVERGING: [[f32; 3]; 5] =
    [[33.0, 102.0, 172.0], [146.0, 197.0, 222.0], [247.0, 247.0, 247.0], [244.0, 165.0, 130.0], [178.0, 24.0, 43.0]];

fn lerp_stops(stops: &[[f32; 3]], t: f32) -> [u8; 3] {
    let n = stops.len() - 1;
    let x = t * n as f32;
    let i = (x.floor() as usize).min(n - 1);
    let f = x - i as f32;
    let a = stops[i];
    let b = stops[i + 1];
    [
        (a[0] + (b[0] - a[0]) * f).round() as u8,
        (a[1] + (b[1] - a[1]) * f).round() as u8,
        (a[2] + (b[2] - a[2]) * f).round() as u8,
    ]
}

/// Encodes an RGB image as PNG (fast compression).
pub fn encode_rgb(img: &RgbImage) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    PngEncoder::new_with_quality(&mut out, CompressionType::Fast, PngFilter::Sub)
        .write_image(img.as_raw(), img.width(), img.height(), image::ExtendedColorType::Rgb8)
        .map_err(|e| Error::Encode(e.to_string()))?;
    Ok(out)
}

/// Encodes a gray image as PNG.
pub fn encode_gray(img: &GrayImage) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    PngEncoder::new_with_quality(&mut out, CompressionType::Fast, PngFilter::Sub)
        .write_image(img.as_raw(), img.width(), img.height(), image::ExtendedColorType::L8)
        .map_err(|e| Error::Encode(e.to_string()))?;
    Ok(out)
}

/// Downscales an RGB image so neither side exceeds [`MAX_VIZ_SIDE`].
pub fn fit(img: RgbImage) -> RgbImage {
    let (w, h) = img.dimensions();
    let m = w.max(h);
    if m <= MAX_VIZ_SIDE {
        return img;
    }
    let s = MAX_VIZ_SIDE as f32 / m as f32;
    let nw = ((w as f32 * s).round() as u32).max(1);
    let nh = ((h as f32 * s).round() as u32).max(1);
    image::imageops::resize(&img, nw, nh, FilterType::Triangle)
}

/// Renders a scalar map with a colormap. Values are normalised to `lo..hi`.
pub fn colorize(values: &[f32], w: u32, h: u32, lo: f32, hi: f32, cmap: Colormap) -> Result<RgbImage> {
    if values.len() != (w as usize) * (h as usize) {
        return Err(Error::InvalidArgument("value buffer size mismatch".into()));
    }
    let span = if (hi - lo).abs() < f32::EPSILON { 1.0 } else { hi - lo };
    let mut img = RgbImage::new(w, h);
    for (p, v) in img.pixels_mut().zip(values) {
        p.0 = cmap.map((v - lo) / span);
    }
    Ok(img)
}

/// Robust upper bound: the given percentile (0..1) of the finite values.
pub fn percentile(values: &[f32], q: f32) -> f32 {
    let mut v: Vec<f32> = values.iter().copied().filter(|x| x.is_finite()).collect();
    if v.is_empty() {
        return 0.0;
    }
    let k = ((v.len() - 1) as f32 * q.clamp(0.0, 1.0)).round() as usize;
    let (_, x, _) = v.select_nth_unstable_by(k, |a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    *x
}

/// Builds a heatmap [`Visualization`] from a scalar map.
#[allow(clippy::too_many_arguments)]
pub fn heatmap(
    id: &str,
    title: &str,
    description: &str,
    values: &[f32],
    w: u32,
    h: u32,
    lo: f32,
    hi: f32,
    cmap: Colormap,
    low_label: &str,
    high_label: &str,
) -> Result<Visualization> {
    let img = fit(colorize(values, w, h, lo, hi, cmap)?);
    Ok(Visualization {
        id: id.into(),
        title: title.into(),
        description: description.into(),
        kind: VizKind::Heatmap,
        width: img.width(),
        height: img.height(),
        png: encode_rgb(&img)?,
        legend: Legend { low: low_label.into(), high: high_label.into(), colormap: cmap.name().into() },
        lines: Vec::new(),
    })
}

/// Builds an image visualization (aligned or standalone) from an RGB image.
pub fn image_viz(id: &str, title: &str, description: &str, img: RgbImage, kind: VizKind) -> Result<Visualization> {
    let img = fit(img);
    Ok(Visualization {
        id: id.into(),
        title: title.into(),
        description: description.into(),
        kind,
        width: img.width(),
        height: img.height(),
        png: encode_rgb(&img)?,
        legend: Legend::default(),
        lines: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colormap_endpoints() {
        assert_eq!(Colormap::Gray.map(0.0), [0, 0, 0]);
        assert_eq!(Colormap::Gray.map(1.0), [255, 255, 255]);
        assert_eq!(Colormap::Inferno.map(1.0), [252, 255, 164]);
        assert_eq!(Colormap::Inferno.map(f32::NAN), [0, 0, 4]);
    }

    #[test]
    fn heatmap_encodes_png() {
        let v: Vec<f32> = (0..64).map(|i| i as f32).collect();
        let viz = heatmap("t", "T", "", &v, 8, 8, 0.0, 63.0, Colormap::Inferno, "lo", "hi").unwrap();
        assert!(viz.png.starts_with(b"\x89PNG"));
        assert_eq!((viz.width, viz.height), (8, 8));
        assert!(colorize(&v, 4, 4, 0.0, 1.0, Colormap::Gray).is_err());
    }

    #[test]
    fn percentile_works() {
        let v: Vec<f32> = (0..=100).map(|i| i as f32).collect();
        assert_eq!(percentile(&v, 0.5), 50.0);
        assert_eq!(percentile(&[], 0.5), 0.0);
    }
}
