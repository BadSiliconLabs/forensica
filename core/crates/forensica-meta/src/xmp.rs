//! XMP packet discovery (container-agnostic) and flattening into properties.

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use serde::Serialize;

use crate::table::Group;
use forensica_core::container::{find_all, jpeg_layout};

/// One `xmpMM:History` event.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct HistoryEvent {
    /// `stEvt:action` (created, saved, converted, derived, ...).
    pub action: String,
    /// `stEvt:when`.
    pub when: String,
    /// `stEvt:softwareAgent`.
    pub software: String,
    /// `stEvt:changed`.
    pub changed: String,
    /// `stEvt:parameters`.
    pub parameters: String,
}

/// Parsed XMP.
#[derive(Debug, Clone, Default, Serialize)]
pub struct XmpData {
    /// Number of packets found.
    pub packets: usize,
    /// Flattened `(path, value)` properties.
    pub properties: Vec<(String, String)>,
    /// Edit history events.
    pub history: Vec<HistoryEvent>,
    /// Packets that failed to parse.
    pub parse_errors: usize,
}

impl XmpData {
    /// First value of a property path (exact match).
    pub fn get(&self, path: &str) -> Option<&str> {
        self.properties.iter().find(|(k, _)| k == path).map(|(_, v)| v.as_str())
    }

    /// All properties whose path starts with `prefix` (namespace prefix like `crs:`).
    pub fn count_prefix(&self, prefix: &str) -> usize {
        self.properties.iter().filter(|(k, _)| k.starts_with(prefix)).count()
    }

    /// Values of any property whose final component equals `name` (e.g. `dc:description`).
    pub fn values_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.properties
            .iter()
            .filter(move |(k, _)| k == name || k.starts_with(&format!("{name}[")))
            .map(|(_, v)| v.as_str())
    }

    /// Display group.
    pub fn group(&self) -> Group {
        let mut g = Group::new("XMP");
        for (k, v) in &self.properties {
            g.push(k.clone(), v.clone());
        }
        g
    }
}

/// Finds XMP packets anywhere in the file, plus reassembled JPEG extended XMP.
pub fn extract_packets(bytes: &[u8]) -> Vec<String> {
    let mut out = packets_in(bytes);
    if let Some(ext) = jpeg_extended_xmp(bytes) {
        out.extend(packets_in(&ext));
    }
    out
}

fn packets_in(bytes: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for (open, close) in [(&b"<x:xmpmeta"[..], &b"</x:xmpmeta>"[..]), (b"<x:xapmeta", b"</x:xapmeta>")] {
        for start in find_all(bytes, open) {
            if let Some(len) = find_all(&bytes[start..], close).first() {
                let end = start + len + close.len();
                out.push(String::from_utf8_lossy(&bytes[start..end]).into_owned());
            }
        }
    }
    if out.is_empty() {
        // Bare rdf:RDF packets (some PNG writers).
        for start in find_all(bytes, b"<rdf:RDF") {
            if let Some(len) = find_all(&bytes[start..], b"</rdf:RDF>").first() {
                let end = start + len + 10;
                out.push(String::from_utf8_lossy(&bytes[start..end]).into_owned());
            }
        }
    }
    out
}

fn jpeg_extended_xmp(bytes: &[u8]) -> Option<Vec<u8>> {
    const SIG: &[u8] = b"http://ns.adobe.com/xmp/extension/\0";
    let layout = jpeg_layout(bytes).ok()?;
    let mut parts: Vec<(u32, &[u8])> = layout
        .app_payloads(bytes, 0xE1, SIG)
        .into_iter()
        .filter(|p| p.len() > 40)
        .map(|p| (u32::from_be_bytes([p[36], p[37], p[38], p[39]]), &p[40..]))
        .collect();
    if parts.is_empty() {
        return None;
    }
    parts.sort_by_key(|p| p.0);
    Some(parts.into_iter().flat_map(|p| p.1.iter().copied()).collect())
}

const STRUCTURAL: &[&str] = &["x:xmpmeta", "x:xapmeta", "rdf:RDF", "rdf:Description", "rdf:Seq", "rdf:Bag", "rdf:Alt"];

/// Parses all packets into one [`XmpData`].
pub fn parse(bytes: &[u8]) -> XmpData {
    let mut data = XmpData::default();
    for p in extract_packets(bytes) {
        data.packets += 1;
        if flatten(&p, &mut data.properties).is_err() {
            data.parse_errors += 1;
        }
    }
    data.history = history(&data.properties);
    data
}

#[derive(Default)]
struct Frame {
    label: String,
    li_count: usize,
}

fn path_of(stack: &[Frame]) -> String {
    let mut s = String::new();
    for f in stack {
        if f.label.is_empty() {
            continue;
        }
        if f.label.starts_with('[') || s.is_empty() {
            s.push_str(&f.label);
        } else {
            s.push('/');
            s.push_str(&f.label);
        }
    }
    s
}

fn open_frame(stack: &mut Vec<Frame>, e: &BytesStart<'_>, props: &mut Vec<(String, String)>) {
    let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
    let label = if name == "rdf:li" {
        let n = stack.last_mut().map(|f| {
            f.li_count += 1;
            f.li_count
        });
        // li counters live on the nearest container; walk up if the parent is rdf:Seq etc.
        format!("[{}]", n.unwrap_or(1))
    } else if STRUCTURAL.contains(&name.as_str()) {
        String::new()
    } else {
        name
    };
    stack.push(Frame { label, li_count: 0 });
    let base = path_of(stack);
    for a in e.attributes().flatten() {
        let k = String::from_utf8_lossy(a.key.as_ref()).into_owned();
        if k.starts_with("xmlns") || k.starts_with("rdf:") || k.starts_with("xml:") || k == "x:xmptk" {
            if k == "rdf:resource" {
                if let Ok(v) = a.unescape_value() {
                    props.push((base.clone(), v.into_owned()));
                }
            }
            continue;
        }
        let v = a.unescape_value().map(|v| v.into_owned()).unwrap_or_default();
        let path = if base.is_empty() { k } else { format!("{base}/{k}") };
        props.push((path, v));
    }
}

/// Collapses li counters: an `rdf:li` frame must count siblings on its container,
/// which is the *structural* parent (rdf:Seq) — kept on that frame by `open_frame`.
fn flatten(packet: &str, props: &mut Vec<(String, String)>) -> Result<(), quick_xml::Error> {
    let mut r = Reader::from_str(packet);
    r.config_mut().trim_text(true);
    let mut stack: Vec<Frame> = Vec::new();
    loop {
        match r.read_event()? {
            Event::Start(e) => open_frame(&mut stack, &e, props),
            Event::Empty(e) => {
                open_frame(&mut stack, &e, props);
                stack.pop();
            }
            Event::End(_) => {
                stack.pop();
            }
            Event::Text(t) => {
                let v = t.unescape().map(|c| c.into_owned()).unwrap_or_default();
                if !v.trim().is_empty() {
                    props.push((path_of(&stack), v));
                }
            }
            Event::CData(t) => {
                let v = String::from_utf8_lossy(&t).into_owned();
                props.push((path_of(&stack), v));
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(())
}

fn history(props: &[(String, String)]) -> Vec<HistoryEvent> {
    let mut events: Vec<(usize, HistoryEvent)> = Vec::new();
    for (k, v) in props {
        let Some(rest) = k.strip_prefix("xmpMM:History[") else { continue };
        let Some((idx, field)) = rest.split_once(']') else { continue };
        let Ok(idx) = idx.parse::<usize>() else { continue };
        let field = field.trim_start_matches('/');
        let ev = match events.iter_mut().find(|(i, _)| *i == idx) {
            Some((_, e)) => e,
            None => {
                events.push((idx, HistoryEvent::default()));
                let last = events.len() - 1;
                &mut events[last].1
            }
        };
        match field {
            "stEvt:action" => ev.action = v.clone(),
            "stEvt:when" => ev.when = v.clone(),
            "stEvt:softwareAgent" => ev.software = v.clone(),
            "stEvt:changed" => ev.changed = v.clone(),
            "stEvt:parameters" => ev.parameters = v.clone(),
            _ => {}
        }
    }
    events.sort_by_key(|e| e.0);
    events.into_iter().map(|e| e.1).collect()
}

#[cfg(test)]
pub(crate) const SAMPLE: &str = r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Adobe XMP Core 7.0">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:xmp="http://ns.adobe.com/xap/1.0/"
    xmlns:xmpMM="http://ns.adobe.com/xap/1.0/mm/"
    xmlns:stEvt="http://ns.adobe.com/xap/1.0/sType/ResourceEvent#"
    xmlns:dc="http://purl.org/dc/elements/1.1/"
    xmlns:photoshop="http://ns.adobe.com/photoshop/1.0/"
    xmp:CreatorTool="Adobe Photoshop 25.0 (Windows)"
    xmp:CreateDate="2023-01-01T10:00:00"
    xmp:ModifyDate="2023-03-05T12:00:00+01:00"
    xmpMM:DocumentID="xmp.did:abc"
    xmpMM:InstanceID="xmp.iid:def">
   <dc:description><rdf:Alt><rdf:li xml:lang="x-default">A &amp; B</rdf:li></rdf:Alt></dc:description>
   <dc:subject><rdf:Bag><rdf:li>one</rdf:li><rdf:li>two</rdf:li></rdf:Bag></dc:subject>
   <photoshop:DocumentAncestors><rdf:Bag><rdf:li>xmp.did:1</rdf:li><rdf:li>xmp.did:2</rdf:li></rdf:Bag></photoshop:DocumentAncestors>
   <xmpMM:History>
    <rdf:Seq>
     <rdf:li stEvt:action="created" stEvt:when="2023-01-01T10:00:00" stEvt:softwareAgent="Adobe Photoshop 25.0"/>
     <rdf:li rdf:parseType="Resource">
      <stEvt:action>saved</stEvt:action>
      <stEvt:softwareAgent>Adobe Photoshop 25.0</stEvt:softwareAgent>
      <stEvt:changed>/</stEvt:changed>
     </rdf:li>
    </rdf:Seq>
   </xmpMM:History>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sample_packet() {
        let mut bytes = b"garbage before".to_vec();
        bytes.extend_from_slice(SAMPLE.as_bytes());
        bytes.extend_from_slice(b"after");
        let x = parse(&bytes);
        assert_eq!(x.packets, 1);
        assert_eq!(x.parse_errors, 0);
        assert_eq!(x.get("xmp:CreatorTool"), Some("Adobe Photoshop 25.0 (Windows)"));
        assert_eq!(x.get("dc:description[1]"), Some("A & B"));
        assert_eq!(x.get("dc:subject[2]"), Some("two"));
        assert_eq!(x.count_prefix("photoshop:DocumentAncestors"), 2);
        assert_eq!(x.history.len(), 2);
        assert_eq!(x.history[0].action, "created");
        assert_eq!(x.history[1].action, "saved");
        assert_eq!(x.history[1].changed, "/");
        assert_eq!(x.values_named("dc:subject").count(), 2);
    }

    #[test]
    fn broken_xml_counts_error() {
        let x = parse(b"<x:xmpmeta><rdf:RDF><a></b></x:xmpmeta>");
        assert_eq!(x.packets, 1);
        assert_eq!(x.parse_errors, 1);
        assert!(parse(b"no xmp").properties.is_empty());
    }
}
