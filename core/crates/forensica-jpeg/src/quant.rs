//! Quantisation tables: zig-zag order, IJG quality scaling, quality estimation and fingerprints.

use serde::Serialize;

/// Zig-zag index → natural (row-major) index.
pub const ZIGZAG: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20, 13, 6, 7, 14, 21,
    28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59, 52, 45, 38, 31, 39, 46, 53, 60, 61, 54,
    47, 55, 62, 63,
];

/// ITU-T T.81 Annex K luminance table (natural order).
pub const STD_LUMA: [u16; 64] = [
    16, 11, 10, 16, 24, 40, 51, 61, 12, 12, 14, 19, 26, 58, 60, 55, 14, 13, 16, 24, 40, 57, 69, 56, 14, 17, 22, 29, 51,
    87, 80, 62, 18, 22, 37, 56, 68, 109, 103, 77, 24, 35, 55, 64, 81, 104, 113, 92, 49, 64, 78, 87, 103, 121, 120, 101,
    72, 92, 95, 98, 112, 100, 103, 99,
];

/// ITU-T T.81 Annex K chrominance table (natural order).
pub const STD_CHROMA: [u16; 64] = [
    17, 18, 24, 47, 99, 99, 99, 99, 18, 21, 26, 66, 99, 99, 99, 99, 24, 26, 56, 99, 99, 99, 99, 99, 47, 66, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99,
];

/// IJG (libjpeg) scaling of a base table for `quality` in 1..=100.
pub fn ijg_table(base: &[u16; 64], quality: u8) -> [u16; 64] {
    let q = u32::from(quality.clamp(1, 100));
    let scale = if q < 50 { 5000 / q } else { 200 - 2 * q };
    let mut out = [0u16; 64];
    for (o, b) in out.iter_mut().zip(base) {
        *o = ((u32::from(*b) * scale + 50) / 100).clamp(1, 255) as u16;
    }
    out
}

/// Result of comparing a table with the IJG family.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct QualityEstimate {
    /// Closest IJG quality.
    pub quality: u8,
    /// Whether the table equals the IJG table exactly.
    pub exact_ijg: bool,
    /// Mean absolute difference to the closest IJG table.
    pub error: f32,
}

/// Estimates the IJG-equivalent quality of a table (natural order).
pub fn estimate(table: &[u16; 64], chroma: bool) -> QualityEstimate {
    let base = if chroma { &STD_CHROMA } else { &STD_LUMA };
    let mut best = QualityEstimate { quality: 50, exact_ijg: false, error: f32::MAX };
    for q in 1..=100u8 {
        let t = ijg_table(base, q);
        let err = t.iter().zip(table).map(|(a, b)| (f32::from(*a) - f32::from(*b)).abs()).sum::<f32>() / 64.0;
        if err < best.error {
            best = QualityEstimate { quality: q, exact_ijg: err == 0.0, error: err };
        }
    }
    best
}

/// A known encoder fingerprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Fingerprint {
    /// Encoder family / software.
    pub name: &'static str,
    /// Notes.
    pub note: &'static str,
}

/// Classifies the luma/chroma tables against the built-in fingerprint knowledge.
///
/// The built-in table currently knows the IJG/libjpeg family (computed for every quality) and a few
/// structural signatures (all-ones tables). Camera-specific tables are *not* shipped because no
/// verified, redistributable dataset was available offline; see `docs/modules/jpeg-structure.md`.
pub fn fingerprint(luma: Option<&[u16; 64]>, chroma: Option<&[u16; 64]>) -> Option<Fingerprint> {
    let l = luma?;
    if l.iter().all(|v| *v == 1) {
        return Some(Fingerprint {
            name: "Lossless-like (all ones)",
            note: "Quality 100 tables, typical of 'maximum quality' exports",
        });
    }
    let el = estimate(l, false);
    let ec = chroma.map(|c| estimate(c, true));
    let chroma_ok = ec.is_none_or(|e| e.exact_ijg && e.quality == el.quality);
    if el.exact_ijg && chroma_ok {
        return Some(Fingerprint {
            name: "IJG / libjpeg standard tables",
            note: "Used by libjpeg(-turbo) and software built on it: GIMP, ImageMagick, Pillow, browsers, many apps and some phones",
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ijg_scaling_matches_reference_points() {
        assert_eq!(ijg_table(&STD_LUMA, 50), STD_LUMA);
        assert!(ijg_table(&STD_LUMA, 100).iter().all(|v| *v == 1));
        assert_eq!(ijg_table(&STD_LUMA, 75)[0], 8);
    }

    #[test]
    fn estimates_quality() {
        for q in [30u8, 60, 75, 90, 95] {
            let e = estimate(&ijg_table(&STD_LUMA, q), false);
            assert!(e.exact_ijg);
            // Some neighbouring qualities produce identical tables; accept ±1.
            assert!(e.quality.abs_diff(q) <= 1, "{q} -> {}", e.quality);
        }
        let mut custom = ijg_table(&STD_LUMA, 80);
        custom[5] += 3;
        let e = estimate(&custom, false);
        assert!(!e.exact_ijg);
        assert!(e.quality.abs_diff(80) <= 2);
        assert!(fingerprint(Some(&ijg_table(&STD_LUMA, 80)), Some(&ijg_table(&STD_CHROMA, 80))).is_some());
        assert!(fingerprint(Some(&custom), None).is_none());
    }

    #[test]
    fn zigzag_is_permutation() {
        let mut z = ZIGZAG.to_vec();
        z.sort_unstable();
        assert_eq!(z, (0..64).collect::<Vec<_>>());
    }
}
