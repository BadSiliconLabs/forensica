//! Noise residual and local noise-level consistency.

use serde_json::json;

use crate::gray::{mad_sigma, mean, Plane};
use forensica_core::viz::{heatmap, percentile, Colormap};
use forensica_core::{AnalysisModule, Context, Finding, ImageInput, ModuleResult, Result, Severity};

/// Block size for the local noise map.
pub const BLOCK: usize = 32;

/// High-pass residual: luma minus its 3×3 median.
pub fn residual(luma: &Plane) -> Plane {
    let med = luma.median3();
    let data = luma.data.iter().zip(&med.data).map(|(a, b)| a - b).collect();
    Plane { w: luma.w, h: luma.h, data }
}

/// Per-block noise σ (MAD of the residual), block mean luma and block gradient energy.
pub fn block_stats(luma: &Plane, res: &Plane) -> (Plane, Plane, Plane) {
    let sigma = res.block_reduce(BLOCK, mad_sigma);
    let lum = luma.block_reduce(BLOCK, mean);
    let mut grad = Plane::new(luma.w, luma.h);
    for y in 0..luma.h {
        for x in 0..luma.w {
            let gx = luma.at(x as isize + 1, y as isize) - luma.at(x as isize - 1, y as isize);
            let gy = luma.at(x as isize, y as isize + 1) - luma.at(x as isize, y as isize - 1);
            grad.set(x, y, (gx * gx + gy * gy).sqrt());
        }
    }
    let g = grad.block_reduce(BLOCK, mean);
    (sigma, lum, g)
}

/// Finds a connected cluster of blocks with anomalous noise. Returns (cluster size, eligible blocks, z of cluster).
fn anomalous_cluster(sigma: &Plane, lum: &Plane, grad: &Plane) -> Option<(usize, usize, f32)> {
    let gmed = crate::gray::median(&grad.data);
    let eligible: Vec<bool> =
        (0..sigma.data.len()).map(|i| (30.0..225.0).contains(&lum.data[i]) && grad.data[i] <= gmed * 1.5).collect();
    let logs: Vec<f32> = (0..sigma.data.len()).filter(|i| eligible[*i]).map(|i| (sigma.data[i] + 0.25).ln()).collect();
    if logs.len() < 30 {
        return None;
    }
    let m = crate::gray::median(&logs);
    let s = mad_sigma(&logs).max(0.08);
    let z: Vec<f32> =
        (0..sigma.data.len()).map(|i| if eligible[i] { ((sigma.data[i] + 0.25).ln() - m) / s } else { 0.0 }).collect();
    // Flood-fill outliers of the same sign.
    let mut seen = vec![false; z.len()];
    let mut best: Option<(usize, f32)> = None;
    for start in 0..z.len() {
        if seen[start] || z[start].abs() < 4.5 {
            continue;
        }
        let sign = z[start].signum();
        let mut stack = vec![start];
        let mut members = Vec::new();
        seen[start] = true;
        while let Some(i) = stack.pop() {
            members.push(i);
            let (x, y) = ((i % sigma.w) as isize, (i / sigma.w) as isize);
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= sigma.w as isize || ny >= sigma.h as isize {
                    continue;
                }
                let j = ny as usize * sigma.w + nx as usize;
                if !seen[j] && z[j].abs() >= 2.5 && z[j].signum() == sign {
                    seen[j] = true;
                    stack.push(j);
                }
            }
        }
        let zmean = members.iter().map(|i| z[*i].abs()).sum::<f32>() / members.len() as f32;
        if best.is_none_or(|(n, _)| members.len() > n) {
            best = Some((members.len(), zmean));
        }
    }
    best.map(|(n, zm)| (n, logs.len(), zm))
}

/// Noise analysis module.
pub struct NoiseModule;

impl AnalysisModule for NoiseModule {
    fn id(&self) -> &'static str {
        "noise"
    }
    fn name(&self) -> &'static str {
        "Noise residual"
    }
    fn description(&self) -> &'static str {
        "High-pass noise residual and local noise-level consistency"
    }
    fn applies_to(&self, input: &ImageInput) -> bool {
        input.format().decodable()
    }
    fn run(&self, input: &ImageInput, ctx: &mut Context<'_>) -> Result<ModuleResult> {
        let (img, scale) = input.working_rgb(ctx.options.max_pixels)?;
        let luma = Plane::luma(&img);
        let res = residual(&luma);
        ctx.check_cancel()?;
        ctx.report(0.4);
        let abs: Vec<f32> = res.data.iter().map(|v| v.abs()).collect();
        let hi = percentile(&abs, 0.99).max(2.0);
        let mut viz = vec![heatmap(
            "noise.residual",
            "Noise residual",
            "High-frequency content left after removing the image structure (3×3 median). Pasted or retouched \
             regions often show smoother or different-grain noise than the rest of the photo.",
            &abs,
            img.width(),
            img.height(),
            0.0,
            hi,
            Colormap::Gray,
            "flat",
            "noisy",
        )?];
        let (sigma, lum, grad) = block_stats(&luma, &res);
        ctx.check_cancel()?;
        let smax = percentile(&sigma.data, 0.98).max(0.5);
        viz.push(heatmap(
            "noise.level",
            "Local noise level",
            "Estimated sensor-noise strength per 32×32 block. A camera image usually has smoothly varying noise; \
             an isolated patch of very different level can indicate splicing, smoothing or AI inpainting.",
            &sigma.data,
            sigma.w as u32,
            sigma.h as u32,
            0.0,
            smax,
            Colormap::Inferno,
            "low noise",
            "high noise",
        )?);
        let mut findings = Vec::new();
        let cluster = anomalous_cluster(&sigma, &lum, &grad);
        if let Some((n, total, z)) = cluster {
            let frac = n as f32 / total as f32;
            if n >= 6 && (0.01..=0.3).contains(&frac) {
                findings.push(
                    Finding::new("noise", "noise.inconsistent_region", Severity::Low, (0.3 + 0.05 * z).min(0.7), "Region with inconsistent noise level")
                        .explain(format!(
                            "A connected region of {n} blocks ({:.1}% of comparable areas) has a noise level far from the rest of the image (robust z ≈ {z:.1}).",
                            frac * 100.0
                        ))
                        .limits("Noise also varies with brightness, focus, denoising, HDR merging and texture. Inspect the 'Local noise level' map before drawing conclusions.")
                        .with("blocks", n)
                        .with("eligible_blocks", total),
                );
            }
        }
        Ok(ModuleResult {
            findings,
            visualizations: viz,
            raw: json!({ "scale": scale, "block": BLOCK, "median_sigma": crate::gray::median(&sigma.data) }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noisy(w: u32, h: u32, amp: f32, seed: &mut u32) -> image::RgbImage {
        image::RgbImage::from_fn(w, h, |_, _| {
            *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let n = ((*seed >> 24) as f32 / 255.0 - 0.5) * amp;
            let v = (128.0 + n).clamp(0.0, 255.0) as u8;
            image::Rgb([v, v, v])
        })
    }

    #[test]
    fn detects_smoothed_patch() {
        let mut seed = 1;
        let mut img = noisy(512, 512, 40.0, &mut seed);
        for y in 160..352 {
            for x in 160..352 {
                img.put_pixel(x, y, image::Rgb([128, 128, 128]));
            }
        }
        let mut buf = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img).write_to(&mut buf, image::ImageFormat::Png).unwrap();
        let input = ImageInput::from_bytes("n.png", buf.into_inner());
        let r = NoiseModule.run(&input, &mut Context::simple()).unwrap();
        assert_eq!(r.visualizations.len(), 2);
        assert!(r.findings.iter().any(|f| f.code == "noise.inconsistent_region"), "{:?}", r.findings);
    }

    #[test]
    fn uniform_noise_is_quiet() {
        let mut seed = 3;
        let img = noisy(512, 512, 40.0, &mut seed);
        let mut buf = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img).write_to(&mut buf, image::ImageFormat::Png).unwrap();
        let input = ImageInput::from_bytes("n.png", buf.into_inner());
        let r = NoiseModule.run(&input, &mut Context::simple()).unwrap();
        assert!(r.findings.is_empty(), "{:?}", r.findings);
    }
}
