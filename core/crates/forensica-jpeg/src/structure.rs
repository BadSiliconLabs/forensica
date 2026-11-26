//! JPEG file structure: frame, tables, scans, APP segments, trailing data.

use std::ops::Range;

use serde::Serialize;

use crate::quant::ZIGZAG;
use forensica_core::container::{find_all, jpeg_layout, JpegLayout};
use forensica_core::{Error, Result};

/// A frame component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Component {
    /// Component id.
    pub id: u8,
    /// Horizontal sampling factor.
    pub h: u8,
    /// Vertical sampling factor.
    pub v: u8,
    /// Quantisation table id.
    pub tq: u8,
}

/// Frame header (SOFn).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Frame {
    /// SOF marker (0xC0..0xCF).
    pub marker: u8,
    /// Sample precision.
    pub precision: u8,
    /// Width.
    pub width: u16,
    /// Height.
    pub height: u16,
    /// Components.
    pub components: Vec<Component>,
}

impl Frame {
    /// Baseline/extended sequential Huffman.
    pub fn sequential_huffman(&self) -> bool {
        matches!(self.marker, 0xC0 | 0xC1)
    }
    /// Progressive.
    pub fn progressive(&self) -> bool {
        matches!(self.marker, 0xC2 | 0xC6 | 0xCA | 0xCE)
    }
    /// Arithmetic coding.
    pub fn arithmetic(&self) -> bool {
        self.marker >= 0xC9
    }
    /// Human-readable process name.
    pub fn process(&self) -> &'static str {
        match self.marker {
            0xC0 => "baseline",
            0xC1 => "extended sequential",
            0xC2 => "progressive",
            0xC3 => "lossless",
            0xC9 => "extended sequential (arithmetic)",
            0xCA => "progressive (arithmetic)",
            _ => "other",
        }
    }
    /// Chroma subsampling label.
    pub fn subsampling(&self) -> String {
        let Some(y) = self.components.first() else { return "-".into() };
        if self.components.len() == 1 {
            return "grayscale".into();
        }
        let c = self.components.get(1).copied().unwrap_or(*y);
        match (y.h / c.h.max(1), y.v / c.v.max(1)) {
            (1, 1) => "4:4:4",
            (2, 1) => "4:2:2",
            (2, 2) => "4:2:0",
            (1, 2) => "4:4:0",
            (4, 1) => "4:1:1",
            _ => "unusual",
        }
        .to_string()
    }
}

/// A quantisation table (natural order).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QuantTable {
    /// Table id (0..3).
    pub id: u8,
    /// 16-bit precision.
    pub precision16: bool,
    /// Values, natural (row-major) order.
    #[serde(serialize_with = "ser_table")]
    pub values: [u16; 64],
}

fn ser_table<S: serde::Serializer>(t: &[u16; 64], s: S) -> std::result::Result<S::Ok, S::Error> {
    s.collect_seq(t.iter())
}

/// A Huffman table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HuffTable {
    /// 0 = DC, 1 = AC.
    pub class: u8,
    /// Table id.
    pub id: u8,
    /// Number of codes of each length 1..16.
    pub counts: [u8; 16],
    /// Symbols.
    pub symbols: Vec<u8>,
}

/// Scan header (SOS) and the tables in effect for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Scan {
    /// (component id, DC table, AC table).
    pub components: Vec<(u8, u8, u8)>,
    /// Spectral selection start.
    pub ss: u8,
    /// Spectral selection end.
    pub se: u8,
    /// Successive approximation high.
    pub ah: u8,
    /// Successive approximation low.
    pub al: u8,
    /// Entropy-coded data range.
    pub entropy: Range<usize>,
    /// Restart interval in effect.
    pub restart_interval: u16,
    /// Quant tables in effect (by id).
    #[serde(skip)]
    pub quant: [Option<[u16; 64]>; 4],
    /// Huffman tables in effect: dc[0..4], ac[0..4].
    #[serde(skip)]
    pub huff: [Option<HuffTable>; 8],
}

/// An APPn / COM segment summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppSegment {
    /// Marker byte.
    pub marker: u8,
    /// Identifier string (e.g. "Exif", "JFIF", "http://ns.adobe.com/xap/1.0/").
    pub signature: String,
    /// Offset in file.
    pub offset: usize,
    /// Payload length.
    pub length: usize,
}

/// Data after EOI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Trailing {
    /// Offset of the first trailing byte.
    pub offset: usize,
    /// Length.
    pub length: usize,
    /// Best guess of the content.
    pub kind: String,
    /// Number of JPEG SOI signatures inside.
    pub embedded_jpegs: usize,
}

/// Parsed JPEG.
#[derive(Debug, Clone, Default, Serialize)]
pub struct JpegInfo {
    /// Marker layout.
    pub layout: JpegLayout,
    /// Frame header.
    pub frame: Option<Frame>,
    /// Every quantisation table definition in file order.
    pub quant_tables: Vec<QuantTable>,
    /// Every Huffman table definition in file order.
    pub huff_tables: Vec<HuffTable>,
    /// Scans.
    pub scans: Vec<Scan>,
    /// APP/COM segments.
    pub apps: Vec<AppSegment>,
    /// Trailing data.
    pub trailing: Option<Trailing>,
    /// Marker names in order (entropy data and RSTs omitted).
    pub markers: Vec<String>,
}

/// Marker display name.
pub fn marker_name(m: u8) -> String {
    match m {
        0xD8 => "SOI".into(),
        0xD9 => "EOI".into(),
        0xDA => "SOS".into(),
        0xDB => "DQT".into(),
        0xC4 => "DHT".into(),
        0xDD => "DRI".into(),
        0xFE => "COM".into(),
        0xCC => "DAC".into(),
        0xC0..=0xCF => format!("SOF{}", m - 0xC0),
        0xE0..=0xEF => format!("APP{}", m - 0xE0),
        0xD0..=0xD7 => format!("RST{}", m - 0xD0),
        _ => format!("0x{m:02X}"),
    }
}

fn signature(payload: &[u8]) -> String {
    let end = payload.iter().take(40).position(|b| *b == 0).unwrap_or(payload.len().min(16));
    let s: String =
        payload[..end].iter().map(|&c| if c.is_ascii_graphic() || c == b' ' { char::from(c) } else { '.' }).collect();
    s.trim().to_string()
}

fn parse_dqt(d: &[u8], out: &mut Vec<QuantTable>) -> Result<()> {
    let mut i = 0;
    while i < d.len() {
        let pq = d[i] >> 4;
        let id = d[i] & 0x0F;
        i += 1;
        let n = if pq == 1 { 128 } else { 64 };
        let t = d.get(i..i + n).ok_or_else(|| Error::malformed("JPEG", "short DQT"))?;
        let mut values = [0u16; 64];
        for k in 0..64 {
            let v = if pq == 1 { u16::from_be_bytes([t[2 * k], t[2 * k + 1]]) } else { u16::from(t[k]) };
            values[ZIGZAG[k]] = v;
        }
        out.push(QuantTable { id, precision16: pq == 1, values });
        i += n;
    }
    Ok(())
}

fn parse_dht(d: &[u8], out: &mut Vec<HuffTable>) -> Result<()> {
    let mut i = 0;
    while i + 17 <= d.len() {
        let class = d[i] >> 4;
        let id = d[i] & 0x0F;
        let mut counts = [0u8; 16];
        counts.copy_from_slice(&d[i + 1..i + 17]);
        let total: usize = counts.iter().map(|c| usize::from(*c)).sum();
        let symbols = d.get(i + 17..i + 17 + total).ok_or_else(|| Error::malformed("JPEG", "short DHT"))?.to_vec();
        out.push(HuffTable { class, id, counts, symbols });
        i += 17 + total;
    }
    Ok(())
}

fn parse_sof(marker: u8, d: &[u8]) -> Result<Frame> {
    if d.len() < 6 {
        return Err(Error::malformed("JPEG", "short SOF"));
    }
    let n = usize::from(d[5]);
    let comps = d.get(6..6 + 3 * n).ok_or_else(|| Error::malformed("JPEG", "short SOF components"))?;
    Ok(Frame {
        marker,
        precision: d[0],
        height: u16::from_be_bytes([d[1], d[2]]),
        width: u16::from_be_bytes([d[3], d[4]]),
        components: comps.chunks(3).map(|c| Component { id: c[0], h: c[1] >> 4, v: c[1] & 0x0F, tq: c[2] }).collect(),
    })
}

fn classify_trailing(t: &[u8]) -> String {
    if t.iter().all(|b| *b == 0 || *b == 0xFF) {
        "padding (zeros/0xFF)".into()
    } else if t.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "JPEG image".into()
    } else if t.starts_with(b"PK\x03\x04") {
        "ZIP archive".into()
    } else if t.starts_with(b"Rar!") {
        "RAR archive".into()
    } else if t.starts_with(b"7z\xBC\xAF") {
        "7-Zip archive".into()
    } else if t.starts_with(b"\x89PNG") {
        "PNG image".into()
    } else if t.starts_with(b"%PDF") {
        "PDF document".into()
    } else if t.windows(4).take(4096).any(|w| w == b"PK\x03\x04") {
        "contains ZIP data".into()
    } else if t.iter().take(512).all(|b| b.is_ascii() && (b.is_ascii_graphic() || b.is_ascii_whitespace())) {
        "text".into()
    } else {
        "unknown binary data".into()
    }
}

/// Parses a JPEG file.
pub fn parse(bytes: &[u8]) -> Result<JpegInfo> {
    let layout = jpeg_layout(bytes)?;
    let mut info = JpegInfo::default();
    let mut restart = 0u16;
    let mut quant: [Option<[u16; 64]>; 4] = [None; 4];
    let mut huff: [Option<HuffTable>; 8] = Default::default();
    for seg in &layout.segments {
        info.markers.push(marker_name(seg.marker));
        let d = bytes.get(seg.data.clone()).unwrap_or_default();
        match seg.marker {
            0xDB => {
                let before = info.quant_tables.len();
                parse_dqt(d, &mut info.quant_tables)?;
                for t in &info.quant_tables[before..] {
                    quant[usize::from(t.id & 3)] = Some(t.values);
                }
            }
            0xC4 => {
                let before = info.huff_tables.len();
                parse_dht(d, &mut info.huff_tables)?;
                for t in &info.huff_tables[before..] {
                    huff[usize::from(t.class.min(1)) * 4 + usize::from(t.id & 3)] = Some(t.clone());
                }
            }
            0xDD if d.len() >= 2 => restart = u16::from_be_bytes([d[0], d[1]]),
            0xC0..=0xCF if !matches!(seg.marker, 0xC4 | 0xC8 | 0xCC) => {
                if info.frame.is_none() {
                    info.frame = Some(parse_sof(seg.marker, d)?);
                }
            }
            0xDA => {
                let n = usize::from(*d.first().unwrap_or(&0));
                let comps = d.get(1..1 + 2 * n).unwrap_or_default();
                let tail = d.get(1 + 2 * n..1 + 2 * n + 3).unwrap_or(&[0, 63, 0]);
                info.scans.push(Scan {
                    components: comps.chunks(2).map(|c| (c[0], c[1] >> 4, c[1] & 0x0F)).collect(),
                    ss: tail[0],
                    se: tail[1],
                    ah: tail[2] >> 4,
                    al: tail[2] & 0x0F,
                    entropy: seg.entropy.clone().unwrap_or(0..0),
                    restart_interval: restart,
                    quant,
                    huff: huff.clone(),
                });
            }
            0xE0..=0xEF | 0xFE => info.apps.push(AppSegment {
                marker: seg.marker,
                signature: if seg.marker == 0xFE { "comment".into() } else { signature(d) },
                offset: seg.offset,
                length: d.len(),
            }),
            _ => {}
        }
    }
    if let Some(end) = layout.eoi_end {
        if end < bytes.len() {
            let t = &bytes[end..];
            info.trailing = Some(Trailing {
                offset: end,
                length: t.len(),
                kind: classify_trailing(t),
                embedded_jpegs: find_all(t, &[0xFF, 0xD8, 0xFF]).len(),
            });
        }
    }
    info.layout = layout;
    Ok(info)
}

/// Standard (Annex K) Huffman code-length counts: DC luma, AC luma, DC chroma, AC chroma.
pub const STD_HUFF_COUNTS: [[u8; 16]; 4] = [
    [0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0],
    [0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 0x7d],
    [0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0],
    [0, 2, 1, 2, 4, 4, 3, 4, 7, 5, 4, 4, 0, 1, 2, 0x77],
];

impl JpegInfo {
    /// Whether every Huffman table is one of the Annex K standard tables.
    pub fn standard_huffman(&self) -> bool {
        !self.huff_tables.is_empty() && self.huff_tables.iter().all(|t| STD_HUFF_COUNTS.contains(&t.counts))
    }

    /// Luma and first chroma quantisation tables (as used by the frame components).
    pub fn frame_tables(&self) -> (Option<[u16; 64]>, Option<[u16; 64]>) {
        let Some(f) = &self.frame else { return (None, None) };
        let lookup = |tq: u8| self.quant_tables.iter().rev().find(|t| t.id == tq).map(|t| t.values);
        let l = f.components.first().and_then(|c| lookup(c.tq));
        let c = f.components.get(1).and_then(|c| lookup(c.tq));
        (l, c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil;

    #[test]
    fn parses_encoder_output() {
        let j = testutil::jpeg(64, 48, 85);
        let info = parse(&j).unwrap();
        let f = info.frame.as_ref().unwrap();
        assert_eq!((f.width, f.height), (64, 48));
        assert_eq!(f.process(), "baseline");
        assert_eq!(f.components.len(), 3);
        assert!(info.quant_tables.len() >= 2);
        assert_eq!(info.scans.len(), 1);
        assert!(info.trailing.is_none());
        assert_eq!(info.markers.first().map(String::as_str), Some("SOI"));
        assert_eq!(info.markers.last().map(String::as_str), Some("EOI"));
        let (l, _) = info.frame_tables();
        assert!(l.is_some());
    }

    #[test]
    fn detects_trailing_archive() {
        let mut j = testutil::jpeg(16, 16, 85);
        j.extend_from_slice(b"PK\x03\x04hidden.txt....");
        let info = parse(&j).unwrap();
        let t = info.trailing.unwrap();
        assert_eq!(t.kind, "ZIP archive");
        assert_eq!(t.length, 18);
        let mut k = testutil::jpeg(16, 16, 85);
        k.extend_from_slice(&[0u8; 32]);
        assert_eq!(parse(&k).unwrap().trailing.unwrap().kind, "padding (zeros/0xFF)");
    }

    #[test]
    fn names() {
        assert_eq!(marker_name(0xC2), "SOF2");
        assert_eq!(marker_name(0xE1), "APP1");
    }
}
