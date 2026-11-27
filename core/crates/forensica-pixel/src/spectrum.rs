//! 2-D Fourier spectrum of the luma channel.

use rustfft::num_complex::Complex32;
use rustfft::FftPlanner;
use serde_json::json;

use crate::gray::Plane;
use forensica_core::viz::{colorize, encode_rgb, percentile, Colormap};
use forensica_core::{AnalysisModule, Context, ImageInput, Legend, ModuleResult, Result, Visualization, VizKind};

/// In-place 2-D FFT of a square `n×n` buffer (row-major).
pub fn fft2(buf: &mut [Complex32], n: usize) {
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(n);
    for row in buf.chunks_mut(n) {
        fft.process(row);
    }
    let mut col = vec![Complex32::new(0.0, 0.0); n];
    for x in 0..n {
        for y in 0..n {
            col[y] = buf[y * n + x];
        }
        fft.process(&mut col);
        for y in 0..n {
            buf[y * n + x] = col[y];
        }
    }
}

/// Largest power of two ≤ `v`, capped.
pub fn pow2_floor(v: usize, cap: usize) -> usize {
    let mut n = 1;
    while n * 2 <= v && n * 2 <= cap {
        n *= 2;
    }
    n
}

/// Log-magnitude spectrum (fft-shifted) of a centred `n×n` crop of `p`, Hann-windowed.
pub fn log_spectrum(p: &Plane, cap: usize) -> (Vec<f32>, usize) {
    let n = pow2_floor(p.w.min(p.h), cap);
    let (ox, oy) = ((p.w - n) / 2, (p.h - n) / 2);
    let hann: Vec<f32> = (0..n).map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / n as f32).cos()).collect();
    let m = crate::gray::mean(&p.data);
    let mut buf: Vec<Complex32> = (0..n * n)
        .map(|i| {
            let (x, y) = (i % n, i / n);
            Complex32::new((p.data[(oy + y) * p.w + ox + x] - m) * hann[x] * hann[y], 0.0)
        })
        .collect();
    fft2(&mut buf, n);
    let mut out = vec![0f32; n * n];
    for y in 0..n {
        for x in 0..n {
            let sx = (x + n / 2) % n;
            let sy = (y + n / 2) % n;
            out[sy * n + sx] = (1.0 + buf[y * n + x].norm()).ln();
        }
    }
    (out, n)
}

/// Renders the spectrum as a standalone visualization.
pub fn render(p: &Plane, id: &str, title: &str, description: &str) -> Result<Visualization> {
    let (s, n) = log_spectrum(p, 1024);
    let lo = percentile(&s, 0.05);
    let hi = percentile(&s, 0.9995);
    let img = colorize(&s, n as u32, n as u32, lo, hi, Colormap::Gray)?;
    Ok(Visualization {
        id: id.into(),
        title: title.into(),
        description: description.into(),
        kind: VizKind::Standalone,
        width: n as u32,
        height: n as u32,
        png: encode_rgb(&img)?,
        legend: Legend { low: "weak".into(), high: "strong".into(), colormap: "gray".into() },
        lines: Vec::new(),
    })
}

/// Spectrum module.
pub struct SpectrumModule;

impl AnalysisModule for SpectrumModule {
    fn id(&self) -> &'static str {
        "spectrum"
    }
    fn name(&self) -> &'static str {
        "Frequency spectrum"
    }
    fn description(&self) -> &'static str {
        "2-D FFT magnitude of the luma channel"
    }
    fn applies_to(&self, input: &ImageInput) -> bool {
        input.format().decodable()
    }
    fn run(&self, input: &ImageInput, ctx: &mut Context<'_>) -> Result<ModuleResult> {
        // Use full resolution crop: downscaling would destroy periodic traces.
        let img = input.rgb()?;
        ctx.check_cancel()?;
        let luma = Plane::luma(&img);
        let v = render(
            &luma,
            "spectrum.luma",
            "Fourier spectrum (luma)",
            "Log-magnitude spectrum of the central crop, centre = low frequencies. Bright isolated dots or regular \
             grids away from the axes indicate periodic patterns: resampling, JPEG grids, halftone screens, or \
             upsampling artefacts typical of some generators. Cross-shaped axes are normal (image borders).",
        )?;
        Ok(ModuleResult { findings: Vec::new(), visualizations: vec![v], raw: json!({}) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn periodic_pattern_produces_peak() {
        let n = 64;
        let mut p = Plane::new(n, n);
        for y in 0..n {
            for x in 0..n {
                p.set(x, y, 128.0 + 50.0 * (2.0 * std::f32::consts::PI * x as f32 / 8.0).cos());
            }
        }
        let (s, m) = log_spectrum(&p, 1024);
        assert_eq!(m, 64);
        // Peak at fx = n/8 = 8 from centre.
        let c = m / 2;
        let peak = s[c * m + c + 8];
        let off = s[(c + 5) * m + c + 3];
        assert!(peak > off + 3.0, "peak {peak} off {off}");
        assert_eq!(pow2_floor(1000, 1024), 512);
    }
}
