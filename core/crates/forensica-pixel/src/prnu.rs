//! PRNU (photo-response non-uniformity) sensor fingerprinting.
//!
//! Each sensor multiplies light by a fixed, pixel-specific pattern K. From N
//! reference photos taken with one camera we estimate K = Σ Wᵢ·Iᵢ / Σ Iᵢ², where
//! Wᵢ is the noise residual of image Iᵢ. A query image's residual is then
//! cross-correlated with I·K and scored with the Peak-to-Correlation-Energy
//! (PCE). PCE > 60 is the usual decision threshold in the literature.

use rustfft::num_complex::Complex32;
use rustfft::FftPlanner;
use serde::Serialize;

use crate::gray::Plane;
use forensica_core::{Error, Result};

/// Side of the central crop used (power of two).
pub const CROP: usize = 1024;
/// PCE threshold commonly used to declare a match.
pub const PCE_THRESHOLD: f32 = 60.0;

/// Result of a PRNU comparison.
#[derive(Debug, Clone, Serialize)]
pub struct PrnuResult {
    /// Peak-to-correlation energy.
    pub pce: f32,
    /// Peak offset (dx, dy); (0,0) for an unshifted match.
    pub peak: (i32, i32),
    /// References used.
    pub references: usize,
    /// References skipped (size mismatch or decode failure).
    pub skipped: usize,
    /// Crop size used.
    pub crop: usize,
}

/// Adaptive local Wiener denoising residual (5×5), with row/column means removed.
pub fn residual(p: &Plane) -> Plane {
    let (w, h) = (p.w, p.h);
    let r = 2isize;
    // Integral images of x and x² for fast local statistics.
    let mut s1 = vec![0f64; (w + 1) * (h + 1)];
    let mut s2 = vec![0f64; (w + 1) * (h + 1)];
    for y in 0..h {
        let (mut a, mut b) = (0f64, 0f64);
        for x in 0..w {
            let v = f64::from(p.data[y * w + x]);
            a += v;
            b += v * v;
            s1[(y + 1) * (w + 1) + x + 1] = s1[y * (w + 1) + x + 1] + a;
            s2[(y + 1) * (w + 1) + x + 1] = s2[y * (w + 1) + x + 1] + b;
        }
    }
    let rect = |s: &[f64], x0: usize, y0: usize, x1: usize, y1: usize| {
        s[y1 * (w + 1) + x1] - s[y0 * (w + 1) + x1] - s[y1 * (w + 1) + x0] + s[y0 * (w + 1) + x0]
    };
    let noise_var = 9.0f64;
    let mut out = Plane::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let x0 = (x as isize - r).max(0) as usize;
            let y0 = (y as isize - r).max(0) as usize;
            let x1 = (x as isize + r + 1).min(w as isize) as usize;
            let y1 = (y as isize + r + 1).min(h as isize) as usize;
            let n = ((x1 - x0) * (y1 - y0)) as f64;
            let m = rect(&s1, x0, y0, x1, y1) / n;
            let var = (rect(&s2, x0, y0, x1, y1) / n - m * m).max(0.0);
            let v = f64::from(p.data[y * w + x]);
            let den = m + (var - noise_var).max(0.0) / var.max(1e-6) * (v - m);
            out.data[y * w + x] = (v - den) as f32;
        }
    }
    zero_mean(&mut out);
    out
}

/// Removes row and column means (suppresses linear patterns such as JPEG/readout artefacts).
pub fn zero_mean(p: &mut Plane) {
    for y in 0..p.h {
        let row = &mut p.data[y * p.w..(y + 1) * p.w];
        let m = row.iter().sum::<f32>() / p.w as f32;
        row.iter_mut().for_each(|v| *v -= m);
    }
    for x in 0..p.w {
        let m = (0..p.h).map(|y| p.data[y * p.w + x]).sum::<f32>() / p.h as f32;
        for y in 0..p.h {
            p.data[y * p.w + x] -= m;
        }
    }
}

/// Central square crop of a luma plane.
pub fn crop(p: &Plane, n: usize) -> Option<Plane> {
    if p.w < n || p.h < n {
        return None;
    }
    let (ox, oy) = ((p.w - n) / 2, (p.h - n) / 2);
    let mut out = Plane::new(n, n);
    for y in 0..n {
        out.data[y * n..(y + 1) * n].copy_from_slice(&p.data[(oy + y) * p.w + ox..(oy + y) * p.w + ox + n]);
    }
    Some(out)
}

/// Fingerprint estimator accumulating references.
pub struct Fingerprint {
    n: usize,
    num: Vec<f32>,
    den: Vec<f32>,
    count: usize,
}

impl Fingerprint {
    /// New estimator for `n×n` crops.
    pub fn new(n: usize) -> Self {
        Self { n, num: vec![0.0; n * n], den: vec![0.0; n * n], count: 0 }
    }

    /// Adds one reference luma crop.
    pub fn add(&mut self, img: &Plane) {
        let w = residual(img);
        for i in 0..self.n * self.n {
            self.num[i] += w.data[i] * img.data[i];
            self.den[i] += img.data[i] * img.data[i];
        }
        self.count += 1;
    }

    /// Number of references added.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Final K (zero-mean).
    pub fn finish(&self) -> Plane {
        let mut k = Plane::new(self.n, self.n);
        for i in 0..self.n * self.n {
            k.data[i] = self.num[i] / self.den[i].max(1.0);
        }
        zero_mean(&mut k);
        k
    }
}

/// PCE of the circular cross-correlation between the query residual and I·K.
pub fn pce(query: &Plane, k: &Plane) -> (f32, (i32, i32)) {
    let n = query.w;
    let w = residual(query);
    let ik: Vec<f32> = query.data.iter().zip(&k.data).map(|(i, k)| i * k).collect();
    let m_ik = ik.iter().sum::<f32>() / ik.len() as f32;
    let mut a: Vec<Complex32> = w.data.iter().map(|v| Complex32::new(*v, 0.0)).collect();
    let mut b: Vec<Complex32> = ik.iter().map(|v| Complex32::new(v - m_ik, 0.0)).collect();
    crate::spectrum::fft2(&mut a, n);
    crate::spectrum::fft2(&mut b, n);
    let mut c: Vec<Complex32> = a.iter().zip(&b).map(|(x, y)| x * y.conj()).collect();
    // Inverse FFT via conjugation trick.
    c.iter_mut().for_each(|v| *v = v.conj());
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(n);
    for row in c.chunks_mut(n) {
        fft.process(row);
    }
    let mut col = vec![Complex32::new(0.0, 0.0); n];
    for x in 0..n {
        for y in 0..n {
            col[y] = c[y * n + x];
        }
        fft.process(&mut col);
        for y in 0..n {
            c[y * n + x] = col[y];
        }
    }
    let corr: Vec<f32> = c.iter().map(|v| v.re).collect();
    let (pi, pv) = corr.iter().enumerate().fold((0, f32::MIN), |acc, (i, v)| if *v > acc.1 { (i, *v) } else { acc });
    let (px, py) = ((pi % n) as i32, (pi / n) as i32);
    let mut energy = 0f64;
    let mut count = 0f64;
    for (i, v) in corr.iter().enumerate() {
        let (x, y) = ((i % n) as i32, (i / n) as i32);
        let dx = (x - px).rem_euclid(n as i32).min((px - x).rem_euclid(n as i32));
        let dy = (y - py).rem_euclid(n as i32).min((py - y).rem_euclid(n as i32));
        if dx <= 5 && dy <= 5 {
            continue;
        }
        energy += f64::from(*v) * f64::from(*v);
        count += 1.0;
    }
    let pce = (f64::from(pv) * f64::from(pv) * f64::from(pv.signum()) / (energy / count.max(1.0)).max(1e-12)) as f32;
    let wrap = |v: i32| if v > n as i32 / 2 { v - n as i32 } else { v };
    (pce, (wrap(px), wrap(py)))
}

/// Compares a query image with references (all decoded RGB). References with a
/// different resolution than the query are skipped.
pub fn compare(query: &image::RgbImage, references: &[image::RgbImage]) -> Result<PrnuResult> {
    let q = Plane::luma(query);
    let n = crate::spectrum::pow2_floor(q.w.min(q.h), CROP);
    if n < 128 {
        return Err(Error::InvalidArgument("image too small for PRNU".into()));
    }
    let qc = crop(&q, n).ok_or_else(|| Error::InvalidArgument("crop failed".into()))?;
    let mut fp = Fingerprint::new(n);
    let mut skipped = 0;
    for r in references {
        if r.dimensions() != query.dimensions() {
            skipped += 1;
            continue;
        }
        match crop(&Plane::luma(r), n) {
            Some(c) => fp.add(&c),
            None => skipped += 1,
        }
    }
    if fp.count() == 0 {
        return Err(Error::InvalidArgument("no reference photo has the same resolution as the image".into()));
    }
    let (pce, peak) = pce(&qc, &fp.finish());
    Ok(PrnuResult { pce, peak, references: fp.count(), skipped, crop: n })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Rng(u64);
    impl Rng {
        fn f(&mut self) -> f32 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            ((z ^ (z >> 31)) >> 40) as f32 / 16_777_216.0 - 0.5
        }
    }

    fn sensor(seed: u64, n: usize) -> Vec<f32> {
        let mut r = Rng(seed);
        (0..n * n).map(|_| r.f() * 0.06).collect()
    }

    fn shot(k: &[f32], n: usize, scene_seed: u64) -> image::RgbImage {
        let mut r = Rng(scene_seed);
        let (a, b) = (r.f() * 0.1, r.f() * 0.1);
        image::RgbImage::from_fn(n as u32, n as u32, |x, y| {
            let s = 120.0 + 60.0 * ((x as f32) * a).sin() * ((y as f32) * b).cos();
            let v = s * (1.0 + k[y as usize * n + x as usize]) + r.f() * 4.0;
            let v = v.clamp(0.0, 255.0) as u8;
            image::Rgb([v, v, v])
        })
    }

    #[test]
    fn same_camera_matches_other_does_not() {
        let n = 256;
        let cam_a = sensor(1, n);
        let cam_b = sensor(2, n);
        let refs: Vec<_> = (0..8).map(|i| shot(&cam_a, n, 100 + i)).collect();
        let same = compare(&shot(&cam_a, n, 999), &refs).unwrap();
        let other = compare(&shot(&cam_b, n, 998), &refs).unwrap();
        assert!(same.pce > PCE_THRESHOLD, "same {}", same.pce);
        assert!(other.pce < PCE_THRESHOLD, "other {}", other.pce);
        assert_eq!(same.peak, (0, 0));
        assert_eq!(same.references, 8);
    }

    #[test]
    fn resolution_mismatch_is_reported() {
        let n = 256;
        let k = sensor(3, n);
        let refs = vec![image::RgbImage::new(100, 100)];
        assert!(compare(&shot(&k, n, 1), &refs).is_err());
    }
}
