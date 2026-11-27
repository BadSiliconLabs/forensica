//! Aligned double JPEG compression (A-DJPG) from DCT coefficient histograms,
//! with primary-quantisation estimation and a per-block localisation map.
//!
//! When a JPEG compressed with step `q1` is decompressed and recompressed with
//! step `q2` on the same 8×8 grid, the requantised coefficients can only take
//! values `round(round(x/q1)·q1/q2)`: some histogram bins become (nearly)
//! unreachable. We search the `q1` that best concentrates the histogram mass on
//! its reachable set. Blocks whose coefficients fall on unreachable bins were
//! likely *not* double-compressed (e.g. pasted from another source).

use serde::Serialize;

use crate::dct::ComponentCoefficients;
use crate::quant::{ijg_table, STD_LUMA, ZIGZAG};

/// Number of low-frequency AC modes (zig-zag 1..=N) examined.
pub const MODES: usize = 15;
/// Histogram range (absolute coefficient values 1..=K).
const K: usize = 40;

/// Per-mode estimate.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ModeEstimate {
    /// Zig-zag index.
    pub zz: usize,
    /// Current quantisation step.
    pub q2: u16,
    /// Estimated primary step (0 = none found).
    pub q1: u16,
    /// Mass of the histogram on the reachable set.
    pub mass: f32,
    /// Expected mass on that set without double compression.
    pub expected: f32,
    /// Number of non-zero samples.
    pub samples: usize,
}

impl ModeEstimate {
    /// Whether this mode shows double-compression evidence.
    pub fn double(&self) -> bool {
        self.q1 != 0 && self.mass > 0.5 && self.mass - self.expected > 0.35
    }
}

/// Image-level result.
#[derive(Debug, Clone, Serialize)]
pub struct DoubleJpeg {
    /// Per-mode estimates.
    pub modes: Vec<ModeEstimate>,
    /// Modes with evidence / modes with enough samples.
    pub evidence: (usize, usize),
    /// Estimated IJG quality of the primary compression (if consistent).
    pub primary_quality: Option<u8>,
    /// Per-block probability (0..1) that the block is *not* double-compressed; empty if no DQ.
    pub map: Vec<f32>,
    /// Map width in blocks.
    pub map_w: usize,
    /// Map height in blocks.
    pub map_h: usize,
}

fn reachable(q1: u16, q2: u16) -> [bool; K + 1] {
    let mut r = [false; K + 1];
    let (q1, q2) = (f32::from(q1), f32::from(q2));
    let mut j = 0f32;
    loop {
        let x = j * q1 / q2;
        if x.floor() > K as f32 {
            break;
        }
        // Exact .5 ties go either way depending on encoder rounding: both bins are reachable.
        let lo = x.floor() as usize;
        if (x - x.floor() - 0.5).abs() < 1e-4 {
            r[lo] = true;
            if lo < K {
                r[lo + 1] = true;
            }
        } else {
            let k = x.round() as usize;
            if k <= K {
                r[k] = true;
            }
        }
        j += 1.0;
    }
    r
}

fn envelope(h: &[f32; K + 1]) -> [f32; K + 1] {
    // Laplacian model: fit ln(h) = a + b·k by least squares on populated bins.
    let pts: Vec<(f32, f32)> = (1..=K).filter(|k| h[*k] > 0.0).map(|k| (k as f32, h[k].ln())).collect();
    let mut out = [0f32; K + 1];
    if pts.len() < 3 {
        out.iter_mut().skip(1).for_each(|v| *v = 1.0);
        return out;
    }
    let n = pts.len() as f32;
    let (sx, sy) = pts.iter().fold((0.0, 0.0), |a, p| (a.0 + p.0, a.1 + p.1));
    let (sxx, sxy) = pts.iter().fold((0.0, 0.0), |a, p| (a.0 + p.0 * p.0, a.1 + p.0 * p.1));
    let b = (n * sxy - sx * sy) / (n * sxx - sx * sx).max(1e-6);
    let a = (sy - b * sx) / n;
    for (k, v) in out.iter_mut().enumerate().skip(1) {
        *v = (a + b * k as f32).exp();
    }
    out
}

fn estimate_mode(values: &[i16], zz: usize, q2: u16) -> ModeEstimate {
    let mut h = [0f32; K + 1];
    let mut samples = 0;
    for v in values {
        let a = v.unsigned_abs() as usize;
        if (1..=K).contains(&a) {
            h[a] += 1.0;
            samples += 1;
        }
    }
    let mut best = ModeEstimate { zz, q2, q1: 0, mass: 0.0, expected: 0.0, samples };
    if samples < 400 {
        return best;
    }
    let env = envelope(&h);
    let total: f32 = h.iter().sum();
    let env_total: f32 = env.iter().skip(1).sum();
    let mut best_gain = 0.0;
    for q1 in 1..=(2 * q2 + 16).min(120) {
        if q1 == q2 {
            continue;
        }
        let r = reachable(q1, q2);
        let frac_bins = (1..=K).filter(|k| r[*k]).count() as f32 / K as f32;
        if frac_bins > 0.85 {
            continue; // Not discriminative.
        }
        let mass = (1..=K).filter(|k| r[*k]).map(|k| h[k]).sum::<f32>() / total;
        let expected = (1..=K).filter(|k| r[*k]).map(|k| env[k]).sum::<f32>() / env_total.max(1e-6);
        let gain = mass - expected;
        // Prefer the largest q1 among near-ties (a multiple of the true step also concentrates mass).
        if gain > best_gain + 0.01 || (gain > best_gain - 0.01 && mass >= best.mass && q1 > best.q1 && gain > 0.0) {
            best_gain = gain.max(best_gain);
            best.q1 = q1;
            best.mass = mass;
            best.expected = expected;
        }
    }
    best
}

/// Analyses the luma component.
pub fn analyse(y: &ComponentCoefficients) -> DoubleJpeg {
    let n = y.used_w * y.used_h;
    let mut modes = Vec::new();
    for (zz, &pos) in ZIGZAG.iter().enumerate().take(MODES + 1).skip(1) {
        let mut vals = Vec::with_capacity(n);
        for by in 0..y.used_h {
            for bx in 0..y.used_w {
                vals.push(y.block(bx, by)[pos]);
            }
        }
        modes.push(estimate_mode(&vals, zz, y.qtable[pos].max(1)));
    }
    let usable: Vec<&ModeEstimate> = modes.iter().filter(|m| m.samples >= 400).collect();
    let dq: Vec<&ModeEstimate> = usable.iter().copied().filter(|m| m.double()).collect();
    let evidence = (dq.len(), usable.len());
    let primary_quality = primary_quality(&dq);
    let is_dq = usable.len() >= 3 && dq.len() * 2 >= usable.len();
    let (map, map_w, map_h) = if is_dq { local_map(y, &dq) } else { (Vec::new(), 0, 0) };
    DoubleJpeg { modes, evidence, primary_quality, map, map_w, map_h }
}

fn primary_quality(dq: &[&ModeEstimate]) -> Option<u8> {
    if dq.len() < 3 {
        return None;
    }
    let mut best = (u32::MAX, 0u8, u16::MAX);
    for q in 1..=100u8 {
        let t = ijg_table(&STD_LUMA, q);
        let mut errs: Vec<u16> = dq.iter().map(|m| t[ZIGZAG[m.zz]].abs_diff(m.q1)).collect();
        let sum: u32 = errs.iter().map(|e| u32::from(*e)).sum();
        errs.sort_unstable();
        let med = errs[errs.len() / 2];
        if sum < best.0 {
            best = (sum, q, med);
        }
    }
    // Accept only if the IJG model explains most estimates (median error ≤ 1 step).
    (best.2 <= 1).then_some(best.1)
}

fn local_map(y: &ComponentCoefficients, dq: &[&ModeEstimate]) -> (Vec<f32>, usize, usize) {
    let (w, h) = (y.used_w, y.used_h);
    let sets: Vec<(usize, [bool; K + 1], f32)> =
        dq.iter().map(|m| (ZIGZAG[m.zz], reachable(m.q1, m.q2), m.expected.clamp(0.05, 0.95))).collect();
    let mut llr = vec![0f32; w * h];
    for by in 0..h {
        for bx in 0..w {
            let b = y.block(bx, by);
            let mut s = 0.0;
            for (pos, r, exp) in &sets {
                let a = b[*pos].unsigned_abs() as usize;
                if a == 0 || a > K {
                    continue;
                }
                // log P(obs | single) - log P(obs | double)
                s += if r[a] { (exp / 0.97).ln() } else { ((1.0 - exp) / 0.03).ln() };
            }
            llr[by * w + bx] = s;
        }
    }
    // Aggregate over 3×3 blocks for stability, then logistic.
    let mut out = vec![0f32; w * h];
    for by in 0..h {
        for bx in 0..w {
            let mut s = 0.0;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let (x, yy) = (bx as i32 + dx, by as i32 + dy);
                    if x >= 0 && yy >= 0 && (x as usize) < w && (yy as usize) < h {
                        s += llr[yy as usize * w + x as usize];
                    }
                }
            }
            out[by * w + bx] = 1.0 / (1.0 + (-s).exp());
        }
    }
    (out, w, h)
}

/// Largest 4-connected region of map values ≥ `t`, in blocks.
pub fn largest_region(map: &[f32], w: usize, h: usize, t: f32) -> usize {
    let mut seen = vec![false; map.len()];
    let mut best = 0;
    for s in 0..map.len() {
        if seen[s] || map[s] < t {
            continue;
        }
        let mut stack = vec![s];
        seen[s] = true;
        let mut n = 0;
        while let Some(i) = stack.pop() {
            n += 1;
            let (x, y) = (i % w, i / w);
            let mut visit = |j: usize| {
                if !seen[j] && map[j] >= t {
                    seen[j] = true;
                    stack.push(j);
                }
            };
            if x > 0 {
                visit(i - 1);
            }
            if x + 1 < w {
                visit(i + 1);
            }
            if y > 0 {
                visit(i - w);
            }
            if y + 1 < h {
                visit(i + w);
            }
        }
        best = best.max(n);
    }
    best
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::dct::decode;
    use crate::testutil::encode;

    /// Natural-ish image: smooth structure plus Gaussian-like noise.
    pub fn natural(w: u32, h: u32, seed: u32) -> image::RgbImage {
        let mut s = u64::from(seed);
        let mut rnd = move || {
            // SplitMix64: no short periods in the low bits (unlike an LCG).
            s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = s;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            ((z ^ (z >> 31)) >> 40) as f32 / 16_777_216.0
        };
        image::RgbImage::from_fn(w, h, |x, y| {
            let n = (rnd() + rnd() + rnd() - 1.5) * 30.0;
            let base = 110.0
                + 60.0 * ((x as f32) * 0.05).sin() * ((y as f32) * 0.033).cos()
                + ((x / 37 + y / 23) % 3) as f32 * 15.0;
            let v = (base + n).clamp(0.0, 255.0);
            image::Rgb([v as u8, (v * 0.8) as u8, (255.0 - v * 0.5) as u8])
        })
    }

    fn recompress(img: &image::RgbImage, q1: u8, q2: u8) -> Vec<u8> {
        let first = image::load_from_memory(&encode(img, q1)).unwrap().to_rgb8();
        encode(&first, q2)
    }

    #[test]
    fn single_compression_is_not_flagged() {
        let j = encode(&natural(512, 512, 1), 90);
        let r = analyse(&decode(&j).unwrap().components[0]);
        assert!(r.map.is_empty(), "evidence {:?} modes {:?}", r.evidence, r.modes);
    }

    #[test]
    fn double_compression_is_detected_with_primary_quality() {
        let j = recompress(&natural(512, 512, 2), 60, 90);
        let r = analyse(&decode(&j).unwrap().components[0]);
        assert!(!r.map.is_empty(), "evidence {:?} modes {:?}", r.evidence, r.modes);
        let q = r.primary_quality.unwrap();
        assert!((50..=70).contains(&q), "primary quality {q}");
    }

    #[test]
    fn pasted_region_is_localised() {
        let orig = natural(512, 512, 3);
        let mut first = image::load_from_memory(&encode(&orig, 55)).unwrap().to_rgb8();
        // Paste a never-compressed region (block aligned) from another image.
        let other = natural(512, 512, 99);
        for y in 128..320 {
            for x in 192..384 {
                first.put_pixel(x, y, *other.get_pixel(x, y));
            }
        }
        let j = encode(&first, 90);
        let c = decode(&j).unwrap();
        let r = analyse(&c.components[0]);
        assert!(!r.map.is_empty(), "evidence {:?}", r.evidence);
        let mean = |x0: usize, x1: usize, y0: usize, y1: usize| {
            let mut s = 0.0;
            let mut n = 0.0;
            for by in y0..y1 {
                for bx in x0..x1 {
                    s += r.map[by * r.map_w + bx];
                    n += 1.0;
                }
            }
            s / n
        };
        let inside = mean(26, 46, 18, 38);
        let outside = mean(2, 20, 44, 62);
        assert!(inside > 0.7 && outside < 0.3, "inside {inside} outside {outside}");
        assert!(largest_region(&r.map, r.map_w, r.map_h, 0.8) > 200);
    }
}
