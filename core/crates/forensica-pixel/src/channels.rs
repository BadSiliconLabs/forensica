//! Channel and bit-plane viewers (interactive).

use image::RgbImage;

use forensica_core::viz::{image_viz, percentile};
use forensica_core::{Error, Result, Visualization, VizKind};

fn hsv(p: [u8; 3]) -> (f32, f32) {
    let r = f32::from(p[0]) / 255.0;
    let g = f32::from(p[1]) / 255.0;
    let b = f32::from(p[2]) / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    } / 6.0;
    let s = if max == 0.0 { 0.0 } else { d / max };
    (h, s)
}

/// Extracts a named channel as 0..255 floats.
pub fn channel_values(img: &RgbImage, name: &str) -> Result<Vec<f32>> {
    let f = |p: &image::Rgb<u8>| -> f32 {
        let [r, g, b] = p.0.map(f32::from);
        match name {
            "red" => r,
            "green" => g,
            "blue" => b,
            "cb" => 128.0 - 0.168_736 * r - 0.331_264 * g + 0.5 * b,
            "cr" => 128.0 + 0.5 * r - 0.418_688 * g - 0.081_312 * b,
            "hue" => hsv(p.0).0 * 255.0,
            "saturation" => hsv(p.0).1 * 255.0,
            _ => 0.299 * r + 0.587 * g + 0.114 * b,
        }
    };
    if !matches!(name, "luma" | "red" | "green" | "blue" | "cb" | "cr" | "hue" | "saturation") {
        return Err(Error::InvalidArgument(format!("unknown channel {name}")));
    }
    Ok(img.pixels().map(f).collect())
}

/// Renders a channel with auto-contrast (0.5–99.5 percentile stretch).
pub fn render_channel(img: &RgbImage, name: &str) -> Result<Visualization> {
    let v = channel_values(img, name)?;
    let lo = percentile(&v, 0.005);
    let hi = percentile(&v, 0.995).max(lo + 1.0);
    let out = RgbImage::from_fn(img.width(), img.height(), |x, y| {
        let t = ((v[(y * img.width() + x) as usize] - lo) / (hi - lo)).clamp(0.0, 1.0);
        let g = (t * 255.0) as u8;
        image::Rgb([g, g, g])
    });
    image_viz(
        &format!("channel.{name}"),
        &format!("Channel: {name}"),
        "Single channel with automatic contrast stretch. Chroma channels (Cb/Cr) and saturation often reveal \
         retouching or compositing that is invisible in the colour image.",
        out,
        VizKind::Aligned,
    )
}

/// Renders one bit plane (0 = least significant) of a channel.
pub fn render_bitplane(img: &RgbImage, name: &str, bit: u8) -> Result<Visualization> {
    if bit > 7 {
        return Err(Error::InvalidArgument("bit must be 0..=7".into()));
    }
    let v = channel_values(img, name)?;
    let out = RgbImage::from_fn(img.width(), img.height(), |x, y| {
        let b = ((v[(y * img.width() + x) as usize].round().clamp(0.0, 255.0) as u8) >> bit) & 1;
        let g = if b == 1 { 255 } else { 0 };
        image::Rgb([g, g, g])
    });
    image_viz(
        &format!("bitplane.{name}.{bit}"),
        &format!("Bit plane {bit} ({name})"),
        "One bit of each pixel value. Low bit planes of camera images look like noise; structure, flat areas or \
         sharp-edged regions in low planes reveal editing, steganography or synthetic content.",
        out,
        VizKind::Aligned,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channels_and_bitplanes() {
        let img = RgbImage::from_fn(8, 8, |x, _| image::Rgb([(x * 30) as u8, 0, 255]));
        assert_eq!(channel_values(&img, "red").unwrap()[1], 30.0);
        assert!(channel_values(&img, "nope").is_err());
        assert!(render_channel(&img, "cb").is_ok());
        let v = render_bitplane(&img, "red", 1).unwrap();
        assert_eq!(v.width, 8);
        assert!(render_bitplane(&img, "red", 9).is_err());
    }
}
