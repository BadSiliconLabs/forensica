//! Copy-move (cloning) detection.
//!
//! Block-matching in the spirit of Fridrich et al. (2003): every 16×16 block of a
//! downscaled luma image is described by the means of its 4×4 cells (robust to
//! mild JPEG noise), blocks are sorted lexicographically, near neighbours in the
//! sorted order with similar features and a large spatial offset vote for their
//! shift vector. Shift vectors with many votes indicate duplicated regions.

use std::collections::HashMap;

use serde_json::json;

use crate::gray::Plane;
use forensica_core::viz::{heatmap, Colormap};
use forensica_core::{AnalysisModule, Context, Finding, ImageInput, Line, ModuleResult, Result, Severity};

const B: usize = 16;
const CELLS: usize = 4;
const FEAT: usize = CELLS * CELLS;
/// Working resolution cap (pixels).
pub const MAX_PIXELS: u64 = 1_000_000;

/// Pairs of (source, target) block origins.
type Pairs = Vec<((usize, usize), (usize, usize))>;

struct Block {
    x: u16,
    y: u16,
    f: [u8; FEAT],
    fine: [f32; FEAT],
}

fn integral(p: &Plane) -> Vec<f64> {
    let (w, h) = (p.w + 1, p.h + 1);
    let mut s = vec![0f64; w * h];
    for y in 0..p.h {
        let mut row = 0f64;
        for x in 0..p.w {
            row += f64::from(p.data[y * p.w + x]);
            s[(y + 1) * w + x + 1] = s[y * w + x + 1] + row;
        }
    }
    s
}

fn rect(s: &[f64], w: usize, x: usize, y: usize, rw: usize, rh: usize) -> f64 {
    let w1 = w + 1;
    s[(y + rh) * w1 + x + rw] - s[y * w1 + x + rw] - s[(y + rh) * w1 + x] + s[y * w1 + x]
}

/// Variance of the 4×4 cell means after removing the best-fit plane a + b·x + c·y.
fn plane_residual_var(f: &[f32; FEAT]) -> f32 {
    // Cell coordinates are centred (-1.5..1.5) so the normal equations decouple.
    let mean = f.iter().sum::<f32>() / FEAT as f32;
    let (mut bx, mut by) = (0f32, 0f32);
    for cy in 0..CELLS {
        for cx in 0..CELLS {
            let (x, y) = (cx as f32 - 1.5, cy as f32 - 1.5);
            bx += x * f[cy * CELLS + cx];
            by += y * f[cy * CELLS + cx];
        }
    }
    // Σx² over the grid = 4 · (2.25+0.25+0.25+2.25) = 20.
    bx /= 20.0;
    by /= 20.0;
    let mut r = 0f32;
    for cy in 0..CELLS {
        for cx in 0..CELLS {
            let (x, y) = (cx as f32 - 1.5, cy as f32 - 1.5);
            r += (f[cy * CELLS + cx] - mean - bx * x - by * y).powi(2);
        }
    }
    r / FEAT as f32
}

/// A detected clone: shift vector and the matched block pairs (in working coordinates).
#[derive(Debug, Clone)]
pub struct CloneMatch {
    /// Shift (dx, dy).
    pub shift: (i32, i32),
    /// Matched (source, target) block origins.
    pub pairs: Vec<((usize, usize), (usize, usize))>,
}

/// Detects copy-move on a luma plane. Returns clones sorted by support.
pub fn detect(l: &Plane, cancel: &dyn Fn() -> bool) -> Vec<CloneMatch> {
    let s = integral(l);
    let stride = 2;
    let cell = B / CELLS;
    let mut blocks = Vec::new();
    let mut y = 0;
    while y + B <= l.h {
        if cancel() {
            return Vec::new();
        }
        let mut x = 0;
        while x + B <= l.w {
            let mut fine = [0f32; FEAT];
            for cy in 0..CELLS {
                for cx in 0..CELLS {
                    fine[cy * CELLS + cx] =
                        (rect(&s, l.w, x + cx * cell, y + cy * cell, cell, cell) / (cell * cell) as f64) as f32;
                }
            }
            // Skip flat and pure-gradient blocks (sky, walls): they match everywhere.
            if plane_residual_var(&fine) > 12.0 {
                let mut f = [0u8; FEAT];
                for (o, v) in f.iter_mut().zip(&fine) {
                    *o = (v / 6.0).clamp(0.0, 255.0) as u8;
                }
                blocks.push(Block { x: x as u16, y: y as u16, f, fine });
            }
            x += stride;
        }
        y += stride;
    }
    blocks.sort_unstable_by_key(|b| b.f);
    let min_dist2 = (3 * B * 3 * B) as i32;
    let mut votes: HashMap<(i32, i32), Pairs> = HashMap::new();
    for i in 0..blocks.len() {
        for j in (i + 1)..(i + 12).min(blocks.len()) {
            let (a, b) = (&blocks[i], &blocks[j]);
            let d: f32 = a.fine.iter().zip(&b.fine).map(|(p, q)| (p - q).powi(2)).sum::<f32>() / FEAT as f32;
            if d > 4.0 {
                continue;
            }
            let (mut dx, mut dy) = (i32::from(b.x) - i32::from(a.x), i32::from(b.y) - i32::from(a.y));
            if dx * dx + dy * dy < min_dist2 {
                continue;
            }
            let (mut src, mut dst) = ((usize::from(a.x), usize::from(a.y)), (usize::from(b.x), usize::from(b.y)));
            // Canonical direction.
            if dx < 0 || (dx == 0 && dy < 0) {
                dx = -dx;
                dy = -dy;
                std::mem::swap(&mut src, &mut dst);
            }
            // Quantise shifts to tolerate ±1 px.
            votes.entry((dx / 2 * 2, dy / 2 * 2)).or_default().push((src, dst));
        }
    }
    let min_votes = ((l.w * l.h) as f32 / 4_000.0).clamp(150.0, 600.0) as usize;
    // Periodic textures (bricks, fences, foliage lattices) make the same blocks match at many
    // shifts with similar support; a genuine clone's shift clearly dominates every other shift
    // that shares its blocks.
    let weak = (min_votes / 4).max(8);
    let keys: Vec<(i32, i32)> = votes.iter().filter(|(_, v)| v.len() >= weak).map(|(k, _)| *k).collect();
    let mut membership: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    for (i, k) in keys.iter().enumerate() {
        if let Some(v) = votes.get(k) {
            for (a, b) in v {
                for p in [*a, *b] {
                    let e = membership.entry(p).or_default();
                    if e.last() != Some(&i) {
                        e.push(i);
                    }
                }
            }
        }
    }
    let dominance = |k: &(i32, i32), v: &Pairs| -> f32 {
        let mut pts: Vec<(usize, usize)> = v.iter().flat_map(|(a, b)| [*a, *b]).collect();
        pts.sort_unstable();
        pts.dedup();
        let mut shared: HashMap<usize, usize> = HashMap::new();
        for p in &pts {
            for &t in membership.get(p).map(Vec::as_slice).unwrap_or_default() {
                if keys[t] != *k {
                    *shared.entry(t).or_default() += 1;
                }
            }
        }
        let rival = shared
            .iter()
            .filter(|(_, n)| **n * 10 >= pts.len())
            .filter_map(|(t, _)| votes.get(&keys[*t]).map(Vec::len))
            .max()
            .unwrap_or(0);
        v.len() as f32 / rival.max(1) as f32
    };
    let mut out: Vec<CloneMatch> = votes
        .iter()
        .filter(|(k, v)| v.len() >= min_votes && dominance(k, v) >= 2.5)
        .map(|(shift, pairs)| CloneMatch { shift: *shift, pairs: pairs.clone() })
        .collect();
    out.sort_by_key(|c| std::cmp::Reverse(c.pairs.len()));
    // Merge near-identical shifts (keep the strongest).
    let mut merged: Vec<CloneMatch> = Vec::new();
    for c in out {
        if !merged.iter().any(|m| (m.shift.0 - c.shift.0).abs() <= 4 && (m.shift.1 - c.shift.1).abs() <= 4) {
            merged.push(c);
        }
    }
    // Many comparable candidates = a lattice-like texture, not a clone.
    if merged.len() >= 2 && merged[0].pairs.len() * 5 < merged[1].pairs.len() * 8 {
        return Vec::new();
    }
    merged.truncate(5);
    merged
}

/// Copy-move module.
pub struct CopyMoveModule;

impl AnalysisModule for CopyMoveModule {
    fn id(&self) -> &'static str {
        "copymove"
    }
    fn name(&self) -> &'static str {
        "Copy-move detection"
    }
    fn description(&self) -> &'static str {
        "Duplicated (cloned) regions with match lines"
    }
    fn applies_to(&self, input: &ImageInput) -> bool {
        input.format().decodable()
    }
    fn run(&self, input: &ImageInput, ctx: &mut Context<'_>) -> Result<ModuleResult> {
        let cap = if ctx.options.max_pixels == 0 { MAX_PIXELS } else { ctx.options.max_pixels.min(MAX_PIXELS) };
        let (img, scale) = input.working_rgb(cap)?;
        let l = Plane::luma(&img);
        let token = ctx.cancel_token().clone();
        let clones = detect(&l, &|| token.is_cancelled());
        ctx.check_cancel()?;
        let inv = 1.0 / scale;
        let mut mask = vec![0f32; l.w * l.h];
        let mut lines = Vec::new();
        for c in &clones {
            // Thin the line overlay: at most ~80 lines per clone, evenly sampled.
            let every = (c.pairs.len() / 80).max(1);
            for (k, (s, d)) in c.pairs.iter().enumerate() {
                for (bx, by) in [*s, *d] {
                    for y in by..(by + B).min(l.h) {
                        for x in bx..(bx + B).min(l.w) {
                            mask[y * l.w + x] = 1.0;
                        }
                    }
                }
                if k % every == 0 && lines.len() < 400 {
                    let h = B as f32 / 2.0;
                    lines.push(Line {
                        x1: (s.0 as f32 + h) * inv,
                        y1: (s.1 as f32 + h) * inv,
                        x2: (d.0 as f32 + h) * inv,
                        y2: (d.1 as f32 + h) * inv,
                    });
                }
            }
        }
        let mut findings = Vec::new();
        let mut viz = Vec::new();
        if let Some(best) = clones.first() {
            let area = mask.iter().filter(|v| **v > 0.0).count() as f32 / mask.len() as f32;
            findings.push(
                Finding::new("copymove", "copymove.detected", Severity::High, (0.5 + best.pairs.len() as f32 / 2000.0).min(0.9), "Duplicated regions found")
                    .explain(format!(
                        "{} group(s) of blocks are near-identical copies of other parts of the image, shifted by a constant offset (strongest: {} matches, shift ≈ {:.0},{:.0} px). Cloning is used to remove or duplicate objects.",
                        clones.len(),
                        best.pairs.len(),
                        best.shift.0 as f32 * inv,
                        best.shift.1 as f32 * inv
                    ))
                    .limits("Repetitive structures (windows, tiles, fences, text, patterned fabric) and flat gradients produce genuine-looking matches. Rotated or scaled clones are not detected.")
                    .with("groups", clones.len())
                    .with("marked_area", format!("{:.1}%", area * 100.0)),
            );
            let mut v = heatmap(
                "copymove.mask",
                "Copy-move matches",
                "Blocks that have a near-identical copy elsewhere at a consistent offset. Lines connect sample source/target pairs.",
                &mask,
                l.w as u32,
                l.h as u32,
                0.0,
                1.0,
                Colormap::Inferno,
                "unique",
                "duplicated",
            )?;
            v.lines = lines;
            viz.push(v);
        }
        Ok(ModuleResult {
            findings,
            visualizations: viz,
            raw: json!({ "scale": scale, "clones": clones.iter().map(|c| json!({"shift": [c.shift.0, c.shift.1], "matches": c.pairs.len()})).collect::<Vec<_>>() }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resampling::tests::natural;

    fn png(img: &image::RgbImage) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img.clone()).write_to(&mut buf, image::ImageFormat::Png).unwrap();
        buf.into_inner()
    }

    #[test]
    fn detects_cloned_patch_even_after_jpeg() {
        let mut img = natural(400, 300, 5);
        for y in 0..64 {
            for x in 0..80 {
                let p = *img.get_pixel(40 + x, 50 + y);
                img.put_pixel(250 + x, 180 + y, p);
            }
        }
        let mut j = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut j, 90).encode_image(&img).unwrap();
        let r = CopyMoveModule.run(&ImageInput::from_bytes("c.jpg", j), &mut Context::simple()).unwrap();
        assert!(r.findings.iter().any(|f| f.code == "copymove.detected"), "{}", r.raw);
        let v = &r.visualizations[0];
        assert!(!v.lines.is_empty());
        let l = v.lines[0];
        assert!(((l.x2 - l.x1).abs() - 210.0).abs() < 4.0 && ((l.y2 - l.y1).abs() - 130.0).abs() < 4.0, "{l:?}");
    }

    #[test]
    fn periodic_texture_is_quiet() {
        let base = natural(400, 300, 8);
        let img = image::RgbImage::from_fn(400, 300, |x, y| {
            let t = 40.0 * ((x as f32 * 0.11).sin() * (y as f32 * 0.07).cos())
                + 25.0 * (x as f32 * 0.53 + y as f32 * 0.31).sin();
            let b = base.get_pixel(x, y).0[0] as f32 * 0.2;
            let v = (120.0 + t + b).clamp(0.0, 255.0) as u8;
            image::Rgb([v, v, v])
        });
        let r = CopyMoveModule.run(&ImageInput::from_bytes("p.png", png(&img)), &mut Context::simple()).unwrap();
        assert!(r.findings.is_empty(), "{}", r.raw);
    }

    #[test]
    fn natural_image_is_quiet() {
        let r = CopyMoveModule
            .run(&ImageInput::from_bytes("n.png", png(&natural(400, 300, 6))), &mut Context::simple())
            .unwrap();
        assert!(r.findings.is_empty(), "{}", r.raw);
    }
}
