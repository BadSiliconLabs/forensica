//! Textual chunks: PNG tEXt/zTXt/iTXt, JPEG COM segments, GIF comments.

use std::io::Read;

use serde::Serialize;

use crate::table::Group;
use forensica_core::container::{jpeg_layout, png_layout};
use forensica_core::FileFormat;

/// One text entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TextChunk {
    /// Source chunk type (tEXt, zTXt, iTXt, COM).
    pub source: String,
    /// Keyword (empty for COM).
    pub key: String,
    /// Text value.
    pub value: String,
}

const MAX_TEXT: u64 = 4 << 20;

fn latin1(b: &[u8]) -> String {
    b.iter().map(|&c| char::from(c)).collect()
}

fn inflate(b: &[u8]) -> Option<String> {
    let mut out = Vec::new();
    flate2::read::ZlibDecoder::new(b).take(MAX_TEXT).read_to_end(&mut out).ok()?;
    Some(String::from_utf8_lossy(&out).into_owned())
}

fn png_text(kind: &str, d: &[u8]) -> Option<TextChunk> {
    let nul = d.iter().position(|b| *b == 0)?;
    let key = latin1(&d[..nul]);
    let rest = &d[nul + 1..];
    let value = match kind {
        "tEXt" => latin1(rest),
        "zTXt" => inflate(rest.get(1..)?)?,
        "iTXt" => {
            let compressed = *rest.first()? == 1;
            let rest = rest.get(2..)?;
            let l = rest.iter().position(|b| *b == 0)?;
            let rest = rest.get(l + 1..)?;
            let t = rest.iter().position(|b| *b == 0)?;
            let text = rest.get(t + 1..)?;
            if compressed {
                inflate(text)?
            } else {
                String::from_utf8_lossy(text).into_owned()
            }
        }
        _ => return None,
    };
    Some(TextChunk { source: kind.to_string(), key, value })
}

/// Extracts textual chunks from the container.
pub fn read(bytes: &[u8], format: FileFormat) -> Vec<TextChunk> {
    match format {
        FileFormat::Png => png_layout(bytes)
            .map(|l| {
                l.chunks
                    .iter()
                    .filter(|c| matches!(c.kind.as_str(), "tEXt" | "zTXt" | "iTXt"))
                    .filter_map(|c| png_text(&c.kind, bytes.get(c.data.clone())?))
                    .collect()
            })
            .unwrap_or_default(),
        FileFormat::Jpeg => jpeg_layout(bytes)
            .map(|l| {
                l.with_marker(0xFE)
                    .filter_map(|s| bytes.get(s.data.clone()))
                    .map(|d| TextChunk {
                        source: "COM".into(),
                        key: String::new(),
                        value: String::from_utf8_lossy(d).trim_end_matches('\0').to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Display group for text chunks (XMP packets are summarised, not dumped).
pub fn group(chunks: &[TextChunk]) -> Option<Group> {
    if chunks.is_empty() {
        return None;
    }
    let mut g = Group::new("Text chunks");
    for c in chunks {
        let key = if c.key.is_empty() { c.source.clone() } else { format!("{} ({})", c.key, c.source) };
        let value =
            if c.key == "XML:com.adobe.xmp" { "<XMP packet, see XMP group>".to_string() } else { c.value.clone() };
        g.push(key, value);
    }
    Some(g)
}

#[cfg(test)]
pub(crate) fn png_with_text(entries: &[(&str, &str, &str)]) -> Vec<u8> {
    use std::io::Write;
    let img = image::RgbImage::from_pixel(8, 8, image::Rgb([5, 6, 7]));
    let mut buf = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img).write_to(&mut buf, image::ImageFormat::Png).unwrap();
    let png = buf.into_inner();
    // Insert chunks before IEND (last 12 bytes).
    let (head, tail) = png.split_at(png.len() - 12);
    let mut out = head.to_vec();
    for (kind, key, val) in entries {
        let mut data = key.as_bytes().to_vec();
        data.push(0);
        match *kind {
            "zTXt" => {
                data.push(0);
                let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
                e.write_all(val.as_bytes()).unwrap();
                data.extend_from_slice(&e.finish().unwrap());
            }
            "iTXt" => {
                data.extend_from_slice(&[0, 0, 0, 0]);
                data.extend_from_slice(val.as_bytes());
            }
            _ => data.extend_from_slice(val.as_bytes()),
        }
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut crc_in = kind.as_bytes().to_vec();
        crc_in.extend_from_slice(&data);
        out.extend_from_slice(&crc_in);
        out.extend_from_slice(&forensica_core::container::crc32(&crc_in).to_be_bytes());
    }
    out.extend_from_slice(tail);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_all_png_text_kinds() {
        let png =
            png_with_text(&[("tEXt", "Title", "hello"), ("zTXt", "Comment", "zipped"), ("iTXt", "Author", "ünï")]);
        let t = read(&png, FileFormat::Png);
        assert_eq!(t.len(), 3);
        assert_eq!(t[0].value, "hello");
        assert_eq!(t[1].value, "zipped");
        assert_eq!(t[2].value, "ünï");
        assert_eq!(group(&t).unwrap().entries.len(), 3);
    }

    #[test]
    fn reads_jpeg_comment() {
        let j = crate::exif_read::testutil::jpeg(8, 8, 90);
        let j = crate::exif_read::testutil::insert_app(&j, 0xFE, b"made with tool X");
        let t = read(&j, FileFormat::Jpeg);
        assert_eq!(t[0].value, "made with tool X");
    }
}
