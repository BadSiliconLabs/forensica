//! A simple f32 grayscale plane and helpers.

use image::RgbImage;

/// Row-major f32 image.
#[derive(Debug, Clone, PartialEq)]
pub struct Plane {
    /// Width.
    pub w: usize,
    /// Height.
    pub h: usize,
    /// Pixels.
    pub data: Vec<f32>,
}

impl Plane {
    /// Zero plane.
    pub fn new(w: usize, h: usize) -> Self {
        Self { w, h, data: vec![0.0; w * h] }
    }

    /// Rec.601 luma of an RGB image (0..255).
    pub fn luma(img: &RgbImage) -> Self {
        let (w, h) = (img.width() as usize, img.height() as usize);
        let data = img
            .pixels()
            .map(|p| 0.299 * f32::from(p.0[0]) + 0.587 * f32::from(p.0[1]) + 0.114 * f32::from(p.0[2]))
            .collect();
        Self { w, h, data }
    }

    /// One RGB channel (0..255).
    pub fn channel(img: &RgbImage, c: usize) -> Self {
        let (w, h) = (img.width() as usize, img.height() as usize);
        Self { w, h, data: img.pixels().map(|p| f32::from(p.0[c.min(2)])).collect() }
    }

    /// Pixel accessor with clamped coordinates.
    #[inline]
    pub fn at(&self, x: isize, y: isize) -> f32 {
        let x = x.clamp(0, self.w as isize - 1) as usize;
        let y = y.clamp(0, self.h as isize - 1) as usize;
        self.data[y * self.w + x]
    }

    /// Mutable pixel.
    #[inline]
    pub fn set(&mut self, x: usize, y: usize, v: f32) {
        self.data[y * self.w + x] = v;
    }

    /// 3×3 median filter.
    pub fn median3(&self) -> Self {
        let mut out = Self::new(self.w, self.h);
        for y in 0..self.h {
            for x in 0..self.w {
                let mut v = [0f32; 9];
                let mut k = 0;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        v[k] = self.at(x as isize + dx, y as isize + dy);
                        k += 1;
                    }
                }
                v.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                out.set(x, y, v[4]);
            }
        }
        out
    }

    /// Mean over non-overlapping `b×b` blocks; returns (plane of block values, bw, bh).
    pub fn block_reduce(&self, b: usize, f: impl Fn(&[f32]) -> f32) -> Self {
        let bw = self.w / b;
        let bh = self.h / b;
        let mut out = Self::new(bw.max(1), bh.max(1));
        let mut buf = Vec::with_capacity(b * b);
        for by in 0..bh {
            for bx in 0..bw {
                buf.clear();
                for y in by * b..(by + 1) * b {
                    buf.extend_from_slice(&self.data[y * self.w + bx * b..y * self.w + (bx + 1) * b]);
                }
                out.set(bx, by, f(&buf));
            }
        }
        out
    }
}

/// Mean of a slice.
pub fn mean(v: &[f32]) -> f32 {
    if v.is_empty() {
        0.0
    } else {
        v.iter().sum::<f32>() / v.len() as f32
    }
}

/// Median (copies).
pub fn median(v: &[f32]) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    let mut c: Vec<f32> = v.to_vec();
    let mid = c.len() / 2;
    let (_, m, _) = c.select_nth_unstable_by(mid, |a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    *m
}

/// Median absolute deviation scaled to σ for Gaussian data.
pub fn mad_sigma(v: &[f32]) -> f32 {
    let m = median(v);
    let d: Vec<f32> = v.iter().map(|x| (x - m).abs()).collect();
    1.4826 * median(&d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_and_blocks() {
        let mut p = Plane::new(4, 4);
        p.set(1, 1, 100.0);
        assert_eq!(p.median3().at(1, 1), 0.0);
        let b = p.block_reduce(2, mean);
        assert_eq!((b.w, b.h), (2, 2));
        assert_eq!(b.at(0, 0), 25.0);
        assert_eq!(median(&[3.0, 1.0, 2.0]), 2.0);
        assert!((mad_sigma(&[1.0, 1.0, 1.0]) - 0.0).abs() < 1e-6);
    }
}
