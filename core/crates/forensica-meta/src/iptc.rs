//! IPTC-IIM (inside Photoshop Image Resource Blocks) and Photoshop IRB inventory.

use serde::Serialize;

use crate::table::Group;
use forensica_core::container::{find_all, jpeg_layout};

/// Parsed IPTC/IRB data.
#[derive(Debug, Clone, Default, Serialize)]
pub struct IptcData {
    /// IIM datasets as `(name, value)`.
    pub datasets: Vec<(String, String)>,
    /// Photoshop image-resource ids present (e.g. 0x0404 IPTC, 0x0409/0x040C thumbnail).
    pub resources: Vec<u16>,
    /// True if a Photoshop IRB block was found at all.
    pub photoshop_irb: bool,
}

impl IptcData {
    /// First value of a dataset by name.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.datasets.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    /// Display groups.
    pub fn groups(&self) -> Vec<Group> {
        let mut out = Vec::new();
        if !self.datasets.is_empty() {
            let mut g = Group::new("IPTC");
            for (k, v) in &self.datasets {
                g.push(k.clone(), v.clone());
            }
            out.push(g);
        }
        if self.photoshop_irb {
            let mut g = Group::new("Photoshop resources");
            for id in &self.resources {
                g.push(format!("0x{id:04X}"), resource_name(*id));
            }
            out.push(g);
        }
        out
    }
}

fn resource_name(id: u16) -> &'static str {
    match id {
        0x03ED => "Resolution info",
        0x0404 => "IPTC-NAA record",
        0x0406 => "JPEG quality (Photoshop Save As)",
        0x0409 | 0x040C => "Thumbnail",
        0x040A => "Copyright flag",
        0x040B => "URL",
        0x040F => "ICC profile",
        0x0421 => "Version info",
        0x0422 => "EXIF data 1",
        0x0424 => "XMP metadata",
        0x0425 => "IPTC digest (MD5)",
        0x0426 => "Print scale",
        0x0428 => "Pixel aspect ratio",
        0x043A => "Print information",
        0x043B => "Print style",
        0x2710 => "Print flags info",
        _ => "",
    }
}

fn dataset_name(record: u8, ds: u8) -> String {
    let n = match (record, ds) {
        (1, 90) => "CodedCharacterSet",
        (2, 0) => "RecordVersion",
        (2, 5) => "ObjectName",
        (2, 7) => "EditStatus",
        (2, 10) => "Urgency",
        (2, 15) => "Category",
        (2, 20) => "SupplementalCategories",
        (2, 25) => "Keywords",
        (2, 40) => "SpecialInstructions",
        (2, 55) => "DateCreated",
        (2, 60) => "TimeCreated",
        (2, 62) => "DigitalCreationDate",
        (2, 63) => "DigitalCreationTime",
        (2, 65) => "OriginatingProgram",
        (2, 70) => "ProgramVersion",
        (2, 80) => "By-line",
        (2, 85) => "By-lineTitle",
        (2, 90) => "City",
        (2, 92) => "Sub-location",
        (2, 95) => "Province-State",
        (2, 100) => "Country-PrimaryLocationCode",
        (2, 101) => "Country-PrimaryLocationName",
        (2, 103) => "OriginalTransmissionReference",
        (2, 105) => "Headline",
        (2, 110) => "Credit",
        (2, 115) => "Source",
        (2, 116) => "CopyrightNotice",
        (2, 118) => "Contact",
        (2, 120) => "Caption-Abstract",
        (2, 122) => "Writer-Editor",
        _ => return format!("{record}:{ds:03}"),
    };
    n.to_string()
}

/// Parses IIM datasets from an IPTC-NAA block.
pub fn parse_iim(d: &[u8]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 5 <= d.len() && d[i] == 0x1C {
        let (rec, ds) = (d[i + 1], d[i + 2]);
        let mut len = usize::from(u16::from_be_bytes([d[i + 3], d[i + 4]]));
        i += 5;
        if len & 0x8000 != 0 {
            // Extended dataset: next (len & 0x7FFF) bytes hold the length.
            let n = len & 0x7FFF;
            let Some(lb) = d.get(i..i + n) else { break };
            len = lb.iter().fold(0usize, |a, b| (a << 8) | usize::from(*b));
            i += n;
        }
        let Some(v) = d.get(i..i.saturating_add(len)) else { break };
        let value = if rec == 1 && ds == 90 {
            format!("{v:02X?}")
        } else {
            String::from_utf8_lossy(v).trim_end_matches('\0').to_string()
        };
        out.push((dataset_name(rec, ds), value));
        i += len;
    }
    out
}

/// Parses Photoshop IRB ("8BIM" blocks) and returns (resource ids, IPTC payload).
pub fn parse_irb(d: &[u8]) -> (Vec<u16>, Option<&[u8]>) {
    let mut ids = Vec::new();
    let mut iptc = None;
    let mut i = 0;
    while i + 12 <= d.len() && &d[i..i + 4] == b"8BIM" {
        let id = u16::from_be_bytes([d[i + 4], d[i + 5]]);
        let name_len = usize::from(d[i + 6]);
        // Pascal string padded to even total length (incl. length byte).
        let mut j = i + 7 + name_len;
        if (name_len + 1) % 2 == 1 {
            j += 1;
        }
        let Some(sz) = d.get(j..j + 4) else { break };
        let size = u32::from_be_bytes([sz[0], sz[1], sz[2], sz[3]]) as usize;
        j += 4;
        let Some(payload) = d.get(j..j.saturating_add(size)) else { break };
        ids.push(id);
        if id == 0x0404 {
            iptc = Some(payload);
        }
        i = j + size + (size & 1);
    }
    (ids, iptc)
}

/// Extracts IPTC from JPEG APP13, or by locating a "Photoshop 3.0" / "8BIM" block in other containers.
pub fn read(bytes: &[u8]) -> IptcData {
    let mut blocks: Vec<Vec<u8>> = Vec::new();
    if let Ok(layout) = jpeg_layout(bytes) {
        let parts = layout.app_payloads(bytes, 0xED, b"Photoshop 3.0\0");
        if !parts.is_empty() {
            // Multiple APP13 segments are concatenated.
            blocks.push(parts.concat());
        }
    } else if let Some(&start) = find_all(bytes, b"8BIM\x04\x04").first() {
        blocks.push(bytes[start..].to_vec());
    }
    let mut out = IptcData::default();
    for b in &blocks {
        out.photoshop_irb = true;
        let (ids, iptc) = parse_irb(b);
        out.resources.extend(ids);
        if let Some(p) = iptc {
            out.datasets.extend(parse_iim(p));
        }
    }
    out
}

#[cfg(test)]
pub(crate) fn sample_app13(datasets: &[(u8, &str)]) -> Vec<u8> {
    let mut iim = Vec::new();
    for (ds, v) in datasets {
        iim.extend_from_slice(&[0x1C, 2, *ds]);
        iim.extend_from_slice(&(v.len() as u16).to_be_bytes());
        iim.extend_from_slice(v.as_bytes());
    }
    let mut p = b"Photoshop 3.0\0".to_vec();
    p.extend_from_slice(b"8BIM\x04\x04\0\0");
    p.extend_from_slice(&(iim.len() as u32).to_be_bytes());
    p.extend_from_slice(&iim);
    if iim.len() % 2 == 1 {
        p.push(0);
    }
    p.extend_from_slice(b"8BIM\x04\x25\0\0\0\0\0\x10");
    p.extend_from_slice(&[0u8; 16]);
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exif_read::testutil::{insert_app, jpeg};

    #[test]
    fn reads_iptc_from_jpeg() {
        let app = sample_app13(&[(5, "Title"), (65, "Adobe Photoshop"), (116, "(c) me")]);
        let j = insert_app(&jpeg(8, 8, 90), 0xED, &app);
        let d = read(&j);
        assert!(d.photoshop_irb);
        assert_eq!(d.get("ObjectName"), Some("Title"));
        assert_eq!(d.get("OriginatingProgram"), Some("Adobe Photoshop"));
        assert_eq!(d.resources, vec![0x0404, 0x0425]);
        assert_eq!(d.groups().len(), 2);
    }

    #[test]
    fn garbage_is_safe() {
        assert!(parse_iim(&[0x1C, 2, 5, 0xFF, 0xFF]).is_empty());
        let (ids, _) = parse_irb(b"8BIM\x04\x04\x05ab");
        assert!(ids.is_empty());
    }
}
