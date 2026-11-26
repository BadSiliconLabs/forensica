//! Low-level container walkers shared by the metadata and JPEG crates.
//! All parsers are bounds-checked and never panic on malformed input.

use std::ops::Range;

use serde::Serialize;

use crate::{Error, Result};

/// One JPEG marker segment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JpegSegment {
    /// Marker byte (the byte after 0xFF).
    pub marker: u8,
    /// Offset of the 0xFF byte.
    pub offset: usize,
    /// Payload range (after the 2-byte length field). Empty for standalone markers.
    pub data: Range<usize>,
    /// For SOS: range of the entropy-coded data that follows the header.
    pub entropy: Option<Range<usize>>,
}

/// Layout of a JPEG file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct JpegLayout {
    /// Segments in file order (SOI first).
    pub segments: Vec<JpegSegment>,
    /// Offset just past the EOI marker, if one was found.
    pub eoi_end: Option<usize>,
    /// Whether parsing stopped early because of malformed data.
    pub truncated: bool,
}

impl JpegLayout {
    /// Segments with the given marker.
    pub fn with_marker(&self, marker: u8) -> impl Iterator<Item = &JpegSegment> {
        self.segments.iter().filter(move |s| s.marker == marker)
    }

    /// APPn payloads whose payload starts with `sig`, returning the bytes after the signature.
    pub fn app_payloads<'a>(&'a self, bytes: &'a [u8], marker: u8, sig: &'a [u8]) -> Vec<&'a [u8]> {
        self.with_marker(marker)
            .filter_map(|s| bytes.get(s.data.clone()))
            .filter(|p| p.starts_with(sig))
            .map(|p| &p[sig.len()..])
            .collect()
    }
}

fn is_standalone(m: u8) -> bool {
    matches!(m, 0xD8 | 0xD9 | 0x01 | 0xD0..=0xD7)
}

/// Walks the marker segments of a JPEG file.
pub fn jpeg_layout(b: &[u8]) -> Result<JpegLayout> {
    if !b.starts_with(&[0xFF, 0xD8]) {
        return Err(Error::malformed("JPEG", "missing SOI"));
    }
    let mut out = JpegLayout::default();
    out.segments.push(JpegSegment { marker: 0xD8, offset: 0, data: 2..2, entropy: None });
    let mut i = 2;
    loop {
        // Expect a marker; tolerate fill bytes.
        if i >= b.len() {
            out.truncated = true;
            break;
        }
        if b[i] != 0xFF {
            out.truncated = true;
            break;
        }
        let start = i;
        while i < b.len() && b[i] == 0xFF {
            i += 1;
        }
        let Some(&m) = b.get(i) else {
            out.truncated = true;
            break;
        };
        i += 1;
        if m == 0xD9 {
            out.segments.push(JpegSegment { marker: m, offset: start, data: i..i, entropy: None });
            out.eoi_end = Some(i);
            break;
        }
        if is_standalone(m) {
            out.segments.push(JpegSegment { marker: m, offset: start, data: i..i, entropy: None });
            continue;
        }
        let Some(len_bytes) = b.get(i..i + 2) else {
            out.truncated = true;
            break;
        };
        let len = usize::from(u16::from_be_bytes([len_bytes[0], len_bytes[1]]));
        if len < 2 || i + len > b.len() {
            out.truncated = true;
            break;
        }
        let data = i + 2..i + len;
        i += len;
        let entropy = if m == 0xDA {
            let s = i;
            i = skip_entropy(b, i);
            Some(s..i)
        } else {
            None
        };
        out.segments.push(JpegSegment { marker: m, offset: start, data, entropy });
    }
    Ok(out)
}

/// Returns the offset of the next real marker after entropy-coded data.
/// RST markers are treated as part of the scan.
fn skip_entropy(b: &[u8], mut i: usize) -> usize {
    while i + 1 < b.len() {
        if b[i] == 0xFF {
            let n = b[i + 1];
            if n == 0x00 || (0xD0..=0xD7).contains(&n) || n == 0xFF {
                i += if n == 0xFF { 1 } else { 2 };
                continue;
            }
            return i;
        }
        i += 1;
    }
    b.len()
}

/// A PNG chunk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PngChunk {
    /// Four-character type.
    pub kind: String,
    /// Offset of the length field.
    pub offset: usize,
    /// Data range.
    pub data: Range<usize>,
    /// Whether the stored CRC matches.
    pub crc_ok: bool,
}

/// Layout of a PNG file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct PngLayout {
    /// Chunks in order.
    pub chunks: Vec<PngChunk>,
    /// Offset just past IEND's CRC.
    pub iend_end: Option<usize>,
    /// Malformed / truncated.
    pub truncated: bool,
}

/// Walks the chunks of a PNG file.
pub fn png_layout(b: &[u8]) -> Result<PngLayout> {
    if !b.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(Error::malformed("PNG", "bad signature"));
    }
    let mut out = PngLayout::default();
    let mut i = 8;
    while i + 12 <= b.len() {
        let len = u32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]) as usize;
        let ty = &b[i + 4..i + 8];
        let Some(end) = (i + 12).checked_add(len).filter(|e| *e <= b.len()) else {
            out.truncated = true;
            break;
        };
        let stored = u32::from_be_bytes([b[end - 4], b[end - 3], b[end - 2], b[end - 1]]);
        let crc_ok = crc32(&b[i + 4..end - 4]) == stored;
        out.chunks.push(PngChunk {
            kind: String::from_utf8_lossy(ty).into_owned(),
            offset: i,
            data: i + 8..i + 8 + len,
            crc_ok,
        });
        i = end;
        if ty == b"IEND" {
            out.iend_end = Some(end);
            break;
        }
    }
    if out.iend_end.is_none() {
        out.truncated = true;
    }
    Ok(out)
}

/// CRC-32 (IEEE) as used by PNG.
pub fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (n, slot) in table.iter_mut().enumerate() {
        let mut c = n as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        *slot = c;
    }
    let mut crc = 0xFFFF_FFFFu32;
    for &x in data {
        crc = table[((crc ^ u32::from(x)) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

/// A RIFF (WebP) chunk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RiffChunk {
    /// FourCC.
    pub kind: String,
    /// Offset of the FourCC.
    pub offset: usize,
    /// Data range.
    pub data: Range<usize>,
}

/// Walks the top-level chunks of a RIFF/WebP file. Returns chunks and the
/// offset where the RIFF container ends (declared size).
pub fn riff_chunks(b: &[u8]) -> Result<(Vec<RiffChunk>, usize)> {
    if b.len() < 12 || &b[0..4] != b"RIFF" {
        return Err(Error::malformed("RIFF", "bad header"));
    }
    let declared = (u32::from_le_bytes([b[4], b[5], b[6], b[7]]) as usize).saturating_add(8);
    let end = declared.min(b.len());
    let mut out = Vec::new();
    let mut i = 12;
    while i + 8 <= end {
        let len = u32::from_le_bytes([b[i + 4], b[i + 5], b[i + 6], b[i + 7]]) as usize;
        let ds = i + 8;
        let de = ds.saturating_add(len).min(end);
        out.push(RiffChunk { kind: String::from_utf8_lossy(&b[i..i + 4]).into_owned(), offset: i, data: ds..de });
        i = ds.saturating_add(len).saturating_add(len & 1);
    }
    Ok((out, declared))
}

/// An ISO-BMFF box (HEIF/AVIF).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BmffBox {
    /// Box type.
    pub kind: String,
    /// Offset of the box header.
    pub offset: usize,
    /// Payload range.
    pub data: Range<usize>,
}

/// Walks boxes inside `range` (non-recursive).
pub fn bmff_boxes(b: &[u8], range: Range<usize>) -> Vec<BmffBox> {
    let mut out = Vec::new();
    let mut i = range.start;
    let end = range.end.min(b.len());
    while i + 8 <= end {
        let size32 = u32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]) as usize;
        let kind = String::from_utf8_lossy(&b[i + 4..i + 8]).into_owned();
        let (header, size) = match size32 {
            0 => (8, end - i),
            1 => {
                let Some(s) = b.get(i + 8..i + 16) else { break };
                let mut a = [0u8; 8];
                a.copy_from_slice(s);
                (16, usize::try_from(u64::from_be_bytes(a)).unwrap_or(usize::MAX))
            }
            s => (8, s),
        };
        if size < header || i.saturating_add(size) > end {
            break;
        }
        out.push(BmffBox { kind, offset: i, data: i + header..i + size });
        i += size;
    }
    out
}

/// Finds all occurrences of `needle` in `hay` (non-overlapping).
pub fn find_all(hay: &[u8], needle: &[u8]) -> Vec<usize> {
    let mut out = Vec::new();
    if needle.is_empty() || hay.len() < needle.len() {
        return out;
    }
    let mut i = 0;
    while i + needle.len() <= hay.len() {
        if hay[i] == needle[0] && &hay[i..i + needle.len()] == needle {
            out.push(i);
            i += needle.len();
        } else {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_jpeg() -> Vec<u8> {
        let img = image::RgbImage::from_pixel(16, 16, image::Rgb([100, 150, 200]));
        let mut buf = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img).write_to(&mut buf, image::ImageFormat::Jpeg).unwrap();
        buf.into_inner()
    }

    #[test]
    fn walks_jpeg() {
        let mut j = tiny_jpeg();
        let l = jpeg_layout(&j).unwrap();
        assert_eq!(l.segments[0].marker, 0xD8);
        assert!(l.with_marker(0xDB).count() >= 1);
        assert!(l.with_marker(0xDA).next().unwrap().entropy.is_some());
        assert_eq!(l.eoi_end, Some(j.len()));
        j.extend_from_slice(b"TRAILING");
        let l2 = jpeg_layout(&j).unwrap();
        assert_eq!(l2.eoi_end, Some(j.len() - 8));
    }

    #[test]
    fn malformed_jpeg_does_not_panic() {
        for n in 0..40 {
            let j = tiny_jpeg();
            let cut = &j[..j.len().min(n * 17)];
            let _ = jpeg_layout(cut);
        }
        assert!(jpeg_layout(b"nope").is_err());
    }

    #[test]
    fn walks_png_and_crc() {
        let img = image::RgbImage::from_pixel(4, 4, image::Rgb([1, 2, 3]));
        let mut buf = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img).write_to(&mut buf, image::ImageFormat::Png).unwrap();
        let b = buf.into_inner();
        let l = png_layout(&b).unwrap();
        assert_eq!(l.chunks[0].kind, "IHDR");
        assert!(l.chunks.iter().all(|c| c.crc_ok));
        assert_eq!(l.iend_end, Some(b.len()));
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn walks_riff_and_bmff() {
        let mut w = b"RIFF\0\0\0\0WEBPVP8X\x04\0\0\0abcdEXIF\x03\0\0\0xyz\0".to_vec();
        let n = (w.len() - 8) as u32;
        w[4..8].copy_from_slice(&n.to_le_bytes());
        let (c, _) = riff_chunks(&w).unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(&w[c[1].data.clone()], b"xyz");
        let h = b"\0\0\0\x10ftypheic\0\0\0\0\0\0\0\x08free".to_vec();
        let boxes = bmff_boxes(&h, 0..h.len());
        assert_eq!(boxes.len(), 2);
        assert_eq!(boxes[1].kind, "free");
    }

    #[test]
    fn find_all_works() {
        assert_eq!(find_all(b"abcabcab", b"abc"), vec![0, 3]);
        assert!(find_all(b"ab", b"abc").is_empty());
    }
}
