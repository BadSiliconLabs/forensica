//! EXIF extraction (via `kamadak-exif`) into a display table and a typed summary.

use std::io::Cursor;

use exif::{Context as ExifCtx, Exif, Field, In, Reader, Tag, Value};
use serde::Serialize;

use crate::makernote::{self, MakerNoteInfo};
use crate::table::Group;

/// GPS position in decimal degrees.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Gps {
    /// Latitude (+N).
    pub lat: f64,
    /// Longitude (+E).
    pub lon: f64,
    /// Altitude in metres, if present.
    pub alt: Option<f64>,
}

/// Typed subset of EXIF used by the rules.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ExifSummary {
    /// Whether any EXIF was found.
    pub present: bool,
    /// Camera make.
    pub make: Option<String>,
    /// Camera model.
    pub model: Option<String>,
    /// Software tag.
    pub software: Option<String>,
    /// IFD0 DateTime (a.k.a. ModifyDate).
    pub modify_date: Option<String>,
    /// DateTimeOriginal.
    pub date_original: Option<String>,
    /// DateTimeDigitized (CreateDate).
    pub date_digitized: Option<String>,
    /// OffsetTimeOriginal.
    pub offset_original: Option<String>,
    /// F-number.
    pub f_number: Option<f64>,
    /// Exposure time in seconds.
    pub exposure_time: Option<f64>,
    /// ISO.
    pub iso: Option<u32>,
    /// Focal length (mm).
    pub focal_length: Option<f64>,
    /// PixelXDimension.
    pub pixel_x: Option<u32>,
    /// PixelYDimension.
    pub pixel_y: Option<u32>,
    /// Orientation (1..8).
    pub orientation: Option<u32>,
    /// Lens model.
    pub lens: Option<String>,
    /// Raw GPS latitude/longitude values before range checks.
    pub gps: Option<Gps>,
    /// GPS fields present but unparsable / out of range.
    pub gps_invalid: bool,
    /// Embedded thumbnail (JPEG bytes from IFD1).
    #[serde(skip)]
    pub thumbnail: Option<Vec<u8>>,
    /// MakerNote info.
    pub makernote: Option<MakerNoteInfo>,
    /// Number of EXIF fields.
    pub field_count: usize,
}

/// Result of EXIF extraction.
#[derive(Debug, Clone, Default)]
pub struct ExifData {
    /// Display groups.
    pub groups: Vec<Group>,
    /// Typed summary.
    pub summary: ExifSummary,
}

/// Reads EXIF from any container supported by kamadak-exif (JPEG, TIFF, PNG, WebP, HEIF).
pub fn read(bytes: &[u8]) -> ExifData {
    let mut cur = Cursor::new(bytes);
    match Reader::new().read_from_container(&mut cur) {
        Ok(exif) => from_exif(&exif),
        Err(_) => ExifData::default(),
    }
}

/// Reads EXIF from a raw TIFF-structured buffer (e.g. a JPEG APP1 payload after `Exif\0\0`).
pub fn read_raw(tiff: &[u8]) -> ExifData {
    match Reader::new().read_raw(tiff.to_vec()) {
        Ok(exif) => from_exif(&exif),
        Err(_) => ExifData::default(),
    }
}

fn group_name(f: &Field) -> &'static str {
    match (f.tag.context(), f.ifd_num) {
        (_, In::THUMBNAIL) => "EXIF · Thumbnail IFD",
        (ExifCtx::Tiff, _) => "EXIF · Image (IFD0)",
        (ExifCtx::Exif, _) => "EXIF · Photo",
        (ExifCtx::Gps, _) => "EXIF · GPS",
        (ExifCtx::Interop, _) => "EXIF · Interop",
        _ => "EXIF · Other",
    }
}

fn from_exif(exif: &Exif) -> ExifData {
    let mut groups: Vec<Group> = Vec::new();
    let mut s = ExifSummary { present: true, ..Default::default() };
    for f in exif.fields() {
        s.field_count += 1;
        let name = group_name(f);
        let value = if f.tag == Tag::MakerNote {
            format!("<{} bytes>", value_len(&f.value))
        } else {
            f.display_value().with_unit(exif).to_string()
        };
        let key = match f.tag.description() {
            Some(_) => format!("{}", f.tag),
            None => format!("Tag 0x{:04X}", f.tag.number()),
        };
        match groups.iter_mut().find(|g| g.name == name) {
            Some(g) => g.push(key, value),
            None => {
                let mut g = Group::new(name);
                g.push(key, value);
                groups.push(g);
            }
        }
    }

    let p = In::PRIMARY;
    s.make = ascii(exif, Tag::Make, p);
    s.model = ascii(exif, Tag::Model, p);
    s.software = ascii(exif, Tag::Software, p);
    s.modify_date = ascii(exif, Tag::DateTime, p);
    s.date_original = ascii(exif, Tag::DateTimeOriginal, p);
    s.date_digitized = ascii(exif, Tag::DateTimeDigitized, p);
    s.offset_original = ascii(exif, Tag::OffsetTimeOriginal, p);
    s.lens = ascii(exif, Tag::LensModel, p);
    s.f_number = rational(exif, Tag::FNumber);
    s.exposure_time = rational(exif, Tag::ExposureTime);
    s.focal_length = rational(exif, Tag::FocalLength);
    s.iso = uint(exif, Tag::PhotographicSensitivity);
    s.pixel_x = uint(exif, Tag::PixelXDimension);
    s.pixel_y = uint(exif, Tag::PixelYDimension);
    s.orientation = uint(exif, Tag::Orientation);
    let (gps, invalid) = gps(exif);
    s.gps = gps;
    s.gps_invalid = invalid;
    s.thumbnail = thumbnail(exif);
    if let Some(f) = exif.get_field(Tag::MakerNote, p) {
        if let Value::Undefined(ref data, _) = f.value {
            s.makernote = Some(makernote::inspect(data, s.make.as_deref()));
        }
    }
    ExifData { groups, summary: s }
}

fn value_len(v: &Value) -> usize {
    match v {
        Value::Undefined(d, _) | Value::Byte(d) => d.len(),
        _ => 0,
    }
}

fn ascii(exif: &Exif, tag: Tag, ifd: In) -> Option<String> {
    let f = exif.get_field(tag, ifd)?;
    match &f.value {
        Value::Ascii(v) => {
            let s: String = v
                .iter()
                .map(|x| String::from_utf8_lossy(x).trim_end_matches('\0').trim().to_string())
                .collect::<Vec<_>>()
                .join(" ");
            let s = s.trim().to_string();
            (!s.is_empty()).then_some(s)
        }
        _ => None,
    }
}

fn rational(exif: &Exif, tag: Tag) -> Option<f64> {
    let f = exif.get_field(tag, In::PRIMARY)?;
    match &f.value {
        Value::Rational(v) => v.first().map(|r| if r.denom == 0 { f64::NAN } else { r.to_f64() }),
        Value::SRational(v) => v.first().map(|r| if r.denom == 0 { f64::NAN } else { r.to_f64() }),
        _ => f.value.get_uint(0).map(f64::from),
    }
}

fn uint(exif: &Exif, tag: Tag) -> Option<u32> {
    exif.get_field(tag, In::PRIMARY)?.value.get_uint(0)
}

fn dms(exif: &Exif, tag: Tag) -> Option<f64> {
    let f = exif.get_field(tag, In::PRIMARY)?;
    match &f.value {
        Value::Rational(v) if v.len() >= 3 && v.iter().all(|r| r.denom != 0) => {
            Some(v[0].to_f64() + v[1].to_f64() / 60.0 + v[2].to_f64() / 3600.0)
        }
        _ => None,
    }
}

fn gps(exif: &Exif) -> (Option<Gps>, bool) {
    let has_any = exif.get_field(Tag::GPSLatitude, In::PRIMARY).is_some()
        || exif.get_field(Tag::GPSLongitude, In::PRIMARY).is_some();
    let (Some(mut lat), Some(mut lon)) = (dms(exif, Tag::GPSLatitude), dms(exif, Tag::GPSLongitude)) else {
        return (None, has_any);
    };
    if ascii(exif, Tag::GPSLatitudeRef, In::PRIMARY).as_deref() == Some("S") {
        lat = -lat;
    }
    if ascii(exif, Tag::GPSLongitudeRef, In::PRIMARY).as_deref() == Some("W") {
        lon = -lon;
    }
    let alt = rational(exif, Tag::GPSAltitude).map(|a| {
        let below = exif.get_field(Tag::GPSAltitudeRef, In::PRIMARY).and_then(|f| f.value.get_uint(0)) == Some(1);
        if below {
            -a
        } else {
            a
        }
    });
    let invalid = !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon);
    (Some(Gps { lat, lon, alt }), invalid)
}

fn thumbnail(exif: &Exif) -> Option<Vec<u8>> {
    let off = exif.get_field(Tag::JPEGInterchangeFormat, In::THUMBNAIL)?.value.get_uint(0)? as usize;
    let len = exif.get_field(Tag::JPEGInterchangeFormatLength, In::THUMBNAIL)?.value.get_uint(0)? as usize;
    let buf = exif.buf();
    let data = buf.get(off..off.checked_add(len)?)?;
    data.starts_with(&[0xFF, 0xD8]).then(|| data.to_vec())
}

#[cfg(test)]
pub(crate) mod testutil {
    //! Helpers to synthesise EXIF-bearing JPEGs for tests.
    use exif::experimental::Writer;
    use exif::{Field, In, Rational, Tag, Value};

    /// Builder for test EXIF.
    #[derive(Default)]
    pub struct ExifSpec {
        pub fields: Vec<Field>,
        pub thumb: Option<Vec<u8>>,
    }

    impl ExifSpec {
        pub fn ascii(mut self, tag: Tag, v: &str) -> Self {
            self.fields.push(Field { tag, ifd_num: In::PRIMARY, value: Value::Ascii(vec![v.as_bytes().to_vec()]) });
            self
        }
        pub fn rational(mut self, tag: Tag, n: u32, d: u32) -> Self {
            self.fields.push(Field {
                tag,
                ifd_num: In::PRIMARY,
                value: Value::Rational(vec![Rational { num: n, denom: d }]),
            });
            self
        }
        pub fn short(mut self, tag: Tag, v: u16) -> Self {
            self.fields.push(Field { tag, ifd_num: In::PRIMARY, value: Value::Short(vec![v]) });
            self
        }
        pub fn long(mut self, tag: Tag, v: u32) -> Self {
            self.fields.push(Field { tag, ifd_num: In::PRIMARY, value: Value::Long(vec![v]) });
            self
        }
        pub fn raw(mut self, f: Field) -> Self {
            self.fields.push(f);
            self
        }
        pub fn thumbnail(mut self, jpeg: Vec<u8>) -> Self {
            self.thumb = Some(jpeg);
            self
        }

        /// TIFF-structured EXIF bytes.
        pub fn tiff(&self) -> Vec<u8> {
            let mut w = Writer::new();
            for f in &self.fields {
                w.push_field(f);
            }
            if let Some(t) = &self.thumb {
                w.set_jpeg(t, In::THUMBNAIL);
            }
            let mut out = std::io::Cursor::new(Vec::new());
            w.write(&mut out, false).unwrap();
            out.into_inner()
        }

        /// Inserts this EXIF as APP1 right after SOI.
        pub fn to_jpeg(&self, jpeg: &[u8]) -> Vec<u8> {
            let tiff = self.tiff();
            let mut seg = b"Exif\0\0".to_vec();
            seg.extend_from_slice(&tiff);
            insert_app(jpeg, 0xE1, &seg)
        }
    }

    /// Inserts an APPn segment after SOI.
    pub fn insert_app(jpeg: &[u8], marker: u8, payload: &[u8]) -> Vec<u8> {
        let mut out = jpeg[..2].to_vec();
        out.extend_from_slice(&[0xFF, marker]);
        out.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
        out.extend_from_slice(payload);
        out.extend_from_slice(&jpeg[2..]);
        out
    }

    /// Plain JPEG of a gradient.
    pub fn jpeg(w: u32, h: u32, q: u8) -> Vec<u8> {
        let img = image::RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([(x * 255 / w.max(1)) as u8, (y * 255 / h.max(1)) as u8, 128])
        });
        encode_jpeg(&img, q)
    }

    /// Encodes RGB as JPEG.
    pub fn encode_jpeg(img: &image::RgbImage, q: u8) -> Vec<u8> {
        let mut out = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, q).encode_image(img).unwrap();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::testutil::*;
    use super::*;
    use exif::Rational;

    #[test]
    fn reads_basic_fields_gps_and_thumbnail() {
        let thumb = jpeg(16, 12, 80);
        let spec = ExifSpec::default()
            .ascii(Tag::Make, "Canon")
            .ascii(Tag::Model, "Canon EOS 5D")
            .ascii(Tag::DateTimeOriginal, "2021:05:01 10:00:00")
            .rational(Tag::FNumber, 28, 10)
            .short(Tag::PhotographicSensitivity, 400)
            .ascii(Tag::GPSLatitudeRef, "S")
            .raw(Field {
                tag: Tag::GPSLatitude,
                ifd_num: In::PRIMARY,
                value: Value::Rational(vec![
                    Rational { num: 33, denom: 1 },
                    Rational { num: 30, denom: 1 },
                    Rational { num: 0, denom: 1 },
                ]),
            })
            .ascii(Tag::GPSLongitudeRef, "E")
            .raw(Field {
                tag: Tag::GPSLongitude,
                ifd_num: In::PRIMARY,
                value: Value::Rational(vec![
                    Rational { num: 151, denom: 1 },
                    Rational { num: 15, denom: 1 },
                    Rational { num: 0, denom: 1 },
                ]),
            })
            .thumbnail(thumb.clone());
        let j = spec.to_jpeg(&jpeg(64, 48, 90));
        let d = read(&j);
        assert!(d.summary.present);
        assert_eq!(d.summary.make.as_deref(), Some("Canon"));
        assert_eq!(d.summary.iso, Some(400));
        assert!((d.summary.f_number.unwrap() - 2.8).abs() < 1e-9);
        let g = d.summary.gps.unwrap();
        assert!((g.lat + 33.5).abs() < 1e-9);
        assert!((g.lon - 151.25).abs() < 1e-9);
        assert!(!d.summary.gps_invalid);
        assert_eq!(d.summary.thumbnail.as_deref(), Some(thumb.as_slice()));
        assert!(d.groups.iter().any(|g| g.name.contains("GPS")));
    }

    #[test]
    fn no_exif_is_not_present() {
        let d = read(&jpeg(8, 8, 90));
        assert!(!d.summary.present);
        assert!(d.groups.is_empty());
    }
}
