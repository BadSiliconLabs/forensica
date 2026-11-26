//! Best-effort MakerNote identification. MakerNotes are proprietary; we only
//! identify the vendor format and count IFD entries, which is enough to tell
//! whether a vendor block survived (editors often drop or corrupt it).

use serde::Serialize;

/// Summary of a MakerNote block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MakerNoteInfo {
    /// Detected vendor format (e.g. "Nikon type 3", "Apple iOS", "Canon (headerless IFD)").
    pub vendor: String,
    /// Size in bytes.
    pub size: usize,
    /// Number of IFD entries, if the IFD could be parsed.
    pub entries: Option<u16>,
    /// Whether the vendor format is consistent with the EXIF Make.
    pub consistent_with_make: Option<bool>,
}

struct Header {
    sig: &'static [u8],
    vendor: &'static str,
    make_hint: &'static str,
    /// Offset of the IFD from the start of the note.
    ifd: usize,
    /// Byte order if fixed (true = little endian); None = TIFF header present at `ifd - 8`.
    le: Option<bool>,
}

const HEADERS: &[Header] = &[
    Header { sig: b"Nikon\0\x02", vendor: "Nikon type 3", make_hint: "nikon", ifd: 18, le: None },
    Header { sig: b"Nikon\0\x01", vendor: "Nikon type 1", make_hint: "nikon", ifd: 8, le: Some(false) },
    Header { sig: b"OLYMPUS\0II", vendor: "Olympus (new)", make_hint: "olympus", ifd: 12, le: Some(true) },
    Header { sig: b"OM SYSTEM\0", vendor: "OM System", make_hint: "om", ifd: 16, le: Some(true) },
    Header { sig: b"OLYMP\0", vendor: "Olympus (old)", make_hint: "olympus", ifd: 8, le: Some(true) },
    Header { sig: b"FUJIFILM", vendor: "Fujifilm", make_hint: "fuji", ifd: 12, le: Some(true) },
    Header { sig: b"Panasonic\0\0\0", vendor: "Panasonic", make_hint: "panasonic", ifd: 12, le: Some(true) },
    Header { sig: b"SONY DSC \0\0\0", vendor: "Sony", make_hint: "sony", ifd: 12, le: Some(true) },
    Header { sig: b"SONY CAM \0\0\0", vendor: "Sony", make_hint: "sony", ifd: 12, le: Some(true) },
    Header { sig: b"Apple iOS\0", vendor: "Apple iOS", make_hint: "apple", ifd: 14, le: Some(false) },
    Header { sig: b"PENTAX \0", vendor: "Pentax", make_hint: "pentax", ifd: 10, le: None },
    Header { sig: b"AOC\0", vendor: "Pentax (AOC)", make_hint: "pentax", ifd: 6, le: Some(false) },
    Header { sig: b"LEICA", vendor: "Leica", make_hint: "leica", ifd: 8, le: Some(true) },
    Header { sig: b"SIGMA\0\0\0", vendor: "Sigma", make_hint: "sigma", ifd: 10, le: Some(true) },
    Header { sig: b"Ricoh", vendor: "Ricoh", make_hint: "ricoh", ifd: 8, le: Some(false) },
    Header { sig: b"QVC\0", vendor: "Casio type 2", make_hint: "casio", ifd: 6, le: Some(false) },
];

fn read_u16(d: &[u8], at: usize, le: bool) -> Option<u16> {
    let b = d.get(at..at + 2)?;
    Some(if le { u16::from_le_bytes([b[0], b[1]]) } else { u16::from_be_bytes([b[0], b[1]]) })
}

fn plausible_ifd(d: &[u8], at: usize, le: bool) -> Option<u16> {
    let n = read_u16(d, at, le)?;
    // Each entry is 12 bytes; reject absurd counts.
    (n > 0 && n < 600 && at + 2 + usize::from(n) * 12 <= d.len()).then_some(n)
}

/// Inspects a MakerNote payload.
pub fn inspect(data: &[u8], make: Option<&str>) -> MakerNoteInfo {
    let make_l = make.map(str::to_ascii_lowercase);
    for h in HEADERS {
        if data.starts_with(h.sig) {
            let entries = match h.le {
                Some(le) => plausible_ifd(data, h.ifd, le),
                None => {
                    let tiff = h.ifd.saturating_sub(8);
                    let le = data.get(tiff..tiff + 2) == Some(b"II");
                    plausible_ifd(data, h.ifd, le)
                }
            };
            return MakerNoteInfo {
                vendor: h.vendor.to_string(),
                size: data.len(),
                entries,
                consistent_with_make: make_l.as_ref().map(|m| m.contains(h.make_hint)),
            };
        }
    }
    // Headerless IFD (Canon, Samsung, Kodak, many phones).
    let entries = plausible_ifd(data, 0, true).or_else(|| plausible_ifd(data, 0, false));
    let vendor = match make_l.as_deref() {
        Some(m) if m.contains("canon") => "Canon (headerless IFD)",
        Some(_) if entries.is_some() => "Headerless IFD",
        _ => "Unknown / proprietary",
    };
    MakerNoteInfo { vendor: vendor.to_string(), size: data.len(), entries, consistent_with_make: None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_nikon_and_apple() {
        let mut nikon = b"Nikon\0\x02\x10\0\0MM\0\x2a\0\0\0\x08".to_vec();
        nikon.extend_from_slice(&[0, 1]);
        nikon.extend_from_slice(&[0u8; 12]);
        let i = inspect(&nikon, Some("NIKON CORPORATION"));
        assert_eq!(i.vendor, "Nikon type 3");
        assert_eq!(i.entries, Some(1));
        assert_eq!(i.consistent_with_make, Some(true));

        let apple = b"Apple iOS\0\0\x01MM\0\x02".iter().copied().chain([0u8; 24]).collect::<Vec<_>>();
        let a = inspect(&apple, Some("Canon"));
        assert_eq!(a.vendor, "Apple iOS");
        assert_eq!(a.entries, Some(2));
        assert_eq!(a.consistent_with_make, Some(false));
    }

    #[test]
    fn headerless_and_garbage() {
        let mut canon = vec![2u8, 0];
        canon.extend_from_slice(&[0u8; 24]);
        assert_eq!(inspect(&canon, Some("Canon")).entries, Some(2));
        let g = inspect(&[0xFF; 3], None);
        assert_eq!(g.vendor, "Unknown / proprietary");
        assert_eq!(g.entries, None);
    }
}
