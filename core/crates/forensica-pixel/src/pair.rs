//! Pairwise comparison of two versions of the same picture.
//!
//! When the same scene exists in two files and one was edited, comparing them is far
//! more powerful than analysing either alone: we can locate *what* changed and then ask,
//! in each file separately, whether the changed region is statistically consistent with
//! the rest of that file. The version whose changed region is the odd one out, and/or
//! which carries less genuine sensor detail overall, is the likelier derived/edited one.

use image::imageops::FilterType;
use image::RgbImage;
use serde::Serialize;

use crate::gray::{median, Plane};
use crate::noise::residual;

/// Block size (common grid) for the difference map.
pub const BLOCK: usize = 16;

/// Statistics of one image inside / around the changed region.
#[derive(Debug, Clone, Serialize)]
pub struct SideStats {
    /// Native size.
    pub width: u32,
    /// Native height.
    pub height: u32,
    /// Median noise level (mean |3×3-median residual|) in 32-px native blocks over the unchanged area.
    pub noise_unchanged: f32,
    /// Same inside the changed region.
    pub noise_changed: f32,
    /// High-frequency detail (mean |Laplacian|) on the common grid, unchanged area.
    pub detail_unchanged: f32,
    /// Same inside the changed region.
    pub detail_changed: f32,
}

impl SideStats {
    /// |log ratio| of noise inside vs outside the changed region (0 = consistent).
    pub fn noise_inconsistency(&self) -> f32 {
        ((self.noise_changed + 0.2) / (self.noise_unchanged + 0.2)).ln().abs()
    }
}

/// Result of a pairwise comparison.
#[derive(Debug, Clone, Serialize)]
pub struct PairReport {
    /// Common grid size.
    pub grid: (u32, u32),
    /// Best alignment shift of B relative to A on the common grid.
    pub shift: (i32, i32),
    /// Mean absolute luma difference after alignment (unchanged blocks).
    pub residual_diff: f32,
    /// Fraction of blocks that changed.
    pub changed_fraction: f32,
    /// Per-block difference (common grid / BLOCK).
    pub map: Vec<f32>,
    /// Map width.
    pub map_w: usize,
    /// Map height.
    pub map_h: usize,
    /// Stats for A.
    pub a: SideStats,
    /// Stats for B.
    pub b: SideStats,
}

fn normalise(p: &mut Plane) {
    let m = p.data.iter().sum::<f32>() / p.data.len() as f32;
    let sd = (p.data.iter().map(|v| (v - m).powi(2)).sum::<f32>() / p.data.len() as f32).sqrt().max(1.0);
    p.data.iter_mut().for_each(|v| *v = (*v - m) / sd * 50.0 + 128.0);
}

fn laplacian_abs(p: &Plane) -> Plane {
    let mut out = Plane::new(p.w, p.h);
    for y in 0..p.h {
        for x in 0..p.w {
            let (xi, yi) = (x as isize, y as isize);
            let v = 4.0 * p.at(xi, yi) - p.at(xi - 1, yi) - p.at(xi + 1, yi) - p.at(xi, yi - 1) - p.at(xi, yi + 1);
            out.set(x, y, v.abs());
        }
    }
    out
}

fn best_shift(a: &Plane, b: &Plane, r: i32) -> (i32, i32) {
    // Coarse search on a 1/4 grid, refined at full grid.
    let score = |dx: i32, dy: i32, step: usize| -> f32 {
        let mut s = 0.0;
        let mut n = 0.0;
        let m = (r.unsigned_abs() as usize) + 1;
        let mut y = m;
        while y + m < a.h {
            let mut x = m;
            while x + m < a.w {
                s += (a.at(x as isize, y as isize) - b.at(x as isize + dx as isize, y as isize + dy as isize)).abs();
                n += 1.0;
                x += step;
            }
            y += step;
        }
        s / n
    };
    let mut best = (0, 0, f32::MAX);
    for dy in -r..=r {
        for dx in -r..=r {
            let s = score(dx, dy, 4);
            if s < best.2 {
                best = (dx, dy, s);
            }
        }
    }
    (best.0, best.1)
}

fn side_stats(native: &RgbImage, common: &Plane, changed: &[bool], mw: usize, mh: usize) -> SideStats {
    // Noise at native resolution, per 32-px block, mapped onto the common block grid.
    let l = Plane::luma(native);
    let res = residual(&l);
    let (sx, sy) = (l.w as f32 / (mw * BLOCK) as f32, l.h as f32 / (mh * BLOCK) as f32);
    let (mut nu, mut nc) = (Vec::new(), Vec::new());
    for by in 0..mh {
        for bx in 0..mw {
            let x0 = (bx as f32 * BLOCK as f32 * sx) as usize;
            let y0 = (by as f32 * BLOCK as f32 * sy) as usize;
            let x1 = (((bx + 1) as f32 * BLOCK as f32 * sx) as usize).min(l.w);
            let y1 = (((by + 1) as f32 * BLOCK as f32 * sy) as usize).min(l.h);
            let mut v = Vec::with_capacity((x1 - x0) * (y1 - y0));
            for y in y0..y1 {
                v.extend_from_slice(&res.data[y * l.w + x0..y * l.w + x1]);
            }
            // Mean |residual| (clipped): MAD collapses to 0 on very smooth, quantised images.
            let s = v.iter().map(|r| r.abs().min(20.0)).sum::<f32>() / v.len().max(1) as f32;
            if changed[by * mw + bx] {
                nc.push(s);
            } else {
                nu.push(s);
            }
        }
    }
    let lap = laplacian_abs(common);
    let blocks = lap.block_reduce(BLOCK, crate::gray::mean);
    let (mut du, mut dc) = (Vec::new(), Vec::new());
    for i in 0..mw * mh {
        if changed[i] {
            dc.push(blocks.data[i]);
        } else {
            du.push(blocks.data[i]);
        }
    }
    SideStats {
        width: native.width(),
        height: native.height(),
        noise_unchanged: median(&nu),
        noise_changed: median(&nc),
        detail_unchanged: median(&du),
        detail_changed: median(&dc),
    }
}

/// Compares two versions of the same scene.
pub fn compare(a: &RgbImage, b: &RgbImage) -> PairReport {
    // Common grid: the smaller of the two, capped at 1 MP for speed.
    let (aw, ah) = a.dimensions();
    let (bw, bh) = b.dimensions();
    let (mut w, mut h) =
        if u64::from(aw) * u64::from(ah) <= u64::from(bw) * u64::from(bh) { (aw, ah) } else { (bw, bh) };
    let k = (1_000_000f64 / (f64::from(w) * f64::from(h))).sqrt().min(1.0);
    w = ((f64::from(w) * k) as u32).max(BLOCK as u32);
    h = ((f64::from(h) * k) as u32).max(BLOCK as u32);
    let ca = image::imageops::resize(a, w, h, FilterType::Triangle);
    let cb = image::imageops::resize(b, w, h, FilterType::Triangle);
    let (mut la, mut lb) = (Plane::luma(&ca), Plane::luma(&cb));
    normalise(&mut la);
    normalise(&mut lb);
    let (dx, dy) = best_shift(&la, &lb, 12);
    let (mw, mh) = (la.w / BLOCK, la.h / BLOCK);
    let mut map = vec![0f32; mw * mh];
    for by in 0..mh {
        for bx in 0..mw {
            let mut s = 0.0;
            for y in by * BLOCK..(by + 1) * BLOCK {
                for x in bx * BLOCK..(bx + 1) * BLOCK {
                    s += (la.at(x as isize, y as isize) - lb.at(x as isize + dx as isize, y as isize + dy as isize))
                        .abs();
                }
            }
            map[by * mw + bx] = s / (BLOCK * BLOCK) as f32;
        }
    }
    let base = median(&map);
    let t = (base * 3.0).max(base + 12.0);
    let changed: Vec<bool> = map.iter().map(|v| *v > t).collect();
    let n_changed = changed.iter().filter(|c| **c).count();
    let unchanged_diff = median(&map.iter().zip(&changed).filter(|(_, c)| !**c).map(|(v, _)| *v).collect::<Vec<_>>());
    PairReport {
        grid: (w, h),
        shift: (dx, dy),
        residual_diff: unchanged_diff,
        changed_fraction: n_changed as f32 / map.len().max(1) as f32,
        a: side_stats(a, &la, &changed, mw, mh),
        b: side_stats(b, &lb, &changed, mw, mh),
        map,
        map_w: mw,
        map_h: mh,
    }
}

/// Plain-language indicators derived from a comparison (A = the open image, B = the other file).
pub fn indicators(r: &PairReport) -> Vec<forensica_core::Finding> {
    use forensica_core::{Finding, Severity};
    let mut out = Vec::new();
    let pct = r.changed_fraction * 100.0;
    out.push(
        Finding::new("compare", "compare.changed_area", if pct > 0.3 { Severity::Medium } else { Severity::Info }, 0.85, format!("{pct:.1}% of the picture differs between the two versions"))
            .explain("After resizing both to a common size and aligning them, blocks whose content differs clearly more than the rest are highlighted in the comparison map.")
            .limits("Different crops, perspective, or strong colour edits also produce differences; faint outlines everywhere suggest one version was entirely re-rendered.")
            .with("alignment_shift", format!("{},{}", r.shift.0, r.shift.1)),
    );
    let ratio = (r.b.detail_unchanged + 0.05) / (r.a.detail_unchanged + 0.05);
    let (rich, poor, k) = if ratio >= 1.0 {
        ("the other file", "this image", ratio)
    } else {
        ("this image", "the other file", 1.0 / ratio)
    };
    if k >= 1.4 {
        out.push(
            Finding::new("compare", "compare.detail_direction", Severity::Medium, (0.4 + (k - 1.4) * 0.3).min(0.8), format!("{poor} looks derived from {rich}"))
                .explain(format!(
                    "In the areas that are identical in both versions, {rich} carries about {k:.1}× more fine detail at the same scale (plus more sensor noise). A copy that was resized, smoothed or re-rendered by an AI editor cannot gain genuine detail, so {poor} is most likely the processed version and {rich} the closer-to-original one."
                ))
                .limits("Sharpening can add fake detail and heavy denoising in the camera can remove it; if both versions were processed independently from a third source, the direction can be wrong.")
                .with("detail_this", format!("{:.2}", r.a.detail_unchanged))
                .with("detail_other", format!("{:.2}", r.b.detail_unchanged)),
        );
    }
    for (name, s) in [("this image", &r.a), ("the other file", &r.b)] {
        if r.changed_fraction > 0.003 && s.noise_inconsistency() > 0.7 {
            out.push(
                Finding::new(
                    "compare",
                    "compare.region_noise",
                    Severity::Low,
                    0.5,
                    format!("Changed region's noise differs from the rest in {name}"),
                )
                .explain(format!(
                    "Inside the differing region, {name} has a noise level {:.2} vs {:.2} elsewhere.",
                    s.noise_changed, s.noise_unchanged
                ))
                .limits("Noise also depends on the content (smooth reflections, glass, shadows)."),
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resampling::tests::natural;

    #[test]
    fn finds_added_object_and_derived_direction() {
        let original = natural(800, 600, 21);
        // Derived: downscaled, smoothed (generative re-render), with an added object.
        let mut derived =
            image::imageops::blur(&image::imageops::resize(&original, 400, 300, FilterType::Triangle), 1.2);
        for y in 100..160 {
            for x in 150..230 {
                derived.put_pixel(x, y, image::Rgb([240, 180, 30]));
            }
        }
        let r = compare(&derived, &original);
        assert_eq!(r.shift, (0, 0));
        assert!(r.changed_fraction > 0.02 && r.changed_fraction < 0.15, "{}", r.changed_fraction);
        let f = indicators(&r);
        let d = f.iter().find(|x| x.code == "compare.detail_direction").expect("direction");
        assert!(d.title.starts_with("this image looks derived"), "{}", d.title);
    }
}
