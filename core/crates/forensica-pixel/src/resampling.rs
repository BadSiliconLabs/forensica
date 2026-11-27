//! Resampling detection (Kirchner 2008, "fast and reliable resampling detection"):
//! a fixed linear predictor's residual is mapped to a probability map ("p-map");
//! interpolation makes the p-map periodic, which shows up as peaks in its spectrum.

use rayon::prelude::*;
use rustfft::num_complex::Complex32;
use serde_json::json;

use crate::gray::{median, Plane};
use crate::spectrum::fft2;
use forensica_core::viz::{colorize, encode_rgb, heatmap, percentile, Colormap};
use forensica_core::{
    AnalysisModule, Context, Finding, ImageInput, Legend, ModuleResult, Result, Severity, Visualization, VizKind,
};

/// Kirchner's p-map with the standard 3×3 predictor.
pub fn pmap(l: &Plane, x0: usize, y0: usize, n: usize) -> Vec<f32> {
    let mut out = vec![0f32; n * n];
    for y in 0..n {
        for x in 0..n {
            let (cx, cy) = ((x0 + x) as isize, (y0 + y) as isize);
            let pred = 0.5 * (l.at(cx - 1, cy) + l.at(cx + 1, cy) + l.at(cx, cy - 1) + l.at(cx, cy + 1))
                - 0.25 * (l.at(cx - 1, cy - 1) + l.at(cx + 1, cy - 1) + l.at(cx - 1, cy + 1) + l.at(cx + 1, cy + 1));
            let e = l.at(cx, cy) - pred;
            out[y * n + x] = (-e * e / 2.0).exp();
        }
    }
    out
}

/// Centred magnitude spectrum of a p-map window (mean removed, Hann window).
pub fn pmap_spectrum(p: &[f32], n: usize) -> Vec<f32> {
    let m = p.iter().sum::<f32>() / p.len() as f32;
    let hann: Vec<f32> = (0..n).map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / n as f32).cos()).collect();
    let mut buf: Vec<Complex32> =
        (0..n * n).map(|i| Complex32::new((p[i] - m) * hann[i % n] * hann[i / n], 0.0)).collect();
    fft2(&mut buf, n);
    let mut out = vec![0f32; n * n];
    for y in 0..n {
        for x in 0..n {
            out[((y + n / 2) % n) * n + (x + n / 2) % n] = buf[y * n + x].norm();
        }
    }
    out
}

/// Peak-to-median ratio of the spectrum outside the low-frequency disc and away
/// from the frequencies of an 8×8 JPEG grid.
pub fn peak_score(s: &[f32], n: usize) -> (f32, (i32, i32)) {
    let c = (n / 2) as i32;
    let r0 = (n / 10) as i32;
    let jpeg = |k: i32| {
        let step = n as i32 / 8;
        let m = k.rem_euclid(step);
        m <= 1 || m >= step - 1
    };
    let mut band = Vec::with_capacity(n * n);
    let mut best = (0f32, (0, 0));
    for y in 0..n as i32 {
        for x in 0..n as i32 {
            let (fx, fy) = (x - c, y - c);
            if fx * fx + fy * fy < r0 * r0 {
                continue;
            }
            if jpeg(fx) && jpeg(fy) {
                continue;
            }
            let v = s[(y as usize) * n + x as usize];
            band.push(v);
            if v > best.0 {
                best = (v, (fx, fy));
            }
        }
    }
    let med = median(&band).max(1e-6);
    (best.0 / med, best.1)
}

/// Global score above which resampling is reported.
pub const GLOBAL_T: f32 = 18.0;
/// Local window score above which a window is considered resampled.
pub const LOCAL_T: f32 = 14.0;

/// Resampling module.
pub struct ResamplingModule;

impl AnalysisModule for ResamplingModule {
    fn id(&self) -> &'static str {
        "resampling"
    }
    fn name(&self) -> &'static str {
        "Resampling detection"
    }
    fn description(&self) -> &'static str {
        "Periodic interpolation traces from resizing/rotation (global and local)"
    }
    fn applies_to(&self, input: &ImageInput) -> bool {
        input.format().decodable()
    }
    fn run(&self, input: &ImageInput, ctx: &mut Context<'_>) -> Result<ModuleResult> {
        // Full resolution: downscaling would itself resample.
        let img = input.rgb()?;
        let l = Plane::luma(&img);
        let n = crate::spectrum::pow2_floor(l.w.min(l.h), 512);
        if n < 64 {
            return Ok(ModuleResult::default());
        }
        let (gx, gy) = ((l.w - n) / 2, (l.h - n) / 2);
        let gspec = pmap_spectrum(&pmap(&l, gx, gy, n), n);
        let (gscore, gpeak) = peak_score(&gspec, n);
        ctx.check_cancel()?;
        ctx.report(0.3);

        let win = 128;
        let step = if l.w * l.h > 8_000_000 { 128 } else { 64 };
        let (mw, mh) = ((l.w.saturating_sub(win)) / step + 1, (l.h.saturating_sub(win)) / step + 1);
        let cancel = ctx.cancel_token().clone();
        let local: Vec<f32> = (0..mw * mh)
            .into_par_iter()
            .map(|i| {
                if cancel.is_cancelled() {
                    return 0.0;
                }
                let (mx, my) = (i % mw, i / mw);
                let s = pmap_spectrum(&pmap(&l, mx * step, my * step, win), win);
                peak_score(&s, win).0
            })
            .collect();
        ctx.check_cancel()?;

        let mut findings = Vec::new();
        if gscore >= GLOBAL_T {
            findings.push(
                Finding::new("resampling", "resample.global", Severity::Medium, (0.4 + gscore / 100.0).min(0.85), "Image shows resampling traces")
                    .explain("The interpolation residual is periodic: the image (or its central part) was resized, rotated or otherwise geometrically transformed after capture.")
                    .limits("Phones routinely resample (digital zoom, stabilisation, lens correction); textures with a regular pattern can produce false peaks. Strong JPEG compression hides the traces.")
                    .with("peak_ratio", format!("{gscore:.1}"))
                    .with("peak_frequency", format!("{},{}", gpeak.0, gpeak.1)),
            );
        } else {
            let hot = local.iter().filter(|v| **v >= LOCAL_T).count();
            let frac = hot as f32 / local.len().max(1) as f32;
            if hot >= 2 && frac < 0.25 {
                findings.push(
                    Finding::new("resampling", "resample.local", Severity::Medium, (0.35 + frac).min(0.7), "Local resampling traces")
                        .explain(format!("{hot} window(s) show periodic interpolation traces while the image as a whole does not: an object may have been scaled or rotated and pasted in."))
                        .limits("Regular textures (grids, fabrics, screens) produce similar peaks; inspect the map and the windows' content.")
                        .with("windows", hot),
                );
            }
        }
        let mut viz = vec![heatmap(
            "resample.map",
            "Resampling map",
            "Strength of periodic interpolation traces per 128×128 window. Uniformly bright: the whole image was resized. Isolated bright areas: possibly scaled/rotated insertions — or regular textures.",
            &local,
            mw as u32,
            mh as u32,
            0.0,
            LOCAL_T * 2.0,
            Colormap::Inferno,
            "none",
            "periodic",
        )?];
        viz.push(spectrum_viz(&gspec, n)?);
        Ok(ModuleResult {
            findings,
            visualizations: viz,
            raw: json!({ "global_score": gscore, "global_peak": [gpeak.0, gpeak.1], "window": win, "step": step, "max_local": local.iter().copied().fold(0.0, f32::max) }),
        })
    }
}

fn spectrum_viz(s: &[f32], n: usize) -> Result<Visualization> {
    let logs: Vec<f32> = s.iter().map(|v| (1.0 + v).ln()).collect();
    let lo = percentile(&logs, 0.5);
    let hi = percentile(&logs, 0.9995);
    let img = colorize(&logs, n as u32, n as u32, lo, hi, Colormap::Gray)?;
    Ok(Visualization {
        id: "resample.spectrum".into(),
        title: "P-map spectrum (resampling)".into(),
        description: "Spectrum of the interpolation-probability map of the central crop. Sharp isolated peaks away from the centre indicate resampling.".into(),
        kind: VizKind::Standalone,
        width: n as u32,
        height: n as u32,
        png: encode_rgb(&img)?,
        legend: Legend { low: "weak".into(), high: "strong".into(), colormap: "gray".into() },
        lines: Vec::new(),
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

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
            let base = 110.0 + 60.0 * ((x as f32) * 0.05).sin() * ((y as f32) * 0.033).cos();
            let v = (base + n).clamp(0.0, 255.0) as u8;
            image::Rgb([v, v, v])
        })
    }

    fn score(img: &image::RgbImage) -> f32 {
        let l = Plane::luma(img);
        let n = 256;
        peak_score(&pmap_spectrum(&pmap(&l, 0, 0, n), n), n).0
    }

    #[test]
    fn upscaled_scores_higher_than_original() {
        let orig = natural(400, 400, 1);
        let up = image::imageops::resize(&orig, 520, 520, image::imageops::FilterType::Triangle);
        let (so, su) = (score(&orig), score(&up));
        assert!(so < GLOBAL_T, "original {so}");
        assert!(su > GLOBAL_T, "upscaled {su}");
    }

    #[test]
    fn module_reports_global_resampling() {
        let up = image::imageops::resize(&natural(400, 400, 2), 560, 560, image::imageops::FilterType::Triangle);
        let mut buf = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(up).write_to(&mut buf, image::ImageFormat::Png).unwrap();
        let r =
            ResamplingModule.run(&ImageInput::from_bytes("r.png", buf.into_inner()), &mut Context::simple()).unwrap();
        assert!(r.findings.iter().any(|f| f.code == "resample.global"), "{}", r.raw);
    }

    #[test]
    fn original_is_quiet() {
        let mut buf = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(natural(512, 512, 3)).write_to(&mut buf, image::ImageFormat::Png).unwrap();
        let r =
            ResamplingModule.run(&ImageInput::from_bytes("o.png", buf.into_inner()), &mut Context::simple()).unwrap();
        assert!(r.findings.is_empty(), "{}", r.raw);
    }
}
