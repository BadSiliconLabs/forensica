//! Input handling: immutable bytes, SHA-256, magic-byte sniffing and lazy decoding.

use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use image::{imageops::FilterType, RgbImage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{Error, Result};

/// Container format detected from magic bytes (never from the extension).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileFormat {
    /// JPEG / JFIF / EXIF.
    Jpeg,
    /// PNG.
    Png,
    /// WebP (RIFF).
    Webp,
    /// GIF.
    Gif,
    /// TIFF (incl. many raw formats).
    Tiff,
    /// Windows bitmap.
    Bmp,
    /// HEIF / HEIC.
    Heif,
    /// AVIF.
    Avif,
    /// JPEG XL.
    Jxl,
    /// Adobe Photoshop document.
    Psd,
    /// Unknown.
    Unknown,
}

impl FileFormat {
    /// Sniffs the format from the first bytes of a file.
    pub fn sniff(b: &[u8]) -> Self {
        if b.starts_with(&[0xFF, 0xD8, 0xFF]) {
            Self::Jpeg
        } else if b.starts_with(b"\x89PNG\r\n\x1a\n") {
            Self::Png
        } else if b.len() >= 12 && &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP" {
            Self::Webp
        } else if b.starts_with(b"GIF87a") || b.starts_with(b"GIF89a") {
            Self::Gif
        } else if b.starts_with(b"II*\0") || b.starts_with(b"MM\0*") {
            Self::Tiff
        } else if b.starts_with(b"BM") && b.len() > 14 {
            Self::Bmp
        } else if b.starts_with(&[0xFF, 0x0A]) || b.starts_with(b"\0\0\0\x0cJXL \r\n\x87\n") {
            Self::Jxl
        } else if b.starts_with(b"8BPS") {
            Self::Psd
        } else if b.len() >= 12 && &b[4..8] == b"ftyp" {
            Self::sniff_ftyp(b)
        } else {
            Self::Unknown
        }
    }

    fn sniff_ftyp(b: &[u8]) -> Self {
        let size = u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize;
        let end = size.clamp(12, b.len().min(256));
        let brands: Vec<&[u8]> = std::iter::once(&b[8..12])
            .chain(b[16.min(end)..end].as_chunks::<4>().0.iter().map(<[u8; 4]>::as_slice))
            .collect();
        if brands.iter().any(|x| *x == b"avif" || *x == b"avis") {
            Self::Avif
        } else if brands
            .iter()
            .any(|x| matches!(*x, b"heic" | b"heix" | b"hevc" | b"heim" | b"heis" | b"mif1" | b"msf1"))
        {
            Self::Heif
        } else {
            Self::Unknown
        }
    }

    /// Typical MIME type.
    pub fn mime(self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::Webp => "image/webp",
            Self::Gif => "image/gif",
            Self::Tiff => "image/tiff",
            Self::Bmp => "image/bmp",
            Self::Heif => "image/heif",
            Self::Avif => "image/avif",
            Self::Jxl => "image/jxl",
            Self::Psd => "image/vnd.adobe.photoshop",
            Self::Unknown => "application/octet-stream",
        }
    }

    /// File extensions conventionally used for this format.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::Jpeg => &["jpg", "jpeg", "jpe", "jfif"],
            Self::Png => &["png"],
            Self::Webp => &["webp"],
            Self::Gif => &["gif"],
            Self::Tiff => &["tif", "tiff", "dng", "nef", "cr2", "arw", "orf", "rw2", "pef"],
            Self::Bmp => &["bmp", "dib"],
            Self::Heif => &["heic", "heif", "hif"],
            Self::Avif => &["avif"],
            Self::Jxl => &["jxl"],
            Self::Psd => &["psd"],
            Self::Unknown => &[],
        }
    }

    /// Whether this crate can decode pixels for the format.
    pub fn decodable(self) -> bool {
        matches!(self, Self::Jpeg | Self::Png | Self::Webp | Self::Gif | Self::Tiff | Self::Bmp)
    }

    fn image_format(self) -> Option<image::ImageFormat> {
        Some(match self {
            Self::Jpeg => image::ImageFormat::Jpeg,
            Self::Png => image::ImageFormat::Png,
            Self::Webp => image::ImageFormat::WebP,
            Self::Gif => image::ImageFormat::Gif,
            Self::Tiff => image::ImageFormat::Tiff,
            Self::Bmp => image::ImageFormat::Bmp,
            _ => return None,
        })
    }
}

/// Lower-case hex SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut s = String::with_capacity(64);
    for b in digest {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// An immutable analysis input. The original bytes are never modified.
pub struct ImageInput {
    name: String,
    bytes: Arc<[u8]>,
    sha256: String,
    format: FileFormat,
    decoded: OnceLock<std::result::Result<Arc<RgbImage>, String>>,
    working: Mutex<Option<(u64, Arc<RgbImage>)>>,
}

impl std::fmt::Debug for ImageInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageInput")
            .field("name", &self.name)
            .field("len", &self.bytes.len())
            .field("sha256", &self.sha256)
            .field("format", &self.format)
            .finish()
    }
}

impl ImageInput {
    /// Builds an input from in-memory bytes.
    pub fn from_bytes(name: impl Into<String>, bytes: impl Into<Arc<[u8]>>) -> Self {
        let bytes: Arc<[u8]> = bytes.into();
        Self {
            name: name.into(),
            sha256: sha256_hex(&bytes),
            format: FileFormat::sniff(&bytes),
            bytes,
            decoded: OnceLock::new(),
            working: Mutex::new(None),
        }
    }

    /// Reads a file (read-only) into memory.
    pub fn from_path(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        Ok(Self::from_bytes(name, bytes))
    }

    /// File name (no directory).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Original bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Shared handle to the original bytes.
    pub fn bytes_arc(&self) -> Arc<[u8]> {
        Arc::clone(&self.bytes)
    }

    /// SHA-256 of the original bytes (hex).
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    /// Sniffed container format.
    pub fn format(&self) -> FileFormat {
        self.format
    }

    /// Lower-case file extension, if any.
    pub fn extension(&self) -> Option<String> {
        Path::new(&self.name).extension().map(|e| e.to_string_lossy().to_ascii_lowercase())
    }

    /// True when the extension does not match the sniffed format.
    pub fn extension_mismatch(&self) -> bool {
        match (self.extension(), self.format) {
            (_, FileFormat::Unknown) | (None, _) => false,
            (Some(ext), fmt) => !fmt.extensions().contains(&ext.as_str()),
        }
    }

    /// Full-resolution decoded pixels (stored orientation, EXIF orientation *not* applied).
    pub fn rgb(&self) -> Result<Arc<RgbImage>> {
        self.decoded.get_or_init(|| self.decode().map(Arc::new)).clone().map_err(Error::Decode)
    }

    fn decode(&self) -> std::result::Result<RgbImage, String> {
        let fmt =
            self.format.image_format().ok_or_else(|| format!("pixel decoding not supported for {:?}", self.format))?;
        image::load_from_memory_with_format(&self.bytes, fmt).map(|img| img.to_rgb8()).map_err(|e| e.to_string())
    }

    /// Decoded pixels downscaled so that `width*height <= max_pixels` (0 = unlimited).
    /// Returns the image and the scale factor relative to the original.
    pub fn working_rgb(&self, max_pixels: u64) -> Result<(Arc<RgbImage>, f32)> {
        let full = self.rgb()?;
        let px = u64::from(full.width()) * u64::from(full.height());
        if max_pixels == 0 || px <= max_pixels {
            return Ok((full, 1.0));
        }
        let mut guard = self.working.lock().map_err(|_| Error::Decode("lock poisoned".into()))?;
        if let Some((mp, img)) = guard.as_ref() {
            if *mp == max_pixels {
                let s = img.width() as f32 / full.width() as f32;
                return Ok((Arc::clone(img), s));
            }
        }
        let scale = (max_pixels as f64 / px as f64).sqrt();
        let w = ((f64::from(full.width()) * scale).floor() as u32).max(1);
        let h = ((f64::from(full.height()) * scale).floor() as u32).max(1);
        let small = Arc::new(image::imageops::resize(full.as_ref(), w, h, FilterType::Triangle));
        *guard = Some((max_pixels, Arc::clone(&small)));
        Ok((small, w as f32 / full.width() as f32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffs_common_formats() {
        assert_eq!(FileFormat::sniff(&[0xFF, 0xD8, 0xFF, 0xE0]), FileFormat::Jpeg);
        assert_eq!(FileFormat::sniff(b"\x89PNG\r\n\x1a\n...."), FileFormat::Png);
        assert_eq!(FileFormat::sniff(b"RIFF\0\0\0\0WEBPVP8 "), FileFormat::Webp);
        assert_eq!(FileFormat::sniff(b"GIF89a"), FileFormat::Gif);
        assert_eq!(FileFormat::sniff(b"II*\0\x08\0\0\0"), FileFormat::Tiff);
        assert_eq!(FileFormat::sniff(b"\0\0\0\x18ftypheic\0\0\0\0mif1heic"), FileFormat::Heif);
        assert_eq!(FileFormat::sniff(b"\0\0\0\x1cftypavif\0\0\0\0avifmif1"), FileFormat::Avif);
        assert_eq!(FileFormat::sniff(b"hello world!"), FileFormat::Unknown);
        assert_eq!(FileFormat::sniff(b""), FileFormat::Unknown);
    }

    #[test]
    fn sha256_known_vector() {
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn extension_mismatch_detected() {
        let png = ImageInput::from_bytes("photo.jpg", b"\x89PNG\r\n\x1a\nxxxx".to_vec());
        assert!(png.extension_mismatch());
        let ok = ImageInput::from_bytes("photo.PNG", b"\x89PNG\r\n\x1a\nxxxx".to_vec());
        assert!(!ok.extension_mismatch());
    }

    #[test]
    fn working_image_is_downscaled() {
        let img = RgbImage::from_pixel(400, 300, image::Rgb([10, 20, 30]));
        let mut buf = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut buf, image::ImageFormat::Png)
            .unwrap_or_else(|e| panic!("{e}"));
        let input = ImageInput::from_bytes("a.png", buf.into_inner());
        let (w, s) = input.working_rgb(30_000).unwrap_or_else(|e| panic!("{e}"));
        assert!(u64::from(w.width()) * u64::from(w.height()) <= 30_000);
        assert!(s < 1.0);
        let (full, s1) = input.working_rgb(0).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(full.width(), 400);
        assert!((s1 - 1.0).abs() < f32::EPSILON);
    }
}
