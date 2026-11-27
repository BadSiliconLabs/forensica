//! Metadata anomaly rules. Each rule is a small function producing zero or more findings.

use forensica_core::{FileFormat, Finding, Severity};

use crate::exif_read::ExifSummary;
use crate::generator;
use crate::iptc::IptcData;
use crate::xmp::XmpData;

/// Module id used for findings.
pub const MODULE: &str = "metadata";

/// Comparison of the embedded EXIF thumbnail with the main image.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThumbCompare {
    /// Aspect ratio (w/h) of the main image.
    pub main_aspect: f32,
    /// Aspect ratio of the thumbnail.
    pub thumb_aspect: f32,
    /// Mean absolute difference (0..1) between both, at a common small size, on the overlapping region.
    pub diff: f32,
    /// Fraction of 32×32 cells that differ strongly (localised edits).
    pub local: f32,
}

/// Everything the rules look at.
#[derive(Debug, Clone, Default)]
pub struct MetaBundle {
    /// Container format.
    pub format: Option<FileFormat>,
    /// Decoded pixel size, if decodable.
    pub dims: Option<(u32, u32)>,
    /// Extension disagrees with magic bytes.
    pub extension_mismatch: bool,
    /// EXIF summary.
    pub exif: ExifSummary,
    /// XMP.
    pub xmp: XmpData,
    /// IPTC / IRB.
    pub iptc: IptcData,
    /// Thumbnail comparison.
    pub thumb: Option<ThumbCompare>,
    /// Current time in seconds since epoch (injected for testability).
    pub now_unix: u64,
}

/// Editing applications (strong processing indicator).
const EDITORS: &[&str] = &[
    "photoshop",
    "lightroom",
    "camera raw",
    "gimp",
    "affinity",
    "pixelmator",
    "paint.net",
    "paintshop",
    "corel",
    "snapseed",
    "picsart",
    "canva",
    "facetune",
    "meitu",
    "vsco",
    "photoscape",
    "acdsee",
    "capture one",
    "darktable",
    "rawtherapee",
    "luminar",
    "photopea",
    "fotor",
    "befunky",
    "krita",
    "photodirector",
    "polarr",
    "lightroom mobile",
    "airbrush",
    "remini",
    "youcam",
    "faceapp",
    "picmonkey",
    "photoshop express",
    "inpixio",
    "on1 photo",
];

/// Re-saving / viewing software (weak indicator: the file was written by a non-camera program).
const RESAVERS: &[&str] = &[
    "imagemagick",
    "graphicsmagick",
    "picasa",
    "windows photo",
    "microsoft windows",
    "photos",
    "preview",
    "shotwell",
    "digikam",
    "xnview",
    "irfanview",
    "libvips",
    "squoosh",
    "tinypng",
    "whatsapp",
    "telegram",
    "instagram",
    "facebook",
    "twitter",
    "pillow",
    "opencv",
    "exiftool",
];

/// Brands whose cameras normally write a MakerNote.
const MAKERNOTE_BRANDS: &[&str] = &[
    "canon",
    "nikon",
    "sony",
    "fujifilm",
    "olympus",
    "om digital",
    "panasonic",
    "pentax",
    "ricoh",
    "apple",
    "leica",
    "sigma",
    "casio",
];

/// Runs all rules.
pub fn evaluate(b: &MetaBundle) -> Vec<Finding> {
    let mut out = Vec::new();
    summary(b, &mut out);
    software(b, &mut out);
    dates(b, &mut out);
    thumbnail(b, &mut out);
    missing_metadata(b, &mut out);
    makernote(b, &mut out);
    impossible_values(b, &mut out);
    dimensions(b, &mut out);
    xmp_history(b, &mut out);
    xmp_misc(b, &mut out);
    container(b, &mut out);
    out
}

fn summary(b: &MetaBundle, out: &mut Vec<Finding>) {
    let e = &b.exif;
    if let (Some(make), Some(model)) = (&e.make, &e.model) {
        let mut f = Finding::new(
            MODULE,
            "meta.camera",
            Severity::Info,
            1.0,
            format!("Capture device declared: {make} {model}"),
        )
        .explain("EXIF names the device that (claims to have) captured the image.")
        .limits("EXIF can be edited or copied from another file.");
        if let Some(d) = &e.date_original {
            f = f.with("DateTimeOriginal", d);
        }
        if let Some(l) = &e.lens {
            f = f.with("Lens", l);
        }
        out.push(f);
    }
    if let Some(g) = e.gps {
        let mut f = Finding::new(MODULE, "meta.gps", Severity::Info, 1.0, "GPS coordinates present")
            .explain("The file embeds a location. It is shown as coordinates only; no map is loaded (offline).")
            .limits("GPS can be wrong (stale fix, indoor) or edited. Sharing this file shares the location.")
            .with("latitude", format!("{:.6}", g.lat))
            .with("longitude", format!("{:.6}", g.lon));
        if let Some(a) = g.alt {
            f = f.with("altitude_m", format!("{a:.1}"));
        }
        out.push(f);
    }
}

fn classify(s: &str) -> Option<(&'static str, bool)> {
    let l = s.to_ascii_lowercase();
    if let Some(e) = EDITORS.iter().find(|e| l.contains(*e)) {
        return Some((e, true));
    }
    RESAVERS.iter().find(|e| l.contains(*e)).map(|e| (*e, false))
}

fn software(b: &MetaBundle, out: &mut Vec<Finding>) {
    let mut sources: Vec<(&str, String)> = Vec::new();
    if let Some(s) = &b.exif.software {
        sources.push(("EXIF Software", s.clone()));
    }
    if let Some(s) = b.xmp.get("xmp:CreatorTool") {
        sources.push(("XMP CreatorTool", s.to_string()));
    }
    if let Some(s) = b.iptc.get("OriginatingProgram") {
        sources.push(("IPTC OriginatingProgram", s.to_string()));
    }
    for ev in &b.xmp.history {
        if !ev.software.is_empty() {
            sources.push(("XMP History softwareAgent", ev.software.clone()));
        }
    }
    let mut seen_edit = false;
    let mut seen_resave = false;
    for (field, value) in &sources {
        if let Some(f) = generator::from_software(field, value) {
            out.push(f);
            continue;
        }
        match classify(value) {
            Some((_, true)) if !seen_edit => {
                seen_edit = true;
                out.push(
                    Finding::new(MODULE, "meta.editing_software", Severity::Medium, 0.9, format!("Written by editing software: {value}"))
                        .explain("A metadata field names an image editor. The file was at least opened and saved by it.")
                        .limits("Saving in an editor does not imply content changes: photographers routinely use editors for exposure, colour or cropping. Software tags can also be forged or removed.")
                        .with("field", field)
                        .with("value", value),
                );
            }
            Some((_, false)) if !seen_resave => {
                seen_resave = true;
                out.push(
                    Finding::new(MODULE, "meta.resave_software", Severity::Low, 0.8, format!("Re-saved by non-camera software: {value}"))
                        .explain("A metadata field names a viewer, converter, library or platform that re-encoded the file.")
                        .limits("Re-encoding is very common (sharing apps, converters) and usually benign, but it destroys some forensic traces.")
                        .with("field", field)
                        .with("value", value),
                );
            }
            _ => {}
        }
    }
}

/// Parses EXIF (`YYYY:MM:DD HH:MM:SS`) or ISO-8601 dates to naive seconds (timezone ignored).
pub fn parse_date(s: &str) -> Option<i64> {
    let s = s.trim();
    let b = s.as_bytes();
    if b.len() < 10 {
        return None;
    }
    let num = |r: std::ops::Range<usize>| -> Option<i64> { s.get(r)?.parse::<i64>().ok() };
    let y = num(0..4)?;
    let mo = num(5..7)?;
    let d = num(8..10)?;
    let (h, mi, se) = if b.len() >= 19 { (num(11..13)?, num(14..16)?, num(17..19)?) } else { (0, 0, 0) };
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || se > 60 {
        return None;
    }
    let (y2, m2) = if mo <= 2 { (y - 1, mo + 9) } else { (y, mo - 3) };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * m2 + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 86_400 + h * 3600 + mi * 60 + se)
}

fn fmt_delta(secs: i64) -> String {
    let a = secs.unsigned_abs();
    if a >= 86_400 {
        format!("{:.1} days", a as f64 / 86_400.0)
    } else if a >= 3600 {
        format!("{:.1} hours", a as f64 / 3600.0)
    } else {
        format!("{a} s")
    }
}

fn dates(b: &MetaBundle, out: &mut Vec<Finding>) {
    let e = &b.exif;
    let modify = e.modify_date.as_deref().and_then(parse_date);
    let orig = e.date_original.as_deref().and_then(parse_date);
    let digi = e.date_digitized.as_deref().and_then(parse_date);
    for (name, raw) in
        [("ModifyDate", &e.modify_date), ("DateTimeOriginal", &e.date_original), ("CreateDate", &e.date_digitized)]
    {
        if let Some(r) = raw {
            let blank = r.trim_matches(|c: char| c == ' ' || c == ':' || c == '0').is_empty();
            if !blank && parse_date(r).is_none() {
                out.push(
                    Finding::new(MODULE, "meta.bad_date", Severity::Low, 0.9, format!("Malformed {name}"))
                        .explain("A date field does not follow the EXIF format or holds an impossible date.")
                        .limits("Some old or cheap devices write malformed dates.")
                        .with(name, r),
                );
            }
        }
    }
    if let (Some(m), Some(o)) = (modify, orig) {
        let d = m - o;
        if d > 60 {
            out.push(
                Finding::new(
                    MODULE,
                    "meta.modified_after_capture",
                    Severity::Medium,
                    0.85,
                    "File modified after capture",
                )
                .explain(format!(
                    "ModifyDate is {} later than DateTimeOriginal: software rewrote the file after it was taken.",
                    fmt_delta(d)
                ))
                .limits("Benign causes: in-camera rating/rotation, photo managers, RAW development, timezone fixes.")
                .with("ModifyDate", e.modify_date.clone().unwrap_or_default())
                .with("DateTimeOriginal", e.date_original.clone().unwrap_or_default()),
            );
        } else if d < -60 {
            out.push(
                Finding::new(MODULE, "meta.modified_before_capture", Severity::Medium, 0.8, "ModifyDate precedes capture date")
                    .explain(format!("ModifyDate is {} *earlier* than DateTimeOriginal, which cannot happen naturally.", fmt_delta(d)))
                    .limits("Can result from a wrong camera clock corrected later, or tools that rewrite only one of the fields.")
                    .with("ModifyDate", e.modify_date.clone().unwrap_or_default())
                    .with("DateTimeOriginal", e.date_original.clone().unwrap_or_default()),
            );
        }
    }
    if let (Some(o), Some(dg)) = (orig, digi) {
        if (o - dg).abs() > 2 {
            out.push(
                Finding::new(
                    MODULE,
                    "meta.digitized_mismatch",
                    Severity::Low,
                    0.8,
                    "Capture and digitization dates differ",
                )
                .explain(format!("DateTimeOriginal and DateTimeDigitized differ by {}.", fmt_delta(o - dg)))
                .limits("Normal for scanned film or prints; unusual for a digital camera.")
                .with("DateTimeOriginal", e.date_original.clone().unwrap_or_default())
                .with("DateTimeDigitized", e.date_digitized.clone().unwrap_or_default()),
            );
        }
    }
    let now = i64::try_from(b.now_unix).unwrap_or(i64::MAX);
    for (name, v) in [("DateTimeOriginal", orig), ("ModifyDate", modify)] {
        if let Some(t) = v {
            // Allow 2 days for timezones (dates are naive local time).
            if now > 0 && t > now + 2 * 86_400 {
                out.push(impossible(name, "date lies in the future", &fmt_secs(t)));
            } else if t < parse_date("1975:01:01 00:00:00").unwrap_or(0) {
                out.push(impossible(name, "date predates digital cameras", &fmt_secs(t)));
            }
        }
    }
    // XMP dates when EXIF is absent.
    if modify.is_none() {
        let c = b.xmp.get("xmp:CreateDate").and_then(parse_date);
        let m = b.xmp.get("xmp:ModifyDate").and_then(parse_date);
        if let (Some(c), Some(m)) = (c, m) {
            if m - c > 3600 {
                out.push(
                    Finding::new(MODULE, "meta.xmp_modified", Severity::Low, 0.8, "XMP records later modification")
                        .explain(format!("xmp:ModifyDate is {} after xmp:CreateDate.", fmt_delta(m - c)))
                        .limits("Timezone differences between the two fields are ignored; small offsets are normal.")
                        .with("xmp:CreateDate", b.xmp.get("xmp:CreateDate").unwrap_or_default())
                        .with("xmp:ModifyDate", b.xmp.get("xmp:ModifyDate").unwrap_or_default()),
                );
            }
        }
    }
}

fn fmt_secs(t: i64) -> String {
    forensica_core::report::format_unix(u64::try_from(t).unwrap_or(0))
}

fn impossible(field: &str, why: &str, value: &str) -> Finding {
    Finding::new(MODULE, "meta.impossible_value", Severity::Medium, 0.85, format!("Implausible {field}: {why}"))
        .explain("A metadata value is outside the range any real device produces.")
        .limits("Firmware bugs, unset camera clocks and buggy converters can also produce odd values.")
        .with(field, value)
}

fn thumbnail(b: &MetaBundle, out: &mut Vec<Finding>) {
    let Some(t) = b.thumb else { return };
    let aspect_off = (t.main_aspect - t.thumb_aspect).abs() / t.main_aspect.max(1e-3);
    let swapped = (t.main_aspect - 1.0 / t.thumb_aspect.max(1e-3)).abs() / t.main_aspect.max(1e-3) < 0.03;
    if aspect_off > 0.06 && !swapped {
        out.push(
            Finding::new(MODULE, "meta.thumbnail_aspect", Severity::Medium, 0.8, "Embedded thumbnail has a different shape")
                .explain("The EXIF thumbnail's aspect ratio differs from the main image: the image was probably cropped without the thumbnail being regenerated.")
                .limits("Some cameras store letterboxed thumbnails (e.g. 160×120 for 16:9 images).")
                .with("main_aspect", format!("{:.3}", t.main_aspect))
                .with("thumbnail_aspect", format!("{:.3}", t.thumb_aspect)),
        );
    } else if (t.diff > 0.10 || t.local >= 0.03) && !swapped {
        let conf = (0.5 + t.diff + t.local).min(0.95);
        out.push(
            Finding::new(MODULE, "meta.thumbnail_mismatch", Severity::High, conf, "Embedded thumbnail differs from the image")
                .explain("The EXIF thumbnail does not look like the main image. Editors that preserve EXIF without regenerating the thumbnail leave the *original* content visible in it.")
                .limits("Benign causes: in-camera effects applied only to the main image, strong tone edits, or letterboxing. Compare both visually.")
                .with("mean_abs_difference", format!("{:.3}", t.diff))
                .with("differing_area", format!("{:.1}%", t.local * 100.0)),
        );
    }
}

fn missing_metadata(b: &MetaBundle, out: &mut Vec<Finding>) {
    let camera_formats =
        matches!(b.format, Some(FileFormat::Jpeg | FileFormat::Heif | FileFormat::Tiff | FileFormat::Avif));
    if !b.exif.present {
        let (sev, note) = if camera_formats {
            (Severity::Low, "Camera files normally carry EXIF.")
        } else {
            (Severity::Info, "This format often carries no EXIF.")
        };
        out.push(
            Finding::new(MODULE, "meta.no_exif", sev, 0.9, "No camera metadata (EXIF)")
                .explain(format!("The file has no EXIF block. {note} Missing EXIF usually means the file was exported, screenshotted or passed through a platform that strips metadata."))
                .limits("Stripping metadata is standard privacy practice for messaging and social apps; it is not evidence of manipulation on its own."),
        );
        return;
    }
    if b.exif.make.is_none() && b.exif.model.is_none() {
        out.push(
            Finding::new(MODULE, "meta.no_camera_id", Severity::Low, 0.8, "EXIF present but no camera make/model")
                .explain(
                    "EXIF exists but does not name a device, typical of files written by software rather than cameras.",
                )
                .limits("Some apps and scanners omit Make/Model."),
        );
    } else if camera_formats && b.exif.date_original.is_none() && b.exif.exposure_time.is_none() {
        out.push(
            Finding::new(MODULE, "meta.sparse_exif", Severity::Low, 0.7, "Camera EXIF is incomplete")
                .explain("The device is named but capture fields (date, exposure) are missing, suggesting the EXIF was rewritten or trimmed.")
                .limits("Some devices write minimal EXIF."),
        );
    }
}

fn makernote(b: &MetaBundle, out: &mut Vec<Finding>) {
    let Some(make) = b.exif.make.as_deref().map(str::to_ascii_lowercase) else { return };
    let brand = MAKERNOTE_BRANDS.iter().any(|m| make.contains(m));
    match &b.exif.makernote {
        None if brand && b.exif.date_original.is_some() => out.push(
            Finding::new(MODULE, "meta.makernote_missing", Severity::Low, 0.6, "Manufacturer MakerNote missing")
                .explain("Cameras from this brand normally embed a proprietary MakerNote. Many editors and converters drop it.")
                .limits("Some firmware or phone modes omit it; some privacy tools remove it deliberately.")
                .with("Make", b.exif.make.clone().unwrap_or_default()),
        ),
        Some(mn) if mn.consistent_with_make == Some(false) => out.push(
            Finding::new(MODULE, "meta.makernote_inconsistent", Severity::Medium, 0.8, "MakerNote format does not match the camera make")
                .explain("The MakerNote's vendor signature belongs to a different manufacturer than the EXIF Make, which suggests EXIF was transplanted or edited.")
                .limits("Rebadged cameras share firmware with other brands (e.g. Leica/Panasonic).")
                .with("Make", b.exif.make.clone().unwrap_or_default())
                .with("MakerNote", &mn.vendor),
        ),
        _ => {}
    }
}

fn impossible_values(b: &MetaBundle, out: &mut Vec<Finding>) {
    let e = &b.exif;
    if let Some(f) = e.f_number {
        if !f.is_finite() || !(0.5..=128.0).contains(&f) {
            out.push(impossible("FNumber", "outside physical lens range", &format!("{f}")));
        }
    }
    if let Some(t) = e.exposure_time {
        if !t.is_finite() || t <= 0.0 || t > 7200.0 {
            out.push(impossible("ExposureTime", "non-positive or absurdly long", &format!("{t}")));
        }
    }
    if let Some(i) = e.iso {
        if i > 6_400_000 {
            out.push(impossible("ISO", "beyond any sensor", &i.to_string()));
        }
    }
    if let Some(fl) = e.focal_length {
        if !fl.is_finite() || fl < 0.0 || fl > 6000.0 {
            out.push(impossible("FocalLength", "outside lens range", &format!("{fl}")));
        }
    }
    if e.gps_invalid {
        let v = e.gps.map(|g| format!("{:.5}, {:.5}", g.lat, g.lon)).unwrap_or_else(|| "unparsable".into());
        out.push(impossible("GPS", "coordinates out of range or malformed", &v));
    }
    if let Some(o) = e.orientation {
        if !(1..=8).contains(&o) {
            out.push(impossible("Orientation", "not one of the 8 defined values", &o.to_string()));
        }
    }
}

fn dimensions(b: &MetaBundle, out: &mut Vec<Finding>) {
    let (Some((w, h)), Some(px), Some(py)) = (b.dims, b.exif.pixel_x, b.exif.pixel_y) else { return };
    if px == 0 || py == 0 {
        return;
    }
    let same = (px == w && py == h) || (px == h && py == w);
    if !same {
        let smaller = u64::from(w) * u64::from(h) < u64::from(px) * u64::from(py);
        out.push(
            Finding::new(MODULE, "meta.dimension_mismatch", Severity::Medium, 0.75, "Pixel size differs from EXIF")
                .explain(format!(
                    "EXIF records {px}×{py} but the image is {w}×{h}: it was {} after the EXIF was written.",
                    if smaller { "resized or cropped" } else { "enlarged or re-rendered" }
                ))
                .limits("Many editors and platforms copy EXIF verbatim while resizing; this indicates re-processing, not necessarily content changes.")
                .with("exif", format!("{px}x{py}"))
                .with("actual", format!("{w}x{h}")),
        );
    }
}

fn xmp_history(b: &MetaBundle, out: &mut Vec<Finding>) {
    if b.xmp.history.is_empty() {
        return;
    }
    let edits: Vec<_> = b
        .xmp
        .history
        .iter()
        .filter(|e| {
            matches!(
                e.action.as_str(),
                "saved" | "converted" | "derived" | "edited" | "produced" | "cropped" | "filtered" | "managed"
            )
        })
        .collect();
    let mut f = if edits.is_empty() {
        Finding::new(MODULE, "meta.xmp_history", Severity::Info, 1.0, "XMP edit history present")
    } else {
        Finding::new(
            MODULE,
            "meta.xmp_history",
            Severity::Medium,
            0.9,
            format!("XMP history records {} editing step(s)", edits.len()),
        )
    }
    .explain("Adobe-compatible software keeps a log of actions (created, saved, converted, derived) in xmpMM:History.")
    .limits("History is optional, may be incomplete, and is easily removed or edited.");
    for (i, ev) in b.xmp.history.iter().enumerate().take(20) {
        let mut v = ev.action.clone();
        if !ev.software.is_empty() {
            v.push_str(&format!(" · {}", ev.software));
        }
        if !ev.when.is_empty() {
            v.push_str(&format!(" · {}", ev.when));
        }
        if !ev.parameters.is_empty() {
            v.push_str(&format!(" · {}", ev.parameters));
        }
        f = f.with(format!("step {}", i + 1), v);
    }
    out.push(f);
}

fn xmp_misc(b: &MetaBundle, out: &mut Vec<Finding>) {
    let x = &b.xmp;
    let ancestors = x.count_prefix("photoshop:DocumentAncestors");
    if ancestors > 0 {
        out.push(
            Finding::new(MODULE, "meta.document_ancestors", Severity::Medium, 0.85, format!("{ancestors} other document(s) were placed into this file"))
                .explain("photoshop:DocumentAncestors lists documents that were pasted or placed as layers, a typical trace of compositing.")
                .limits("Also produced by harmless operations such as adding a logo or a watermark, or by templates.")
                .with("count", ancestors),
        );
    }
    if x.count_prefix("xmpMM:DerivedFrom") > 0 {
        out.push(
            Finding::new(MODULE, "meta.derived_from", Severity::Low, 0.85, "Derived from another document")
                .explain("xmpMM:DerivedFrom references a source document; this file is a derivative (export, conversion or edit).")
                .limits("Every export from Lightroom/Photoshop records this; it does not imply content changes."),
        );
    }
    let crs = x.count_prefix("crs:");
    if crs > 0 {
        let retouch = x.properties.iter().any(|(k, _)| {
            k.starts_with("crs:RetouchAreas")
                || k.starts_with("crs:RetouchInfo")
                || k.starts_with("crs:MaskGroupBasedCorrections")
                || k.starts_with("crs:PaintBasedCorrections")
                || k.starts_with("crs:CircularGradientBasedCorrections")
        });
        let crop = x.get("crs:HasCrop") == Some("True");
        let (sev, title) = if retouch {
            (Severity::Medium, "Local retouching recorded by Camera Raw / Lightroom")
        } else {
            (Severity::Low, "Developed in Camera Raw / Lightroom")
        };
        let mut f = Finding::new(MODULE, "meta.raw_processing", sev, 0.85, title)
            .explain("Adobe Camera Raw settings (crs:*) are embedded, recording the development applied to the image.")
            .limits("Global development (exposure, colour) is routine photography; local healing/masking edits change specific regions.")
            .with("crs_properties", crs);
        if crop {
            f = f.with("cropped", "yes");
        }
        out.push(f);
    }
    for (k, v) in &x.properties {
        if k.ends_with("Iptc4xmpExt:DigitalSourceType") || k.ends_with("DigitalSourceType") {
            if let Some(f) = generator::digital_source_type(k, v) {
                out.push(f);
            }
        }
    }
    if x.parse_errors > 0 {
        out.push(
            Finding::new(MODULE, "meta.xmp_malformed", Severity::Low, 0.7, "Malformed XMP packet")
                .explain("An XMP packet could not be parsed as XML, which suggests careless manual or tool editing.")
                .limits("Truncated downloads and buggy writers cause this too."),
        );
    }
}

fn container(b: &MetaBundle, out: &mut Vec<Finding>) {
    if b.extension_mismatch {
        out.push(
            Finding::new(
                MODULE,
                "meta.extension_mismatch",
                Severity::Low,
                0.95,
                "File extension does not match content",
            )
            .explain(
                "The file's magic bytes indicate a different format than its extension: it was renamed or converted.",
            )
            .limits("Common with downloads and messaging apps; harmless by itself."),
        );
    }
    let lossless = matches!(b.format, Some(FileFormat::Png | FileFormat::Gif | FileFormat::Bmp | FileFormat::Webp));
    if lossless && b.exif.make.is_some() {
        out.push(
            Finding::new(MODULE, "meta.camera_exif_in_export", Severity::Low, 0.7, "Camera EXIF inside a non-camera format")
                .explain("Cameras do not write PNG/GIF/BMP/WebP; camera EXIF here means the file was converted by software that copied the metadata.")
                .limits("Conversion is often harmless (e.g. for web publishing)."),
        );
    }
    if b.iptc.photoshop_irb && b.iptc.resources.contains(&0x0406) {
        out.push(
            Finding::new(
                MODULE,
                "meta.photoshop_save",
                Severity::Low,
                0.85,
                "Saved by Adobe Photoshop (JPEG quality resource)",
            )
            .explain("Photoshop's 'Save As JPEG' writes an image resource (0x0406) recording the chosen quality.")
            .limits("Indicates the save path only, not what was changed."),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xmp::HistoryEvent;

    fn codes(f: &[Finding]) -> Vec<&str> {
        f.iter().map(|x| x.code.as_str()).collect()
    }

    fn camera() -> ExifSummary {
        ExifSummary {
            present: true,
            make: Some("Canon".into()),
            model: Some("EOS".into()),
            date_original: Some("2020:01:01 10:00:00".into()),
            modify_date: Some("2020:01:01 10:00:00".into()),
            exposure_time: Some(0.01),
            ..Default::default()
        }
    }

    fn bundle(exif: ExifSummary) -> MetaBundle {
        MetaBundle { format: Some(FileFormat::Jpeg), exif, now_unix: 1_750_000_000, ..Default::default() }
    }

    #[test]
    fn parse_dates() {
        assert_eq!(parse_date("1970:01:01 00:00:00"), Some(0));
        assert_eq!(parse_date("2000-02-29T00:00:00+01:00"), Some(951_782_400));
        assert_eq!(parse_date("2020:13:01 00:00:00"), None);
        assert_eq!(parse_date("garbage"), None);
    }

    #[test]
    fn clean_camera_file_has_only_info_and_makernote() {
        let f = evaluate(&bundle(camera()));
        assert!(codes(&f).contains(&"meta.camera"));
        assert!(f.iter().all(|x| x.severity <= Severity::Low), "{:?}", codes(&f));
    }

    #[test]
    fn editing_software_and_dates() {
        let mut e = camera();
        e.software = Some("Adobe Photoshop 25.0 (Windows)".into());
        e.modify_date = Some("2020:03:01 10:00:00".into());
        let f = evaluate(&bundle(e));
        let c = codes(&f);
        assert!(c.contains(&"meta.editing_software"));
        assert!(c.contains(&"meta.modified_after_capture"));
    }

    #[test]
    fn modify_before_capture_and_future() {
        let mut e = camera();
        e.modify_date = Some("2019:01:01 10:00:00".into());
        e.date_original = Some("2099:01:01 10:00:00".into());
        let c = evaluate(&bundle(e));
        let c = codes(&c);
        assert!(c.contains(&"meta.modified_before_capture"));
        assert!(c.contains(&"meta.impossible_value"));
    }

    #[test]
    fn missing_exif_rules() {
        let f = evaluate(&bundle(ExifSummary::default()));
        assert_eq!(f[0].code, "meta.no_exif");
        assert_eq!(f[0].severity, Severity::Low);
        let mut b = bundle(ExifSummary::default());
        b.format = Some(FileFormat::Png);
        assert_eq!(evaluate(&b)[0].severity, Severity::Info);
        let e = ExifSummary { present: true, ..Default::default() };
        assert!(codes(&evaluate(&bundle(e))).contains(&"meta.no_camera_id"));
    }

    #[test]
    fn impossible_values_and_dimensions() {
        let mut e = camera();
        e.f_number = Some(0.1);
        e.exposure_time = Some(-1.0);
        e.gps_invalid = true;
        e.pixel_x = Some(4000);
        e.pixel_y = Some(3000);
        let mut b = bundle(e);
        b.dims = Some((1200, 900));
        let f = evaluate(&b);
        assert_eq!(f.iter().filter(|x| x.code == "meta.impossible_value").count(), 3);
        assert!(codes(&f).contains(&"meta.dimension_mismatch"));
        b.dims = Some((3000, 4000));
        assert!(!codes(&evaluate(&b)).contains(&"meta.dimension_mismatch"));
    }

    #[test]
    fn thumbnail_rules() {
        let mut b = bundle(camera());
        b.thumb = Some(ThumbCompare { main_aspect: 1.5, thumb_aspect: 1.5, diff: 0.3, local: 0.2 });
        assert!(codes(&evaluate(&b)).contains(&"meta.thumbnail_mismatch"));
        b.thumb = Some(ThumbCompare { main_aspect: 1.0, thumb_aspect: 1.5, diff: 0.0, local: 0.0 });
        assert!(codes(&evaluate(&b)).contains(&"meta.thumbnail_aspect"));
        b.thumb = Some(ThumbCompare { main_aspect: 1.5, thumb_aspect: 1.5, diff: 0.02, local: 0.005 });
        let c = evaluate(&b);
        assert!(!codes(&c).iter().any(|c| c.starts_with("meta.thumbnail")));
    }

    #[test]
    fn xmp_rules() {
        let mut b = bundle(camera());
        b.xmp.history = vec![
            HistoryEvent { action: "created".into(), ..Default::default() },
            HistoryEvent { action: "saved".into(), software: "Adobe Photoshop".into(), ..Default::default() },
        ];
        b.xmp.properties = vec![
            ("photoshop:DocumentAncestors[1]".into(), "x".into()),
            ("crs:RetouchAreas[1]/crs:SpotType".into(), "heal".into()),
            (
                "Iptc4xmpExt:DigitalSourceType".into(),
                "http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia".into(),
            ),
        ];
        let f = evaluate(&b);
        let c = codes(&f);
        assert!(c.contains(&"meta.xmp_history"));
        assert!(c.contains(&"meta.document_ancestors"));
        assert!(c.contains(&"meta.raw_processing"));
        assert!(c.contains(&"gen.digital_source_type"));
        assert!(c.contains(&"meta.editing_software"));
    }

    #[test]
    fn makernote_rules() {
        let mut e = camera();
        e.makernote = Some(crate::makernote::MakerNoteInfo {
            vendor: "Nikon type 3".into(),
            size: 10,
            entries: Some(1),
            consistent_with_make: Some(false),
        });
        assert!(codes(&evaluate(&bundle(e))).contains(&"meta.makernote_inconsistent"));
        assert!(codes(&evaluate(&bundle(camera()))).contains(&"meta.makernote_missing"));
    }
}
