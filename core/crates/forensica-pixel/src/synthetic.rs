//! Synthetic-image (AI generation) indicators — weak heuristics, by design.
//!
//! There is no reliable, permissively licensed on-device detector for images from
//! current generators (see docs/decisions.md D14). This module reports only weak,
//! explainable statistical hints, always at *low* severity:
//! * periodic up-sampling artefacts in the noise-residual spectrum (lossless files only,
//!   because JPEG's 8×8 grid produces peaks at the same frequencies);
//! * typical generator canvas sizes without camera metadata.
//!
//! Explicit declarations (C2PA, IPTC DigitalSourceType, generator parameters) are
//! handled by the metadata modules and are far stronger evidence.

use serde_json::json;

use crate::gray::{median, Plane};
use crate::noise::residual;
use crate::spectrum::log_spectrum;
use forensica_core::container::find_all;
use forensica_core::viz::{colorize, encode_rgb, percentile, Colormap};
use forensica_core::{
    AnalysisModule, Context, FileFormat, Finding, ImageInput, Legend, ModuleResult, Result, Severity, Visualization,
    VizKind,
};

/// Ratio of energy at up-sampling grid frequencies (N/4, N/8 on axes and diagonals) to the
/// median of the surrounding annulus, maximised over candidates.
pub fn grid_peak_score(spec: &[f32], n: usize) -> f32 {
    let c = (n / 2) as i32;
    let mut best = 0f32;
    // Period 2 is excluded: Bayer demosaicing in genuine camera images produces it too.
    for q in [4i32, 8] {
        let f = n as i32 / q;
        for (fx, fy) in [(f, 0), (0, f), (f, f), (-f, f)] {
            for sign in [1, -1] {
                let (x, y) = (c + sign * fx, c + sign * fy);
                // The spectrum is periodic: wrap indices (the Nyquist row/column sits on the border).
                let at = |dx: i32, dy: i32| {
                    spec[((y + dy).rem_euclid(n as i32) as usize) * n + (x + dx).rem_euclid(n as i32) as usize]
                };
                let peak = (-1..=1)
                    .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
                    .map(|(dx, dy)| at(dx, dy))
                    .fold(f32::MIN, f32::max);
                let mut ring = Vec::new();
                for dy in -6i32..=6 {
                    for dx in -6i32..=6 {
                        if dx.abs() <= 2 && dy.abs() <= 2 {
                            continue;
                        }
                        ring.push(at(dx, dy));
                    }
                }
                // Log-spectrum values: difference of logs = log ratio.
                best = best.max(peak - median(&ring));
            }
        }
    }
    best
}

const GENERATOR_SIDES: &[u32] =
    &[512, 576, 640, 704, 768, 832, 896, 960, 1024, 1088, 1152, 1216, 1280, 1344, 1408, 1472, 1536, 1792, 2048];

/// Synthetic-image indicators module.
pub struct SyntheticModule;

impl AnalysisModule for SyntheticModule {
    fn id(&self) -> &'static str {
        "synthetic"
    }
    fn name(&self) -> &'static str {
        "AI-generation indicators (weak)"
    }
    fn description(&self) -> &'static str {
        "Spectral up-sampling artefacts and generator canvas sizes; unreliable by nature"
    }
    fn applies_to(&self, input: &ImageInput) -> bool {
        input.format().decodable()
    }
    fn run(&self, input: &ImageInput, ctx: &mut Context<'_>) -> Result<ModuleResult> {
        let img = input.rgb()?;
        let (w, h) = img.dimensions();
        let l = Plane::luma(&img);
        let res = residual(&l);
        ctx.check_cancel()?;
        let (spec, n) = log_spectrum(&res, 1024);
        let lossless = matches!(input.format(), FileFormat::Png | FileFormat::Bmp | FileFormat::Tiff);
        let score = if n >= 128 { grid_peak_score(&spec, n) } else { 0.0 };
        let has_exif = !find_all(input.bytes(), b"Exif\0\0").is_empty() || input.format() == FileFormat::Tiff;
        let mut findings = vec![Finding::new("synthetic", "synthetic.scope", Severity::Info, 1.0, "AI-generation check: weak heuristics only")
            .explain("Forensica has no reliable on-device AI-image detector (none with a permissive licence generalises to current generators). It checks a few explainable statistical hints; explicit declarations (Content Credentials, IPTC, generator parameters) are reported by the metadata modules.")
            .limits("Absence of hints does not mean the image is camera-made; presence does not mean it is generated.")];
        if lossless && score > 1.6 {
            findings.push(
                Finding::new("synthetic", "synthetic.spectral_peaks", Severity::Low, (0.2 + score / 20.0).min(0.45), "Periodic up-sampling artefacts in the noise spectrum")
                    .explain("The noise residual has peaks at N/4 or N/8 frequencies, the signature of up-sampling layers used by many image generators (and by some upscalers).")
                    .limits("Upscaling software, screenshots of scaled content and earlier JPEG compression produce the same peaks.")
                    .with("log_peak_ratio", format!("{score:.2}")),
            );
        }
        let gen_size = GENERATOR_SIDES.contains(&w) && GENERATOR_SIDES.contains(&h);
        if gen_size && !has_exif {
            findings.push(
                Finding::new(
                    "synthetic",
                    "synthetic.canvas_size",
                    Severity::Low,
                    0.25,
                    format!("Typical generator canvas size ({w}×{h}) without camera metadata"),
                )
                .explain("Both sides are multiples of 64 in the range image generators use, and there is no EXIF.")
                .limits("Many edited, cropped or exported photos also have such sizes; very weak hint.")
                .with("size", format!("{w}x{h}")),
            );
        }
        let lo = percentile(&spec, 0.05);
        let hi = percentile(&spec, 0.9995);
        let im = colorize(&spec, n as u32, n as u32, lo, hi, Colormap::Gray)?;
        let viz = vec![Visualization {
            id: "synthetic.residual_spectrum".into(),
            title: "Noise-residual spectrum".into(),
            description: "Spectrum of the high-pass noise residual. Regular dot grids at half/quarter/eighth frequencies suggest up-sampling (generators, upscalers) or, in JPEGs, the 8×8 block grid.".into(),
            kind: VizKind::Standalone,
            width: n as u32,
            height: n as u32,
            png: encode_rgb(&im)?,
            legend: Legend { low: "weak".into(), high: "strong".into(), colormap: "gray".into() },
            lines: Vec::new(),
        }];
        Ok(ModuleResult {
            findings,
            visualizations: viz,
            raw: json!({ "grid_peak_score": score, "lossless": lossless, "generator_size": gen_size, "exif": has_exif }),
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
    fn nearest_upsampled_shows_peaks_natural_does_not() {
        let small = natural(128, 128, 1);
        let up = image::imageops::resize(&small, 512, 512, image::imageops::FilterType::Nearest);
        let r = SyntheticModule.run(&ImageInput::from_bytes("u.png", png(&up)), &mut Context::simple()).unwrap();
        assert!(r.findings.iter().any(|f| f.code == "synthetic.spectral_peaks"), "{}", r.raw);
        assert!(r.findings.iter().any(|f| f.code == "synthetic.canvas_size"));
        assert!(r.findings.iter().all(|f| f.severity <= Severity::Low));
        let n = SyntheticModule
            .run(&ImageInput::from_bytes("n.png", png(&natural(500, 400, 2))), &mut Context::simple())
            .unwrap();
        assert!(!n.findings.iter().any(|f| f.code == "synthetic.spectral_peaks"), "{}", n.raw);
    }
}
