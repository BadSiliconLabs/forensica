//! Synthetic demonstration images, one per detector. All content is generated
//! procedurally (no third-party photos), so the files are free to share.

use std::path::Path;

use exif::experimental::Writer;
use exif::{Field, In, Tag, Value};
use image::RgbImage;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 40) as f32 / 16_777_216.0
    }
    fn gauss(&mut self) -> f32 {
        self.next() + self.next() + self.next() + self.next() - 2.0
    }
}

fn hash(ix: i64, iy: i64, seed: u64) -> f32 {
    let mut z =
        (ix as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (iy as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F) ^ seed;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    ((z ^ (z >> 31)) >> 40) as f32 / 16_777_216.0
}

/// Fractal value noise in 0..1 (non-periodic natural-looking texture).
fn value_noise(x: f32, y: f32, seed: u64) -> f32 {
    let mut total = 0.0;
    let mut amp = 0.5;
    let mut scale = 48.0;
    for o in 0..4 {
        let (fx, fy) = (x / scale, y / scale);
        let (ix, iy) = (fx.floor() as i64, fy.floor() as i64);
        let (tx, ty) = (fx - fx.floor(), fy - fy.floor());
        let (sx, sy) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
        let s = seed + o;
        let a = hash(ix, iy, s) + (hash(ix + 1, iy, s) - hash(ix, iy, s)) * sx;
        let b = hash(ix, iy + 1, s) + (hash(ix + 1, iy + 1, s) - hash(ix, iy + 1, s)) * sx;
        total += amp * (a + (b - a) * sy);
        amp *= 0.5;
        scale *= 0.5;
    }
    total / 0.9375
}

/// A procedural "landscape" with sensor-like noise and Bayer demosaicing.
fn scene(w: u32, h: u32, seed: u64) -> RgbImage {
    let mut r = Rng(seed);
    let horizon = h as f32 * 0.55;
    let mut img = RgbImage::from_fn(w, h, |x, y| {
        let (xf, yf) = (x as f32, y as f32);
        let mut c = if yf < horizon {
            let t = yf / horizon;
            let cloud = 40.0 * (value_noise(xf * 0.5, yf, seed + 99) - 0.5).max(0.0);
            [90.0 + 80.0 * t + cloud, 140.0 + 60.0 * t + cloud, 220.0 - 20.0 * t + cloud]
        } else {
            let t = (yf - horizon) / (h as f32 - horizon);
            let tex = 70.0 * (value_noise(xf, yf, seed) - 0.5);
            [70.0 + 40.0 * t + tex, 120.0 - 30.0 * t + tex, 50.0 + tex * 0.5]
        };
        // Sun and a "house".
        let (sx, sy) = (w as f32 * 0.78, h as f32 * 0.2);
        if (xf - sx).powi(2) + (yf - sy).powi(2) < (h as f32 * 0.06).powi(2) {
            c = [250.0, 230.0, 150.0];
        }
        let (hx0, hx1, hy0, hy1) =
            (w as f32 * 0.2, w as f32 * 0.38, horizon - h as f32 * 0.18, horizon + h as f32 * 0.05);
        if xf > hx0 && xf < hx1 && yf > hy0 && yf < hy1 {
            let brick =
                if (yf as u32 / 9).is_multiple_of(2) ^ (xf as u32 / 17).is_multiple_of(2) { 12.0 } else { -8.0 };
            c = [170.0 + brick, 80.0 + brick * 0.5, 60.0];
        }
        image::Rgb(c.map(|v| v.clamp(0.0, 255.0) as u8))
    });
    for p in img.pixels_mut() {
        let n = r.gauss() * 6.0;
        for c in &mut p.0 {
            *c = (f32::from(*c) + n).clamp(0.0, 255.0) as u8;
        }
    }
    // Bayer RGGB + bilinear green interpolation.
    let src = img.clone();
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            if (x + y) % 2 == 0 {
                let g = (u32::from(src.get_pixel(x - 1, y).0[1])
                    + u32::from(src.get_pixel(x + 1, y).0[1])
                    + u32::from(src.get_pixel(x, y - 1).0[1])
                    + u32::from(src.get_pixel(x, y + 1).0[1]))
                    / 4;
                img.get_pixel_mut(x, y).0[1] = g as u8;
            }
        }
    }
    img
}

fn jpeg(img: &RgbImage, q: u8) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, q).encode_image(img).map_err(|e| e.to_string())?;
    Ok(out)
}

fn png(img: &RgbImage) -> Result<Vec<u8>, String> {
    let mut buf = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img.clone())
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(buf.into_inner())
}

fn decode(j: &[u8]) -> Result<RgbImage, String> {
    image::load_from_memory(j).map(|i| i.to_rgb8()).map_err(|e| e.to_string())
}

fn ascii(tag: Tag, v: &str) -> Field {
    Field { tag, ifd_num: In::PRIMARY, value: Value::Ascii(vec![v.as_bytes().to_vec()]) }
}

fn with_app(jpeg: &[u8], marker: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = jpeg[..2].to_vec();
    out.extend_from_slice(&[0xFF, marker]);
    out.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
    out.extend_from_slice(payload);
    out.extend_from_slice(&jpeg[2..]);
    out
}

fn with_exif(jpeg: &[u8], fields: &[Field], thumb: Option<&[u8]>) -> Result<Vec<u8>, String> {
    let mut w = Writer::new();
    for f in fields {
        w.push_field(f);
    }
    if let Some(t) = thumb {
        w.set_jpeg(t, In::THUMBNAIL);
    }
    let mut tiff = std::io::Cursor::new(Vec::new());
    w.write(&mut tiff, false).map_err(|e| e.to_string())?;
    let mut seg = b"Exif\0\0".to_vec();
    seg.extend_from_slice(&tiff.into_inner());
    Ok(with_app(jpeg, 0xE1, &seg))
}

fn camera_fields(software: Option<&str>, modify: &str) -> Vec<Field> {
    let mut f = vec![
        ascii(Tag::Make, "DemoCam"),
        ascii(Tag::Model, "DC-1 (synthetic)"),
        ascii(Tag::DateTimeOriginal, "2024:06:01 10:15:00"),
        ascii(Tag::DateTime, modify),
        Field { tag: Tag::ExposureTime, ifd_num: In::PRIMARY, value: Value::Rational(vec![(1, 250).into()]) },
        Field { tag: Tag::FNumber, ifd_num: In::PRIMARY, value: Value::Rational(vec![(56, 10).into()]) },
    ];
    if let Some(s) = software {
        f.push(ascii(Tag::Software, s));
    }
    f
}

fn thumb_of(img: &RgbImage) -> Result<Vec<u8>, String> {
    jpeg(&image::imageops::resize(img, 160, 120, image::imageops::FilterType::Triangle), 85)
}

fn png_text(png_bytes: &[u8], key: &str, text: &str) -> Vec<u8> {
    let (head, tail) = png_bytes.split_at(png_bytes.len() - 12);
    let mut out = head.to_vec();
    let mut data = key.as_bytes().to_vec();
    data.push(0);
    data.extend_from_slice(text.as_bytes());
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut c = b"tEXt".to_vec();
    c.extend_from_slice(&data);
    out.extend_from_slice(&c);
    out.extend_from_slice(&forensica_core::container::crc32(&c).to_be_bytes());
    out.extend_from_slice(tail);
    out
}

/// Writes a 24-megapixel camera-like JPEG for benchmarking.
pub fn write_big(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let img = scene(6000, 4000, 5);
    let j = with_exif(&jpeg(&img, 92)?, &camera_fields(None, "2024:06:01 10:15:00"), None)?;
    std::fs::write(dir.join("24mp.jpg"), j).map_err(|e| e.to_string())
}

/// Writes every sample to `dir`. Returns the number of files.
pub fn write_all(dir: &Path) -> Result<usize, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let (w, h) = (1024u32, 768u32);
    let base = scene(w, h, 1);
    let mut files: Vec<(&str, Vec<u8>)> = Vec::new();

    // 1. Camera-like original.
    let orig = jpeg(&base, 95)?;
    files.push((
        "01-camera-original.jpg",
        with_exif(&orig, &camera_fields(None, "2024:06:01 10:15:00"), Some(&thumb_of(&base)?))?,
    ));

    // 2. Spliced + re-saved in an editor (double compression, local inconsistency, metadata, stale thumbnail).
    let mut first = decode(&jpeg(&base, 70)?)?;
    let other = scene(w, h, 42);
    for y in 256..448 {
        for x in 512..704 {
            first.put_pixel(x, y, *other.get_pixel(x - 300, y - 150));
        }
    }
    let spliced = jpeg(&first, 92)?;
    files.push((
        "02-spliced-resaved.jpg",
        with_exif(
            &spliced,
            &camera_fields(Some("Adobe Photoshop 25.0 (Windows)"), "2024:06:03 18:40:00"),
            Some(&thumb_of(&base)?),
        )?,
    ));

    // 3. Copy-move: the house cloned to the right.
    let mut cm = base.clone();
    for y in 260..460 {
        for x in 200..400 {
            let p = *base.get_pixel(x, y);
            cm.put_pixel(x + 420, y + 10, p);
        }
    }
    files.push(("03-copy-move.jpg", jpeg(&cm, 93)?));

    // 4. Resized (upscaled) PNG.
    let up = image::imageops::resize(
        &image::imageops::crop_imm(&base, 0, 0, 640, 480).to_image(),
        896,
        672,
        image::imageops::FilterType::Triangle,
    );
    files.push(("04-upscaled.png", png(&up)?));

    // 5. AI generator parameters in PNG.
    files.push((
        "05-generator-parameters.png",
        png_text(&png(&scene(512, 384, 7))?, "parameters", "a quiet landscape, house, sun\nSteps: 30, Sampler: DPM++ 2M Karras, CFG scale: 7, Seed: 1234, Size: 512x384, Model: demo"),
    ));

    // 6. Hidden archive appended.
    let mut hidden = jpeg(&scene(640, 480, 9), 90)?;
    hidden.extend_from_slice(b"PK\x03\x04\x14\0\0\0\0\0demo-hidden-archive (not a real zip)");
    files.push(("06-appended-archive.jpg", hidden));

    // 7. Thumbnail mismatch: the thumbnail still shows the house, the main image had it painted over.
    let mut painted = base.clone();
    for y in 230..460 {
        for x in 180..410 {
            let src = *base.get_pixel(x, y.min(400) - 200);
            painted.put_pixel(x, y, src);
        }
    }
    files.push((
        "07-thumbnail-mismatch.jpg",
        with_exif(
            &jpeg(&painted, 92)?,
            &camera_fields(Some("GIMP 2.10"), "2024:06:02 09:00:00"),
            Some(&thumb_of(&base)?),
        )?,
    ));

    // 8. Declared AI-generated via XMP DigitalSourceType.
    let xmp = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" xmlns:Iptc4xmpExt="http://iptc.org/std/Iptc4xmpExt/2008-02-29/" xmlns:xmp="http://ns.adobe.com/xap/1.0/" Iptc4xmpExt:DigitalSourceType="http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia" xmp:CreatorTool="Demo Generator 1.0"/></rdf:RDF></x:xmpmeta>"#;
    let mut app = b"http://ns.adobe.com/xap/1.0/\0".to_vec();
    app.extend_from_slice(xmp.as_bytes());
    files.push(("08-declared-ai.jpg", with_app(&jpeg(&scene(640, 480, 11), 90)?, 0xE1, &app)));

    let n = files.len();
    for (name, bytes) in files {
        std::fs::write(dir.join(name), bytes).map_err(|e| e.to_string())?;
    }
    std::fs::write(
        dir.join("README.txt"),
        "Synthetic Forensica demo images (procedurally generated, public domain).\n\
         01 camera-like original · 02 spliced + re-saved in an editor · 03 copy-move · 04 upscaled PNG ·\n\
         05 Stable-Diffusion-style parameters · 06 appended archive · 07 stale EXIF thumbnail · 08 declared AI (IPTC)\n",
    )
    .map_err(|e| e.to_string())?;
    Ok(n)
}

#[cfg(test)]
mod tests {
    use forensica_core::{CancelToken, ImageInput, Options, Severity};

    fn codes(path: &std::path::Path) -> Vec<(String, Severity)> {
        let input = ImageInput::from_path(path).unwrap();
        let r = forensica_engine::build_pipeline().run(&input, &Options::default(), &CancelToken::new(), &|_| {});
        assert!(r.modules.iter().all(|m| m.error.is_none()), "{:?}", r.modules.iter().filter_map(|m| m.error.clone()).collect::<Vec<_>>());
        r.findings().into_iter().map(|f| (f.code.clone(), f.severity)).collect()
    }

    #[test]
    fn every_sample_triggers_its_detector_and_the_original_stays_quiet() {
        let dir = std::env::temp_dir().join(format!("forensica-samples-{}", std::process::id()));
        super::write_all(&dir).unwrap();
        let expect = [
            ("02-spliced-resaved.jpg", vec!["djpeg.aligned", "djpeg.local_inconsistency", "meta.editing_software", "meta.modified_after_capture"]),
            ("03-copy-move.jpg", vec!["copymove.detected"]),
            ("04-upscaled.png", vec!["resample.global"]),
            ("05-generator-parameters.png", vec!["gen.sd_parameters"]),
            ("06-appended-archive.jpg", vec!["jpeg.hidden_payload"]),
            ("07-thumbnail-mismatch.jpg", vec!["meta.thumbnail_mismatch"]),
            ("08-declared-ai.jpg", vec!["gen.digital_source_type"]),
        ];
        for (file, want) in expect {
            let got = codes(&dir.join(file));
            for w in want {
                assert!(got.iter().any(|(c, _)| c == w), "{file}: missing {w} in {got:?}");
            }
        }
        let original = codes(&dir.join("01-camera-original.jpg"));
        assert!(original.iter().all(|(_, s)| *s <= Severity::Low), "original: {original:?}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
