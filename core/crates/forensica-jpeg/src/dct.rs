//! Baseline JPEG entropy decoder that recovers the *quantised DCT coefficients*
//! (which pixel decoders discard). Supports sequential Huffman (SOF0/SOF1), 8-bit,
//! interleaved or single-component scans, and restart intervals. Progressive and
//! arithmetic files return [`Error::Unsupported`].

use crate::quant::ZIGZAG;
use crate::structure::{parse, HuffTable, JpegInfo, Scan};
use forensica_core::{Error, Result};

/// Quantised coefficients of one component.
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentCoefficients {
    /// Component id.
    pub id: u8,
    /// Sampling factors.
    pub h: u8,
    /// Vertical sampling factor.
    pub v: u8,
    /// Blocks per row (padded to whole MCUs).
    pub blocks_w: usize,
    /// Block rows (padded to whole MCUs).
    pub blocks_h: usize,
    /// Blocks covering actual image samples (unpadded).
    pub used_w: usize,
    /// Block rows covering actual samples.
    pub used_h: usize,
    /// Quantisation table (natural order).
    pub qtable: [u16; 64],
    /// Quantised coefficients per block, natural order.
    pub blocks: Vec<[i16; 64]>,
}

impl ComponentCoefficients {
    /// Block at (bx, by).
    pub fn block(&self, bx: usize, by: usize) -> &[i16; 64] {
        &self.blocks[by * self.blocks_w + bx]
    }
}

/// All components.
#[derive(Debug, Clone, PartialEq)]
pub struct Coefficients {
    /// Image width.
    pub width: usize,
    /// Image height.
    pub height: usize,
    /// Components in frame order.
    pub components: Vec<ComponentCoefficients>,
}

struct Huff {
    maxcode: [i32; 18],
    valptr: [i32; 17],
    mincode: [i32; 17],
    symbols: Vec<u8>,
}

impl Huff {
    fn new(t: &HuffTable) -> Self {
        let mut h = Huff { maxcode: [-1; 18], valptr: [0; 17], mincode: [0; 17], symbols: t.symbols.clone() };
        let mut code = 0i32;
        let mut k = 0i32;
        for l in 1..=16 {
            let n = i32::from(t.counts[l - 1]);
            if n > 0 {
                h.valptr[l] = k;
                h.mincode[l] = code;
                code += n;
                k += n;
                h.maxcode[l] = code - 1;
            } else {
                h.maxcode[l] = -1;
            }
            code <<= 1;
        }
        h.maxcode[17] = i32::MAX;
        h
    }
}

struct Bits<'a> {
    d: &'a [u8],
    pos: usize,
    acc: u32,
    n: u32,
    hit_marker: bool,
}

impl<'a> Bits<'a> {
    fn new(d: &'a [u8]) -> Self {
        Self { d, pos: 0, acc: 0, n: 0, hit_marker: false }
    }

    fn fill(&mut self) {
        while self.n <= 24 {
            let mut byte = 0u8;
            if !self.hit_marker && self.pos < self.d.len() {
                let b = self.d[self.pos];
                if b == 0xFF {
                    let next = self.d.get(self.pos + 1).copied().unwrap_or(0);
                    if next == 0x00 {
                        byte = 0xFF;
                        self.pos += 2;
                    } else {
                        // A marker (RST or end): stop feeding real bits.
                        self.hit_marker = true;
                    }
                } else {
                    byte = b;
                    self.pos += 1;
                }
            }
            self.acc |= u32::from(byte) << (24 - self.n);
            self.n += 8;
        }
    }

    fn bit(&mut self) -> u32 {
        if self.n == 0 {
            self.fill();
        }
        let b = self.acc >> 31;
        self.acc <<= 1;
        self.n -= 1;
        b
    }

    fn bits(&mut self, k: u32) -> u32 {
        if k == 0 {
            return 0;
        }
        if self.n < k {
            self.fill();
        }
        let v = self.acc >> (32 - k);
        self.acc <<= k;
        self.n -= k;
        v
    }

    /// Skips to the byte after the next RSTn marker and resets the bit buffer.
    fn restart(&mut self) {
        self.acc = 0;
        self.n = 0;
        self.hit_marker = false;
        while self.pos + 1 < self.d.len() {
            if self.d[self.pos] == 0xFF && (0xD0..=0xD7).contains(&self.d[self.pos + 1]) {
                self.pos += 2;
                return;
            }
            self.pos += 1;
        }
    }

    fn decode(&mut self, h: &Huff) -> Result<u8> {
        let mut code = 0i32;
        for l in 1..=16usize {
            code = (code << 1) | self.bit() as i32;
            if code <= h.maxcode[l] {
                let idx = (h.valptr[l] + code - h.mincode[l]) as usize;
                return h.symbols.get(idx).copied().ok_or_else(|| Error::malformed("JPEG", "bad Huffman symbol"));
            }
        }
        Err(Error::malformed("JPEG", "invalid Huffman code"))
    }
}

fn extend(v: u32, t: u32) -> i32 {
    if t == 0 {
        return 0;
    }
    let v = v as i32;
    if v < (1 << (t - 1)) {
        v - (1 << t) + 1
    } else {
        v
    }
}

fn decode_block(bits: &mut Bits<'_>, dc: &Huff, ac: &Huff, pred: &mut i32, out: &mut [i16; 64]) -> Result<()> {
    let t = u32::from(bits.decode(dc)?);
    if t > 16 {
        return Err(Error::malformed("JPEG", "bad DC magnitude"));
    }
    let diff = extend(bits.bits(t), t);
    *pred += diff;
    out[0] = (*pred).clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
    let mut k = 1;
    while k < 64 {
        let rs = bits.decode(ac)?;
        let r = usize::from(rs >> 4);
        let s = u32::from(rs & 0x0F);
        if s == 0 {
            if r == 15 {
                k += 16;
                continue;
            }
            break;
        }
        k += r;
        if k > 63 {
            return Err(Error::malformed("JPEG", "AC index out of range"));
        }
        out[ZIGZAG[k]] = extend(bits.bits(s), s) as i16;
        k += 1;
    }
    Ok(())
}

fn table(tables: &[Option<Huff>; 8], class: usize, id: u8) -> Result<&Huff> {
    tables[class * 4 + usize::from(id & 3)].as_ref().ok_or_else(|| Error::malformed("JPEG", "missing Huffman table"))
}

/// Decodes the quantised DCT coefficients of a baseline JPEG.
pub fn decode(bytes: &[u8]) -> Result<Coefficients> {
    decode_info(bytes, &parse(bytes)?)
}

/// Same as [`decode`] with an already parsed structure.
pub fn decode_info(bytes: &[u8], info: &JpegInfo) -> Result<Coefficients> {
    let f = info.frame.as_ref().ok_or_else(|| Error::malformed("JPEG", "no frame header"))?;
    if !f.sequential_huffman() || f.precision != 8 {
        return Err(Error::Unsupported(format!("DCT decoding needs baseline 8-bit JPEG (this is {})", f.process())));
    }
    let (w, h) = (usize::from(f.width), usize::from(f.height));
    if w == 0 || h == 0 || f.components.is_empty() {
        return Err(Error::malformed("JPEG", "empty frame"));
    }
    let hmax = usize::from(f.components.iter().map(|c| c.h).max().unwrap_or(1).max(1));
    let vmax = usize::from(f.components.iter().map(|c| c.v).max().unwrap_or(1).max(1));
    let mcux = w.div_ceil(8 * hmax);
    let mcuy = h.div_ceil(8 * vmax);
    let mut comps: Vec<ComponentCoefficients> = f
        .components
        .iter()
        .map(|c| {
            let (ch, cv) = (usize::from(c.h.max(1)), usize::from(c.v.max(1)));
            let bw = mcux * ch;
            let bh = mcuy * cv;
            ComponentCoefficients {
                id: c.id,
                h: c.h,
                v: c.v,
                blocks_w: bw,
                blocks_h: bh,
                used_w: (w * ch).div_ceil(hmax).div_ceil(8),
                used_h: (h * cv).div_ceil(vmax).div_ceil(8),
                qtable: [1; 64],
                blocks: vec![[0i16; 64]; bw * bh],
            }
        })
        .collect();
    if info.scans.is_empty() {
        return Err(Error::malformed("JPEG", "no scan"));
    }
    for scan in &info.scans {
        decode_scan(bytes, scan, f, &mut comps, mcux, mcuy)?;
    }
    Ok(Coefficients { width: w, height: h, components: comps })
}

fn decode_scan(
    bytes: &[u8],
    scan: &Scan,
    f: &crate::structure::Frame,
    comps: &mut [ComponentCoefficients],
    mcux: usize,
    mcuy: usize,
) -> Result<()> {
    let huffs: [Option<Huff>; 8] = std::array::from_fn(|i| scan.huff[i].as_ref().map(Huff::new));
    let idx: Vec<usize> = scan
        .components
        .iter()
        .map(|(id, _, _)| {
            f.components
                .iter()
                .position(|c| c.id == *id)
                .ok_or_else(|| Error::malformed("JPEG", "scan component not in frame"))
        })
        .collect::<Result<_>>()?;
    for &ci in &idx {
        let tq = usize::from(f.components[ci].tq & 3);
        comps[ci].qtable = scan.quant[tq].ok_or_else(|| Error::malformed("JPEG", "missing quantisation table"))?;
    }
    let data = bytes.get(scan.entropy.clone()).unwrap_or_default();
    let mut bits = Bits::new(data);
    let mut preds = vec![0i32; idx.len()];
    let ri = usize::from(scan.restart_interval);
    let mut units = 0usize;
    let handle_restart = |bits: &mut Bits<'_>, preds: &mut Vec<i32>, units: &mut usize| {
        if ri > 0 && *units > 0 && (*units).is_multiple_of(ri) {
            bits.restart();
            preds.iter_mut().for_each(|p| *p = 0);
        }
    };
    if idx.len() == 1 {
        // Non-interleaved: iterate the component's own block grid.
        let ci = idx[0];
        let (_, td, ta) = scan.components[0];
        let (dc, ac) = (table(&huffs, 0, td)?, table(&huffs, 1, ta)?);
        let (uw, uh, bw) = (comps[ci].used_w, comps[ci].used_h, comps[ci].blocks_w);
        for by in 0..uh {
            for bx in 0..uw {
                handle_restart(&mut bits, &mut preds, &mut units);
                let blk = &mut comps[ci].blocks[by * bw + bx];
                decode_block(&mut bits, dc, ac, &mut preds[0], blk)?;
                units += 1;
            }
        }
        return Ok(());
    }
    for my in 0..mcuy {
        for mx in 0..mcux {
            handle_restart(&mut bits, &mut preds, &mut units);
            for (k, &ci) in idx.iter().enumerate() {
                let (_, td, ta) = scan.components[k];
                let (dc, ac) = (table(&huffs, 0, td)?, table(&huffs, 1, ta)?);
                let (ch, cv) = (usize::from(comps[ci].h.max(1)), usize::from(comps[ci].v.max(1)));
                let bw = comps[ci].blocks_w;
                for v in 0..cv {
                    for u in 0..ch {
                        let (bx, by) = (mx * ch + u, my * cv + v);
                        let blk = &mut comps[ci].blocks[by * bw + bx];
                        decode_block(&mut bits, dc, ac, &mut preds[k], blk)?;
                    }
                }
            }
            units += 1;
        }
    }
    Ok(())
}

/// Separable 8×8 inverse DCT of dequantised coefficients (natural order) → samples (level-shifted +128).
pub fn idct8x8(coef: &[f32; 64]) -> [f32; 64] {
    let mut c = [[0f32; 8]; 8];
    for (u, row) in c.iter_mut().enumerate() {
        for (x, v) in row.iter_mut().enumerate() {
            let a = if u == 0 { std::f32::consts::FRAC_1_SQRT_2 } else { 1.0 };
            *v = a * 0.5 * (((2 * x + 1) as f32 * u as f32 * std::f32::consts::PI) / 16.0).cos();
        }
    }
    let mut tmp = [0f32; 64];
    for y in 0..8 {
        for u in 0..8 {
            let mut s = 0.0;
            for v in 0..8 {
                s += c[v][y] * coef[v * 8 + u];
            }
            tmp[y * 8 + u] = s;
        }
    }
    let mut out = [0f32; 64];
    for y in 0..8 {
        for x in 0..8 {
            let mut s = 0.0;
            for u in 0..8 {
                s += c[u][x] * tmp[y * 8 + u];
            }
            out[y * 8 + x] = s + 128.0;
        }
    }
    out
}

/// Forward 8×8 DCT of level-shifted samples (input already minus 128), natural order.
pub fn fdct8x8(px: &[f32; 64]) -> [f32; 64] {
    let mut c = [[0f32; 8]; 8];
    for (u, row) in c.iter_mut().enumerate() {
        for (x, v) in row.iter_mut().enumerate() {
            let a = if u == 0 { std::f32::consts::FRAC_1_SQRT_2 } else { 1.0 };
            *v = a * 0.5 * (((2 * x + 1) as f32 * u as f32 * std::f32::consts::PI) / 16.0).cos();
        }
    }
    let mut tmp = [0f32; 64];
    for y in 0..8 {
        for u in 0..8 {
            tmp[y * 8 + u] = (0..8).map(|x| c[u][x] * px[y * 8 + x]).sum();
        }
    }
    let mut out = [0f32; 64];
    for v in 0..8 {
        for u in 0..8 {
            out[v * 8 + u] = (0..8).map(|y| c[v][y] * tmp[y * 8 + u]).sum();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil;

    #[test]
    fn reconstructs_luma_close_to_pixel_decoder() {
        let img = testutil::textured(96, 64);
        let j = testutil::encode(&img, 90);
        let c = decode(&j).unwrap();
        assert_eq!(c.components.len(), 3);
        let y = &c.components[0];
        let dec = image::load_from_memory(&j).unwrap().to_rgb8();
        let mut err = 0.0;
        let mut n = 0.0;
        for by in 0..y.used_h {
            for bx in 0..y.used_w {
                let b = y.block(bx, by);
                let mut deq = [0f32; 64];
                for k in 0..64 {
                    deq[k] = f32::from(b[k]) * f32::from(y.qtable[k]);
                }
                let px = idct8x8(&deq);
                for yy in 0..8 {
                    for xx in 0..8 {
                        let (x, yv) = (bx * 8 + xx, by * 8 + yy);
                        if x >= 96 || yv >= 64 {
                            continue;
                        }
                        let p = dec.get_pixel(x as u32, yv as u32).0;
                        let luma = 0.299 * f32::from(p[0]) + 0.587 * f32::from(p[1]) + 0.114 * f32::from(p[2]);
                        err += (luma - px[yy * 8 + xx].clamp(0.0, 255.0)).abs();
                        n += 1.0;
                    }
                }
            }
        }
        assert!(err / n < 3.0, "mean abs error {}", err / n);
    }

    #[test]
    fn dct_roundtrip() {
        let mut px = [0f32; 64];
        for (i, p) in px.iter_mut().enumerate() {
            *p = (i as f32 * 3.7).sin() * 50.0;
        }
        let back = idct8x8(&fdct8x8(&px));
        for i in 0..64 {
            assert!((back[i] - 128.0 - px[i]).abs() < 1e-3);
        }
    }

    #[test]
    fn grayscale_and_restart_markers() {
        // Grayscale JPEG via image crate.
        let g = image::GrayImage::from_fn(40, 24, |x, y| image::Luma([((x * 7 + y * 3) % 255) as u8]));
        let mut out = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 80).encode_image(&g).unwrap();
        let c = decode(&out).unwrap();
        assert_eq!(c.components.len(), 1);
        assert_eq!(c.components[0].used_w, 5);
    }

    #[test]
    fn rejects_garbage_without_panicking() {
        let j = testutil::jpeg(32, 32, 80);
        for cut in (100..j.len()).step_by(37) {
            let mut k = j[..cut].to_vec();
            k.extend_from_slice(&[0xFF, 0xD9]);
            let _ = decode(&k);
        }
        let mut k = j.clone();
        let mid = k.len() / 2;
        for b in &mut k[mid..mid + 20] {
            *b = 0x5A;
        }
        let _ = decode(&k);
    }
}
