//! 8×8 block-artifact grid analysis in the pixel domain.
//!
//! JPEG leaves faint discontinuities every 8 pixels. Their phase reveals the grid
//! position: a grid not at (0,0) means the image was cropped after compression;
//! a grid in a PNG means it was a JPEG before; local windows whose grid disagrees
//! with the global one suggest content pasted from another JPEG.

use image::RgbImage;
use serde::Serialize;

/// Grid estimate.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Grid {
    /// Horizontal offset 0..8 (0 = aligned with the image origin).
    pub dx: u8,
    /// Vertical offset 0..8.
    pub dy: u8,
    /// Peak-to-median strength of the horizontal periodicity.
    pub sx: f32,
    /// Peak-to-median strength of the vertical periodicity.
    pub sy: f32,
}

impl Grid {
    /// Combined strength (mean of both directions: real grids can be weaker along one axis).
    pub fn strength(&self) -> f32 {
        0.5 * (self.sx + self.sy)
    }
}

fn luma(img: &RgbImage) -> (Vec<f32>, usize, usize) {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let v = img
        .pixels()
        .map(|p| 0.299 * f32::from(p.0[0]) + 0.587 * f32::from(p.0[1]) + 0.114 * f32::from(p.0[2]))
        .collect();
    (v, w, h)
}

/// Phase sums of boundary-ness: B[p] = Σ (|d(x)| − (|d(x−1)|+|d(x+1)|)/2) for x ≡ p (mod 8),
/// where d(x) = L(x+1) − L(x) is the step between column x and x+1.
fn phase_profile(l: &[f32], w: usize, x0: usize, x1: usize, y0: usize, y1: usize, horizontal: bool) -> [f32; 8] {
    let mut acc = [0f32; 8];
    let at = |x: usize, y: usize| l[y * w + x];
    if horizontal {
        for y in y0..y1 {
            for x in (x0 + 1)..x1.saturating_sub(2) {
                let d = (at(x + 1, y) - at(x, y)).abs();
                let dl = (at(x, y) - at(x - 1, y)).abs();
                let dr = (at(x + 2, y) - at(x + 1, y)).abs();
                acc[x % 8] += (d - 0.5 * (dl + dr)).clamp(-20.0, 20.0);
            }
        }
    } else {
        for y in (y0 + 1)..y1.saturating_sub(2) {
            for x in x0..x1 {
                let d = (at(x, y + 1) - at(x, y)).abs();
                let du = (at(x, y) - at(x, y - 1)).abs();
                let dd = (at(x, y + 2) - at(x, y + 1)).abs();
                acc[y % 8] += (d - 0.5 * (du + dd)).clamp(-20.0, 20.0);
            }
        }
    }
    acc
}

fn peak(p: &[f32; 8]) -> (u8, f32) {
    let (i, m) = p.iter().enumerate().fold((0, f32::MIN), |a, (i, v)| if *v > a.1 { (i, *v) } else { a });
    let mut s = *p;
    s.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let med = 0.5 * (s[3] + s[4]);
    let second = s[6];
    let spread = s.iter().map(|v| (v - med).abs()).sum::<f32>() / 8.0;
    // A JPEG grid has ONE dominant phase. Period-2/4 patterns (Bayer demosaicing, textures)
    // light up several phases equally, so measure the margin over the second-best phase.
    // Boundary between x=i and x=i+1 → grid starts at i+1.
    (((i + 1) % 8) as u8, (m - second) / spread.max(1e-3))
}

/// Estimates the grid over a window.
pub fn estimate(l: &[f32], w: usize, x0: usize, x1: usize, y0: usize, y1: usize) -> Grid {
    let (dx, sx) = peak(&phase_profile(l, w, x0, x1, y0, y1, true));
    let (dy, sy) = peak(&phase_profile(l, w, x0, x1, y0, y1, false));
    Grid { dx, dy, sx, sy }
}

/// Global grid plus a per-window agreement map.
#[derive(Debug, Clone, Serialize)]
pub struct GridAnalysis {
    /// Global grid.
    pub global: Grid,
    /// Window size in pixels.
    pub window: usize,
    /// Map width in windows.
    pub map_w: usize,
    /// Map height in windows.
    pub map_h: usize,
    /// Per window: strength of a *different* grid than the global one (0 when agreeing or weak).
    pub mismatch: Vec<f32>,
}

/// Strength above which a grid is considered present.
pub const PRESENT: f32 = 2.2;

/// Images smaller than this (either side) are not analysed.
pub const MIN_SIDE: usize = 128;

/// Local window strength above which a differing grid is counted.
pub const LOCAL: f32 = 3.5;

/// Runs the analysis.
pub fn analyse(img: &RgbImage) -> GridAnalysis {
    let (l, w, h) = luma(img);
    // Tiny images do not carry enough boundaries for a reliable estimate.
    let global = if w < MIN_SIDE || h < MIN_SIDE { Grid { dx: 0, dy: 0, sx: 0.0, sy: 0.0 } } else { estimate(&l, w, 0, w, 0, h) };
    let window = 64;
    let (mw, mh) = (w / window, h / window);
    let mut mismatch = vec![0f32; mw * mh];
    for my in 0..mh {
        for mx in 0..mw {
            let g = estimate(&l, w, mx * window, (mx + 1) * window, my * window, (my + 1) * window);
            if g.strength() >= LOCAL && (g.dx != global.dx || g.dy != global.dy) {
                mismatch[my * mw + mx] = g.strength();
            }
        }
    }
    GridAnalysis { global, window, map_w: mw, map_h: mh, mismatch }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::double::tests::natural;
    use crate::testutil::encode;

    fn decoded(img: &RgbImage, q: u8) -> RgbImage {
        image::load_from_memory(&encode(img, q)).unwrap().to_rgb8()
    }

    #[test]
    fn finds_aligned_grid_after_jpeg() {
        let g = analyse(&decoded(&natural(256, 256, 1), 70)).global;
        assert_eq!((g.dx, g.dy), (0, 0));
        assert!(g.strength() > PRESENT, "{g:?}");
    }

    #[test]
    fn finds_shifted_grid_after_crop() {
        let d = decoded(&natural(300, 300, 2), 70);
        let crop = image::imageops::crop_imm(&d, 3, 5, 256, 256).to_image();
        let g = analyse(&crop).global;
        assert_eq!((g.dx, g.dy), (5, 3), "{g:?}");
    }

    #[test]
    fn no_grid_in_uncompressed() {
        let g = analyse(&natural(256, 256, 3)).global;
        assert!(g.strength() < PRESENT, "{g:?}");
    }
}
