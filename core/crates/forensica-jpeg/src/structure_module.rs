//! Analysis module: JPEG structure, quantisation fingerprints and trailing data.

use std::io::Cursor;

use serde_json::json;

use crate::quant::{estimate, fingerprint};
use crate::structure::{parse, JpegInfo};
use forensica_core::{AnalysisModule, Context, FileFormat, Finding, ImageInput, ModuleResult, Result, Severity};

const MODULE: &str = "jpeg";

/// JPEG structure module.
pub struct JpegStructureModule;

fn make(bytes: &[u8]) -> Option<String> {
    let exif = exif::Reader::new().read_from_container(&mut Cursor::new(bytes)).ok()?;
    let f = exif.get_field(exif::Tag::Make, exif::In::PRIMARY)?;
    Some(f.display_value().to_string().trim_matches('"').trim().to_string()).filter(|s| !s.is_empty())
}

fn table_str(t: &[u16; 64]) -> String {
    t.chunks(8).map(|r| r.iter().map(u16::to_string).collect::<Vec<_>>().join(" ")).collect::<Vec<_>>().join(" / ")
}

fn groups(info: &JpegInfo) -> serde_json::Value {
    let mut entries = Vec::new();
    let mut push = |k: &str, v: String| entries.push(json!({"key": k, "value": v}));
    if let Some(f) = &info.frame {
        push("Process", f.process().to_string());
        push("Size", format!("{} × {}", f.width, f.height));
        push("Precision", format!("{} bit", f.precision));
        push("Components", f.components.len().to_string());
        push("Chroma subsampling", f.subsampling());
    }
    push("Marker sequence", info.markers.join(" "));
    push("Quantisation tables", info.quant_tables.len().to_string());
    push(
        "Huffman tables",
        format!(
            "{} ({})",
            info.huff_tables.len(),
            if info.standard_huffman() { "standard Annex K" } else { "optimised / custom" }
        ),
    );
    push("Scans", info.scans.len().to_string());
    if let Some(r) = info.scans.first().map(|s| s.restart_interval).filter(|r| *r > 0) {
        push("Restart interval", r.to_string());
    }
    for a in &info.apps {
        let name = crate::structure::marker_name(a.marker);
        push(&format!("{name} @{}", a.offset), format!("{} ({} bytes)", a.signature, a.length));
    }
    for t in &info.quant_tables {
        let e = estimate(&t.values, t.id != 0);
        push(
            &format!("DQT {}", t.id),
            format!(
                "≈ IJG quality {}{} · {}",
                e.quality,
                if e.exact_ijg { " (exact)" } else { "" },
                table_str(&t.values)
            ),
        );
    }
    if let Some(t) = &info.trailing {
        push("Data after EOI", format!("{} bytes at offset {} · {}", t.length, t.offset, t.kind));
    }
    json!([{ "name": "JPEG structure", "entries": entries }])
}

fn findings(info: &JpegInfo, bytes: &[u8]) -> Vec<Finding> {
    let mut out = Vec::new();
    let has = |sig: &str| info.apps.iter().any(|a| a.signature.starts_with(sig));
    let mpf = has("MPF");

    // Trailing data / embedded payloads.
    if let Some(t) = &info.trailing {
        let archive =
            t.kind.contains("archive") || t.kind.contains("ZIP") || t.kind.contains("PDF") || t.kind.contains("PNG");
        let f = if t.kind.starts_with("padding") {
            Finding::new(MODULE, "jpeg.trailing_padding", Severity::Info, 0.9, "Padding after end of image")
                .explain("Zero or 0xFF bytes follow the end-of-image marker; some cameras and encoders pad files.")
                .limits("Harmless in itself.")
        } else if mpf && t.kind == "JPEG image" {
            Finding::new(MODULE, "jpeg.mpf_images", Severity::Info, 0.95, format!("{} additional image(s) stored after the main one (MPF)", t.embedded_jpegs))
                .explain("The Multi-Picture Format stores extra images (previews, depth maps, gain maps) after the main JPEG. Phones commonly do this.")
                .limits("If the main image was edited but the secondary images were kept, they may show the original content; inspect them with an external tool.")
        } else if archive {
            Finding::new(MODULE, "jpeg.hidden_payload", Severity::High, 0.95, format!("Hidden {} appended after the image", t.kind))
                .explain("A complete file of another type is concatenated after the JPEG end marker. Viewers ignore it, so it is invisible.")
                .limits("This indicates deliberate packing (polyglot/steganography) rather than photo manipulation.")
        } else {
            Finding::new(MODULE, "jpeg.trailing_data", Severity::Medium, 0.9, format!("{} bytes of data after end of image", t.length))
                .explain(format!("Data follows the end-of-image marker ({}). Editors normally write nothing there; this can be a leftover of the original file, an appended image, or hidden data.", t.kind))
                .limits("Some phones/cameras append proprietary data (e.g. Samsung trailers).")
        };
        out.push(f.with("offset", t.offset).with("length", t.length).with("kind", &t.kind));
    }
    if info.layout.eoi_end.is_none() {
        out.push(
            Finding::new(MODULE, "jpeg.truncated", Severity::Low, 0.9, "File is truncated or has no end marker")
                .explain("The JPEG stream ends without an EOI marker or with malformed segments.")
                .limits("Often an incomplete download or copy."),
        );
    }

    // Software signatures in APP segments.
    if has("JFIF") && has("Exif") {
        out.push(
            Finding::new(MODULE, "jpeg.jfif_and_exif", Severity::Low, 0.6, "Both JFIF and EXIF headers present")
                .explain("Cameras write EXIF only; a JFIF header added alongside usually comes from software that re-saved the file.")
                .limits("Some cameras and phones write both."),
        );
    }
    if has("Ducky") {
        out.push(
            Finding::new(MODULE, "jpeg.ducky", Severity::Low, 0.9, "Adobe 'Save for Web' signature (APP12 Ducky)")
                .explain("The APP12 'Ducky' segment is written by Photoshop's Save for Web / Export.")
                .limits("Indicates the save path only."),
        );
    }
    if has("Adobe") {
        out.push(
            Finding::new(MODULE, "jpeg.adobe_app14", Severity::Low, 0.7, "Adobe APP14 segment")
                .explain("APP14 'Adobe' is written by Adobe software and some other encoders to describe the colour transform.")
                .limits("Also written by a few non-Adobe libraries."),
        );
    }
    let frame_pos = info.markers.iter().position(|m| m.starts_with("SOF"));
    let late_app = frame_pos.is_some_and(|fp| info.markers.iter().skip(fp).any(|m| m.starts_with("APP")));
    if late_app {
        out.push(
            Finding::new(MODULE, "jpeg.marker_order", Severity::Low, 0.7, "Unusual marker order (APP segment after frame header)")
                .explain("Metadata segments appear after the frame header, which camera firmware never does; typical of tools that inject metadata.")
                .limits("Legal per the standard; some libraries do it."),
        );
    }

    // Encoding parameters.
    if let Some(f) = &info.frame {
        if f.progressive() {
            out.push(
                Finding::new(MODULE, "jpeg.progressive", Severity::Low, 0.6, "Progressive JPEG")
                    .explain("Cameras write baseline JPEGs; progressive encoding is typical of web export and re-compression by software or platforms.")
                    .limits("Some phones and cameras can save progressive files."),
            );
        }
    }
    let (l, c) = info.frame_tables();
    if let Some(lt) = l {
        let e = estimate(&lt, false);
        let fp = fingerprint(Some(&lt), c.as_ref());
        let mut f = Finding::new(MODULE, "jpeg.quality", Severity::Info, 0.9, format!("Estimated JPEG quality ≈ {}", e.quality))
            .explain(match fp {
                Some(fp) => format!("Quantisation tables match: {}. {}", fp.name, fp.note),
                None => "Quantisation tables are not the standard IJG/libjpeg tables: typical of camera firmware, Photoshop and other custom encoders.".into(),
            })
            .limits("Quality is expressed on the IJG scale; other encoders' scales differ. The built-in fingerprint table does not include camera-specific tables yet.")
            .with("exact_ijg", e.exact_ijg)
            .with("mean_table_error", format!("{:.2}", e.error));
        if let Some(fp) = fp {
            f = f.with("fingerprint", fp.name);
        }
        out.push(f);
        if e.quality < 70 {
            out.push(
                Finding::new(
                    MODULE,
                    "jpeg.low_quality",
                    Severity::Info,
                    0.9,
                    "Strong compression limits pixel analysis",
                )
                .explain(
                    "At this quality, compression artefacts mask fine traces; ELA and noise maps become less reliable.",
                )
                .limits("Not an indicator of manipulation."),
            );
        }
        if let Some(mk) = make(bytes) {
            if fp.is_some_and(|f| f.name.starts_with("IJG")) {
                out.push(
                    Finding::new(MODULE, "jpeg.camera_with_software_tables", Severity::Low, 0.55, "Camera metadata but software-style quantisation tables")
                        .explain(format!("EXIF names '{mk}', but the quantisation tables are exactly the IJG/libjpeg ones, which suggests the file was re-encoded by software after capture."))
                        .limits("Some phones and inexpensive cameras use libjpeg with standard tables.")
                        .with("Make", mk),
                );
            }
        }
    }
    if info.quant_tables.len() > 4 && info.frame.as_ref().is_some_and(|f| !f.progressive()) {
        out.push(
            Finding::new(MODULE, "jpeg.redefined_tables", Severity::Low, 0.6, "Quantisation tables redefined")
                .explain("A baseline file defines more quantisation tables than it needs, which some editing tools leave behind.")
                .limits("Allowed by the standard.")
                .with("tables", info.quant_tables.len()),
        );
    }
    out
}

impl AnalysisModule for JpegStructureModule {
    fn id(&self) -> &'static str {
        MODULE
    }
    fn name(&self) -> &'static str {
        "JPEG structure"
    }
    fn description(&self) -> &'static str {
        "Markers, quantisation tables, encoder fingerprints, trailing data"
    }
    fn applies_to(&self, input: &ImageInput) -> bool {
        input.format() == FileFormat::Jpeg
    }
    fn run(&self, input: &ImageInput, _ctx: &mut Context<'_>) -> Result<ModuleResult> {
        let info = parse(input.bytes())?;
        let findings = findings(&info, input.bytes());
        let (l, c) = info.frame_tables();
        let raw = json!({
            "groups": groups(&info),
            "markers": info.markers,
            "luma_quality": l.map(|t| estimate(&t, false)),
            "chroma_quality": c.map(|t| estimate(&t, true)),
            "trailing": info.trailing,
        });
        Ok(ModuleResult { findings, visualizations: Vec::new(), raw })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil;

    fn run(bytes: Vec<u8>) -> ModuleResult {
        JpegStructureModule.run(&ImageInput::from_bytes("x.jpg", bytes), &mut Context::simple()).unwrap()
    }

    #[test]
    fn plain_encoder_output() {
        let r = run(testutil::jpeg(64, 64, 85));
        let q = r.findings.iter().find(|f| f.code == "jpeg.quality").unwrap();
        assert!(q.title.contains("85"), "{}", q.title);
        assert!(!r.findings.iter().any(|f| f.severity >= Severity::Medium));
    }

    #[test]
    fn appended_zip_is_high() {
        let mut j = testutil::jpeg(32, 32, 85);
        j.extend_from_slice(b"PK\x03\x04secret");
        let r = run(j);
        assert!(r.findings.iter().any(|f| f.code == "jpeg.hidden_payload" && f.severity == Severity::High));
    }

    #[test]
    fn appended_garbage_is_medium_and_low_quality_noted() {
        let mut j = testutil::jpeg(32, 32, 40);
        j.extend_from_slice(&[1, 2, 3, 4, 5, 200, 100]);
        let r = run(j);
        assert!(r.findings.iter().any(|f| f.code == "jpeg.trailing_data"));
        assert!(r.findings.iter().any(|f| f.code == "jpeg.low_quality"));
    }
}
