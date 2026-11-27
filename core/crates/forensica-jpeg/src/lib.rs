//! JPEG-specific forensics: file structure, quantisation tables, DCT coefficients.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::sync::Arc;

use forensica_core::AnalysisModule;

pub mod dct;
pub mod double;
pub mod grid;
pub mod quant;
pub mod structure;

mod double_module;
mod structure_module;

pub use double_module::DoubleCompressionModule;
pub use structure_module::JpegStructureModule;

/// All modules provided by this crate.
pub fn modules() -> Vec<Arc<dyn AnalysisModule>> {
    vec![Arc::new(JpegStructureModule), Arc::new(DoubleCompressionModule)]
}

#[cfg(test)]
pub(crate) mod testutil {
    /// Encodes a deterministic textured test image as JPEG.
    pub fn jpeg(w: u32, h: u32, q: u8) -> Vec<u8> {
        encode(&textured(w, h), q)
    }

    /// A textured RGB image.
    pub fn textured(w: u32, h: u32) -> image::RgbImage {
        image::RgbImage::from_fn(w, h, |x, y| {
            let v = ((x as f32 * 0.31).sin() * 60.0 + (y as f32 * 0.17).cos() * 50.0 + ((x * y) % 37) as f32) as i32;
            image::Rgb([
                (120 + v).clamp(0, 255) as u8,
                (100 + v / 2).clamp(0, 255) as u8,
                (140 - v / 3).clamp(0, 255) as u8,
            ])
        })
    }

    /// Encodes RGB as JPEG.
    pub fn encode(img: &image::RgbImage, q: u8) -> Vec<u8> {
        let mut out = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, q).encode_image(img).unwrap();
        out
    }
}
