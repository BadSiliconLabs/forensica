//! ICC profile discovery (JPEG APP2, PNG iCCP, WebP ICCP) and header parsing.

use std::io::Read;

use serde::Serialize;

use crate::table::Group;
use forensica_core::container::{jpeg_layout, png_layout, riff_chunks};
use forensica_core::FileFormat;

/// Parsed ICC header summary.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct IccInfo {
    /// Profile size from the header.
    pub size: u32,
    /// Preferred CMM.
    pub cmm: String,
    /// Version "major.minor".
    pub version: String,
    /// Device class (mntr, scnr, ...).
    pub class: String,
    /// Colour space.
    pub color_space: String,
    /// Profile connection space.
    pub pcs: String,
    /// Creation date.
    pub created: String,
    /// Device manufacturer.
    pub manufacturer: String,
    /// Device model.
    pub model: String,
    /// Profile creator.
    pub creator: String,
    /// Profile description (from `desc`).
    pub description: String,
    /// Copyright (from `cprt`).
    pub copyright: String,
}

impl IccInfo {
    /// Display group.
    pub fn group(&self) -> Group {
        let mut g = Group::new("ICC profile");
        for (k, v) in [
            ("Description", &self.description),
            ("Size", &self.size.to_string()),
            ("CMM", &self.cmm),
            ("Version", &self.version),
            ("Device class", &self.class),
            ("Color space", &self.color_space),
            ("PCS", &self.pcs),
            ("Created", &self.created),
            ("Manufacturer", &self.manufacturer),
            ("Model", &self.model),
            ("Creator", &self.creator),
            ("Copyright", &self.copyright),
        ] {
            if !v.is_empty() {
                g.push(k, v.clone());
            }
        }
        g
    }
}

fn sig(d: &[u8], at: usize) -> String {
    d.get(at..at + 4).map(|s| String::from_utf8_lossy(s).trim_end_matches(['\0', ' ']).to_string()).unwrap_or_default()
}

fn be16(d: &[u8], at: usize) -> u16 {
    d.get(at..at + 2).map(|b| u16::from_be_bytes([b[0], b[1]])).unwrap_or(0)
}

fn be32(d: &[u8], at: usize) -> u32 {
    d.get(at..at + 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]])).unwrap_or(0)
}

/// Parses an ICC profile header and its `desc`/`cprt` tags.
pub fn parse(d: &[u8]) -> Option<IccInfo> {
    if d.len() < 132 || &d[36..40] != b"acsp" {
        return None;
    }
    let mut info = IccInfo {
        size: be32(d, 0),
        cmm: sig(d, 4),
        version: format!("{}.{}", d[8], d[9] >> 4),
        class: sig(d, 12),
        color_space: sig(d, 16),
        pcs: sig(d, 20),
        created: format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            be16(d, 24),
            be16(d, 26),
            be16(d, 28),
            be16(d, 30),
            be16(d, 32),
            be16(d, 34)
        ),
        manufacturer: sig(d, 48),
        model: sig(d, 52),
        creator: sig(d, 80),
        ..Default::default()
    };
    let count = be32(d, 128) as usize;
    for t in 0..count.min(200) {
        let at = 132 + t * 12;
        let tag = d.get(at..at + 4).unwrap_or_default();
        let off = be32(d, at + 4) as usize;
        let len = be32(d, at + 8) as usize;
        let Some(body) = d.get(off..off.saturating_add(len)) else { continue };
        match tag {
            b"desc" => info.description = text_tag(body),
            b"cprt" => info.copyright = text_tag(body),
            _ => {}
        }
    }
    Some(info)
}

fn text_tag(b: &[u8]) -> String {
    match b.get(0..4) {
        Some(b"desc") => {
            let n = be32(b, 8) as usize;
            b.get(12..12 + n.saturating_sub(1)).map(|s| String::from_utf8_lossy(s).into_owned()).unwrap_or_default()
        }
        Some(b"text") => String::from_utf8_lossy(b.get(8..).unwrap_or_default()).trim_end_matches('\0').to_string(),
        Some(b"mluc") => {
            // First record only.
            let len = be32(b, 20) as usize;
            let off = be32(b, 24) as usize;
            let Some(s) = b.get(off..off.saturating_add(len)) else { return String::new() };
            let u: Vec<u16> = s.as_chunks::<2>().0.iter().map(|c| u16::from_be_bytes(*c)).collect();
            String::from_utf16_lossy(&u).trim_end_matches('\0').to_string()
        }
        _ => String::new(),
    }
}

/// Extracts the raw ICC profile from the container.
pub fn extract(bytes: &[u8], format: FileFormat) -> Option<Vec<u8>> {
    match format {
        FileFormat::Jpeg => {
            let layout = jpeg_layout(bytes).ok()?;
            let mut parts: Vec<(u8, &[u8])> = layout
                .app_payloads(bytes, 0xE2, b"ICC_PROFILE\0")
                .into_iter()
                .filter(|p| p.len() > 2)
                .map(|p| (p[0], &p[2..]))
                .collect();
            if parts.is_empty() {
                return None;
            }
            parts.sort_by_key(|p| p.0);
            Some(parts.into_iter().flat_map(|p| p.1.iter().copied()).collect())
        }
        FileFormat::Png => {
            let layout = png_layout(bytes).ok()?;
            let c = layout.chunks.iter().find(|c| c.kind == "iCCP")?;
            let d = bytes.get(c.data.clone())?;
            let nul = d.iter().position(|b| *b == 0)?;
            let comp = d.get(nul + 2..)?;
            let mut out = Vec::new();
            flate2::read::ZlibDecoder::new(comp).take(16 << 20).read_to_end(&mut out).ok()?;
            Some(out)
        }
        FileFormat::Webp => {
            let (chunks, _) = riff_chunks(bytes).ok()?;
            let c = chunks.iter().find(|c| c.kind == "ICCP")?;
            bytes.get(c.data.clone()).map(<[u8]>::to_vec)
        }
        _ => None,
    }
}

#[cfg(test)]
pub(crate) fn sample_profile(desc: &str) -> Vec<u8> {
    let mut d = vec![0u8; 132];
    d[4..8].copy_from_slice(b"lcms");
    d[8] = 2;
    d[9] = 0x10;
    d[12..16].copy_from_slice(b"mntr");
    d[16..20].copy_from_slice(b"RGB ");
    d[20..24].copy_from_slice(b"XYZ ");
    d[24..26].copy_from_slice(&2020u16.to_be_bytes());
    d[36..40].copy_from_slice(b"acsp");
    d[128..132].copy_from_slice(&1u32.to_be_bytes());
    let tag_off = 132 + 12;
    let mut body = b"desc\0\0\0\0".to_vec();
    body.extend_from_slice(&((desc.len() + 1) as u32).to_be_bytes());
    body.extend_from_slice(desc.as_bytes());
    body.push(0);
    d.extend_from_slice(b"desc");
    d.extend_from_slice(&(tag_off as u32).to_be_bytes());
    d.extend_from_slice(&(body.len() as u32).to_be_bytes());
    d.extend_from_slice(&body);
    let n = d.len() as u32;
    d[0..4].copy_from_slice(&n.to_be_bytes());
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exif_read::testutil::{insert_app, jpeg};

    #[test]
    fn parses_profile_from_jpeg() {
        let prof = sample_profile("sRGB IEC61966-2.1");
        let mut app = b"ICC_PROFILE\0\x01\x01".to_vec();
        app.extend_from_slice(&prof);
        let j = insert_app(&jpeg(8, 8, 90), 0xE2, &app);
        let raw = extract(&j, FileFormat::Jpeg).unwrap();
        let info = parse(&raw).unwrap();
        assert_eq!(info.description, "sRGB IEC61966-2.1");
        assert_eq!(info.class, "mntr");
        assert_eq!(info.color_space, "RGB");
        assert_eq!(info.version, "2.1");
        assert!(info.group().get("Description").is_some());
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse(&[0u8; 200]).is_none());
        assert!(extract(b"nope", FileFormat::Jpeg).is_none());
    }
}
