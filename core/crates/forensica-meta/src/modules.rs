//! [`AnalysisModule`] implementations for metadata, provenance and generator hints.

use std::time::{SystemTime, UNIX_EPOCH};

use image::imageops::FilterType;
use serde_json::json;

use crate::rules::{self, MetaBundle, ThumbCompare};
use crate::table::Group;
use crate::{c2pa_reader, exif_read, generator, icc, iptc, textchunks, xmp};
use forensica_core::viz::image_viz;
use forensica_core::{AnalysisModule, Context, ImageInput, ModuleResult, Result, VizKind};

/// EXIF/XMP/IPTC/ICC/MakerNotes/GPS/thumbnail viewer and anomaly rules.
pub struct MetadataModule;

/// C2PA / Content Credentials.
pub struct ProvenanceModule;

/// Text chunks and AI-generator hints.
pub struct GeneratorHintsModule;

fn file_group(input: &ImageInput, dims: Option<(u32, u32)>) -> Group {
    let mut g = Group::new("File");
    g.push("Name", input.name());
    g.push("Size", format!("{} bytes", input.bytes().len()));
    g.push("SHA-256", input.sha256());
    g.push("Detected format", format!("{:?}", input.format()));
    g.push("MIME", input.format().mime());
    if let Some((w, h)) = dims {
        g.push("Dimensions", format!("{w} × {h} ({:.1} MP)", f64::from(w) * f64::from(h) / 1e6));
    }
    g
}

fn compare_thumbnail(input: &ImageInput, thumb: &[u8]) -> Option<ThumbCompare> {
    let t = image::load_from_memory_with_format(thumb, image::ImageFormat::Jpeg).ok()?.to_luma8();
    let main = input.rgb().ok()?;
    let main_aspect = main.width() as f32 / main.height().max(1) as f32;
    let thumb_aspect = t.width() as f32 / t.height().max(1) as f32;
    // Thumbnails may be letterboxed: crop the thumbnail to the main aspect before comparing.
    let (tw, th) = (t.width() as f32, t.height() as f32);
    let (cw, ch) = if thumb_aspect > main_aspect { (th * main_aspect, th) } else { (tw, tw / main_aspect.max(1e-3)) };
    let cx = ((tw - cw) / 2.0).max(0.0) as u32;
    let cy = ((th - ch) / 2.0).max(0.0) as u32;
    let crop = image::imageops::crop_imm(&t, cx, cy, (cw as u32).max(1), (ch as u32).max(1)).to_image();
    let n = 32;
    let a = image::imageops::resize(&crop, n, n, FilterType::Triangle);
    let gray = image::DynamicImage::ImageRgb8((*main).clone()).to_luma8();
    let b = image::imageops::resize(&gray, n, n, FilterType::Triangle);
    // Normalise brightness/contrast to tolerate tone edits.
    let norm = |img: &image::GrayImage| -> Vec<f32> {
        let v: Vec<f32> = img.pixels().map(|p| f32::from(p.0[0])).collect();
        let m = v.iter().sum::<f32>() / v.len() as f32;
        let sd = (v.iter().map(|x| (x - m).powi(2)).sum::<f32>() / v.len() as f32).sqrt().max(1.0);
        v.into_iter().map(|x| (x - m) / sd).collect()
    };
    let (va, vb) = (norm(&a), norm(&b));
    let diff = va.iter().zip(&vb).map(|(x, y)| (x - y).abs()).sum::<f32>() / va.len() as f32 / 4.0;
    // Letterboxed thumbnails keep their own aspect; report aspect of the content area.
    let letterboxed =
        (thumb_aspect - main_aspect).abs() / main_aspect > 0.06 && has_black_bars(&t, thumb_aspect > main_aspect);
    Some(ThumbCompare { main_aspect, thumb_aspect: if letterboxed { main_aspect } else { thumb_aspect }, diff })
}

fn has_black_bars(t: &image::GrayImage, vertical: bool) -> bool {
    let (w, h) = t.dimensions();
    let mean = |xs: &mut dyn Iterator<Item = u8>| {
        let v: Vec<u32> = xs.map(u32::from).collect();
        v.iter().sum::<u32>() / (v.len().max(1) as u32)
    };
    if vertical {
        mean(&mut (0..h).map(|y| t.get_pixel(0, y).0[0])) < 12
            && mean(&mut (0..h).map(|y| t.get_pixel(w - 1, y).0[0])) < 12
    } else {
        mean(&mut (0..w).map(|x| t.get_pixel(x, 0).0[0])) < 12
            && mean(&mut (0..w).map(|x| t.get_pixel(x, h - 1).0[0])) < 12
    }
}

impl AnalysisModule for MetadataModule {
    fn id(&self) -> &'static str {
        rules::MODULE
    }
    fn name(&self) -> &'static str {
        "Metadata"
    }
    fn version(&self) -> u32 {
        1
    }
    fn description(&self) -> &'static str {
        "EXIF, XMP, IPTC, ICC, MakerNotes, GPS, thumbnail and consistency rules"
    }
    fn applies_to(&self, _: &ImageInput) -> bool {
        true
    }
    fn run(&self, input: &ImageInput, ctx: &mut Context<'_>) -> Result<ModuleResult> {
        let bytes = input.bytes();
        let fmt = input.format();
        let exif = exif_read::read(bytes);
        ctx.report(0.2);
        let xmp = xmp::parse(bytes);
        let iptc = iptc::read(bytes);
        let icc = icc::extract(bytes, fmt).and_then(|p| icc::parse(&p));
        let text = textchunks::read(bytes, fmt);
        ctx.check_cancel()?;
        ctx.report(0.5);
        let dims = if fmt.decodable() { input.rgb().ok().map(|i| i.dimensions()) } else { None };
        let thumb = exif.summary.thumbnail.as_deref().and_then(|t| compare_thumbnail(input, t));
        ctx.report(0.8);

        let mut groups = vec![file_group(input, dims)];
        groups.extend(exif.groups.iter().cloned());
        if let Some(mn) = &exif.summary.makernote {
            let mut g = Group::new("MakerNote");
            g.push("Vendor format", mn.vendor.clone());
            g.push("Size", format!("{} bytes", mn.size));
            if let Some(n) = mn.entries {
                g.push("IFD entries", n.to_string());
            }
            groups.push(g);
        }
        if xmp.packets > 0 {
            groups.push(xmp.group());
        }
        groups.extend(iptc.groups());
        if let Some(i) = &icc {
            groups.push(i.group());
        }
        if let Some(g) = textchunks::group(&text) {
            groups.push(g);
        }

        let mut visualizations = Vec::new();
        if let Some(t) = &exif.summary.thumbnail {
            if let Ok(img) = image::load_from_memory_with_format(t, image::ImageFormat::Jpeg) {
                let v = image_viz(
                    "meta.thumbnail",
                    "Embedded EXIF thumbnail",
                    "Preview stored by the camera or software. If it differs from the main image, the image may have been edited without regenerating the thumbnail.",
                    img.to_rgb8(),
                    VizKind::Standalone,
                )?;
                visualizations.push(v);
            }
        }

        let bundle = MetaBundle {
            format: Some(fmt),
            dims,
            extension_mismatch: input.extension_mismatch(),
            exif: exif.summary.clone(),
            xmp: xmp.clone(),
            iptc: iptc.clone(),
            thumb,
            now_unix: SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
        };
        let findings = rules::evaluate(&bundle);
        let raw = json!({
            "groups": groups,
            "exif": exif.summary,
            "gps": exif.summary.gps,
            "xmp_history": xmp.history,
            "icc": icc,
            "thumbnail_compare": thumb.map(|t| json!({"main_aspect": t.main_aspect, "thumb_aspect": t.thumb_aspect, "diff": t.diff})),
        });
        Ok(ModuleResult { findings, visualizations, raw })
    }
}

impl AnalysisModule for ProvenanceModule {
    fn id(&self) -> &'static str {
        c2pa_reader::MODULE
    }
    fn name(&self) -> &'static str {
        "Content Credentials (C2PA)"
    }
    fn description(&self) -> &'static str {
        "Reads and validates C2PA manifests offline"
    }
    fn applies_to(&self, _: &ImageInput) -> bool {
        true
    }
    fn run(&self, input: &ImageInput, _ctx: &mut Context<'_>) -> Result<ModuleResult> {
        let c = c2pa_reader::read(input.bytes(), input.format());
        let findings = c2pa_reader::findings(&c);
        let groups: Vec<Group> = c2pa_reader::group(&c).into_iter().collect();
        let raw = json!({ "groups": groups, "summary": c, "manifest_store": c.json });
        Ok(ModuleResult { findings, visualizations: Vec::new(), raw })
    }
}

impl AnalysisModule for GeneratorHintsModule {
    fn id(&self) -> &'static str {
        generator::MODULE
    }
    fn name(&self) -> &'static str {
        "Generator hints"
    }
    fn description(&self) -> &'static str {
        "Text chunks and AI-generator parameters (Stable Diffusion, ComfyUI, Midjourney, ...)"
    }
    fn applies_to(&self, input: &ImageInput) -> bool {
        matches!(input.format(), forensica_core::FileFormat::Png | forensica_core::FileFormat::Jpeg)
    }
    fn run(&self, input: &ImageInput, _ctx: &mut Context<'_>) -> Result<ModuleResult> {
        let text = textchunks::read(input.bytes(), input.format());
        let findings = generator::from_text_chunks(&text);
        Ok(ModuleResult { findings, visualizations: Vec::new(), raw: json!({ "text_chunks": text }) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exif_read::testutil::*;
    use exif::Tag;

    fn run(m: &dyn AnalysisModule, bytes: Vec<u8>, name: &str) -> ModuleResult {
        let input = ImageInput::from_bytes(name, bytes);
        m.run(&input, &mut Context::simple()).unwrap()
    }

    #[test]
    fn metadata_module_end_to_end() {
        let main = jpeg(160, 120, 90);
        // Thumbnail of *different* content: inverted gradient.
        let inv = image::RgbImage::from_fn(32, 24, |x, y| image::Rgb([255 - (x * 8) as u8, 255 - (y * 10) as u8, 0]));
        let spec = ExifSpec::default()
            .ascii(Tag::Make, "Canon")
            .ascii(Tag::Model, "EOS")
            .ascii(Tag::Software, "GIMP 2.10")
            .ascii(Tag::DateTimeOriginal, "2020:01:01 10:00:00")
            .ascii(Tag::DateTime, "2020:06:01 10:00:00")
            .long(Tag::PixelXDimension, 4000)
            .long(Tag::PixelYDimension, 3000)
            .thumbnail(encode_jpeg(&inv, 90));
        let r = run(&MetadataModule, spec.to_jpeg(&main), "a.jpg");
        let codes: Vec<&str> = r.findings.iter().map(|f| f.code.as_str()).collect();
        for c in [
            "meta.editing_software",
            "meta.modified_after_capture",
            "meta.dimension_mismatch",
            "meta.thumbnail_mismatch",
        ] {
            assert!(codes.contains(&c), "missing {c} in {codes:?}");
        }
        assert_eq!(r.visualizations.len(), 1);
        assert!(r.raw["groups"].as_array().unwrap().len() >= 3);
    }

    #[test]
    fn matching_thumbnail_is_quiet() {
        let main = jpeg(160, 120, 90);
        let img = image::load_from_memory(&main).unwrap().to_rgb8();
        let small = image::imageops::resize(&img, 32, 24, FilterType::Triangle);
        let spec = ExifSpec::default().ascii(Tag::Make, "X").thumbnail(encode_jpeg(&small, 90));
        let r = run(&MetadataModule, spec.to_jpeg(&main), "a.jpg");
        assert!(!r.findings.iter().any(|f| f.code.starts_with("meta.thumbnail")), "{:?}", r.findings);
    }

    #[test]
    fn generator_module_png() {
        let png = textchunks::png_with_text(&[("tEXt", "parameters", "cat\nSteps: 20, Sampler: Euler")]);
        let r = run(&GeneratorHintsModule, png, "x.png");
        assert_eq!(r.findings[0].code, "gen.sd_parameters");
    }

    #[test]
    fn xmp_in_jpeg_via_module() {
        let mut app = b"http://ns.adobe.com/xap/1.0/\0".to_vec();
        app.extend_from_slice(crate::xmp::SAMPLE.as_bytes());
        let j = insert_app(&jpeg(32, 32, 90), 0xE1, &app);
        let r = run(&MetadataModule, j, "x.jpg");
        let codes: Vec<&str> = r.findings.iter().map(|f| f.code.as_str()).collect();
        assert!(codes.contains(&"meta.xmp_history"));
        assert!(codes.contains(&"meta.document_ancestors"));
        assert!(codes.contains(&"meta.editing_software"));
    }

    #[test]
    fn provenance_module_runs() {
        let r = run(&ProvenanceModule, jpeg(8, 8, 90), "x.jpg");
        assert_eq!(r.findings[0].code, "c2pa.absent");
    }
}
