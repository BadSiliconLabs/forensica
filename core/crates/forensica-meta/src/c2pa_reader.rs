//! C2PA / Content Credentials reading and validation (offline).
//!
//! Uses the `c2pa` crate compiled without any HTTP client: remote manifests and
//! OCSP are never fetched. Trust lists are not bundled, so signer trust is
//! reported as "not verified against a trust list" rather than trusted/untrusted.

use serde::Serialize;
use serde_json::Value;

use crate::generator;
use crate::table::Group;
use forensica_core::{FileFormat, Finding, Severity};

/// Module id used for findings.
pub const MODULE: &str = "provenance";

/// One validation status entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Status {
    /// C2PA status code.
    pub code: String,
    /// Explanation from the SDK.
    pub explanation: String,
}

/// Summary of the active manifest.
#[derive(Debug, Clone, Default, Serialize)]
pub struct C2paSummary {
    /// Whether a manifest store was found.
    pub present: bool,
    /// Claim generator.
    pub claim_generator: String,
    /// Signer issuer.
    pub issuer: String,
    /// Signer common name.
    pub signer: String,
    /// Signing time.
    pub signed_at: String,
    /// Actions in the active manifest.
    pub actions: Vec<String>,
    /// Digital source types declared in actions.
    pub source_types: Vec<String>,
    /// Digital source types declared by *earlier* manifests in the provenance chain, with the action description.
    pub history_source_types: Vec<(String, String)>,
    /// Action descriptions from every manifest (e.g. "Created by Google Generative AI.").
    pub descriptions: Vec<String>,
    /// Ingredient titles.
    pub ingredients: Vec<String>,
    /// Number of manifests in the store.
    pub manifest_count: usize,
    /// Failure codes (excluding trust).
    pub failures: Vec<Status>,
    /// Whether the signer was untrusted (no trust list offline).
    pub untrusted_signer: bool,
    /// Error message when the store is present but unreadable.
    pub error: Option<String>,
    /// Full SDK JSON report.
    #[serde(skip)]
    pub json: Value,
}

/// Cheap pre-check for a JUMBF/C2PA store, to avoid invoking the SDK on every file.
pub fn maybe_has_c2pa(bytes: &[u8]) -> bool {
    !forensica_core::container::find_all(bytes, b"c2pa").is_empty()
        && !forensica_core::container::find_all(bytes, b"jumb").is_empty()
}

/// Reads and validates C2PA data.
#[cfg(feature = "c2pa")]
pub fn read(bytes: &[u8], format: FileFormat) -> C2paSummary {
    if !maybe_has_c2pa(bytes) {
        return C2paSummary::default();
    }
    let reader = c2pa::Reader::default().with_stream(format.mime(), std::io::Cursor::new(bytes));
    match reader {
        Ok(r) => {
            let json: Value = serde_json::from_str(&r.json()).unwrap_or(Value::Null);
            summarize(json)
        }
        Err(c2pa::Error::JumbfNotFound) => C2paSummary::default(),
        Err(e) => C2paSummary { present: true, error: Some(e.to_string()), ..Default::default() },
    }
}

/// Fallback when built without the `c2pa` feature.
#[cfg(not(feature = "c2pa"))]
pub fn read(bytes: &[u8], _format: FileFormat) -> C2paSummary {
    C2paSummary {
        present: maybe_has_c2pa(bytes),
        error: maybe_has_c2pa(bytes).then(|| "C2PA support not compiled in".to_string()),
        ..Default::default()
    }
}

fn s(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_string()
}

fn statuses(v: &Value) -> Vec<Status> {
    v.as_array()
        .map(|a| a.iter().map(|x| Status { code: s(&x["code"]), explanation: s(&x["explanation"]) }).collect())
        .unwrap_or_default()
}

/// Builds a summary from the SDK JSON.
pub fn summarize(json: Value) -> C2paSummary {
    let mut out = C2paSummary { present: true, ..Default::default() };
    let active = s(&json["active_manifest"]);
    let manifests = &json["manifests"];
    out.manifest_count = manifests.as_object().map_or(0, serde_json::Map::len);
    let m = &manifests[&active];
    out.claim_generator = s(&m["claim_generator"]);
    if out.claim_generator.is_empty() {
        out.claim_generator = m["claim_generator_info"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|g| format!("{} {}", s(&g["name"]), s(&g["version"])).trim().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
    }
    out.issuer = s(&m["signature_info"]["issuer"]);
    out.signer = s(&m["signature_info"]["common_name"]);
    out.signed_at = s(&m["signature_info"]["time"]);
    if let Some(assertions) = m["assertions"].as_array() {
        for a in assertions {
            if s(&a["label"]).starts_with("c2pa.actions") {
                for act in a["data"]["actions"].as_array().into_iter().flatten() {
                    out.actions.push(s(&act["action"]));
                    let dst = s(&act["digitalSourceType"]);
                    if !dst.is_empty() {
                        out.source_types.push(dst);
                    }
                }
            }
        }
    }
    // Walk the whole provenance chain, not only the active manifest: an AI generation step is often
    // recorded in an ingredient manifest while the latest one only says "opened / transcoded".
    if let Some(all) = manifests.as_object() {
        for (label, man) in all {
            for a in man["assertions"].as_array().into_iter().flatten() {
                if !s(&a["label"]).starts_with("c2pa.actions") {
                    continue;
                }
                for act in a["data"]["actions"].as_array().into_iter().flatten() {
                    let desc = s(&act["description"]);
                    if !desc.is_empty() && !out.descriptions.contains(&desc) {
                        out.descriptions.push(desc.clone());
                    }
                    let dst = s(&act["digitalSourceType"]);
                    if !dst.is_empty() && *label != active {
                        let what = if desc.is_empty() { s(&act["action"]) } else { desc };
                        out.history_source_types.push((dst, what));
                    }
                }
            }
        }
    }
    out.ingredients =
        m["ingredients"].as_array().map(|a| a.iter().map(|i| s(&i["title"])).collect()).unwrap_or_default();
    let mut all = statuses(&json["validation_status"]);
    all.extend(statuses(&json["validation_results"]["activeManifest"]["failure"]));
    for st in all {
        if st.code.contains("untrusted") || st.code.contains("Untrusted") {
            out.untrusted_signer = true;
        } else if !out.failures.iter().any(|f| f.code == st.code) {
            out.failures.push(st);
        }
    }
    out.json = json;
    out
}

/// Display group.
pub fn group(c: &C2paSummary) -> Option<Group> {
    if !c.present {
        return None;
    }
    let mut g = Group::new("C2PA / Content Credentials");
    g.push("Claim generator", c.claim_generator.clone());
    g.push("Signer", format!("{} ({})", c.signer, c.issuer));
    g.push("Signed at", c.signed_at.clone());
    g.push("Manifests", c.manifest_count.to_string());
    g.push("Actions", c.actions.join(", "));
    if !c.ingredients.is_empty() {
        g.push("Ingredients", c.ingredients.join(", "));
    }
    let v = if c.failures.is_empty() {
        "no integrity failures".to_string()
    } else {
        c.failures.iter().map(|f| f.code.clone()).collect::<Vec<_>>().join(", ")
    };
    g.push("Validation", v);
    if let Some(e) = &c.error {
        g.push("Error", e.clone());
    }
    Some(g)
}

/// Findings for a C2PA summary.
pub fn findings(c: &C2paSummary) -> Vec<Finding> {
    let mut out = Vec::new();
    if !c.present {
        out.push(
            Finding::new(MODULE, "c2pa.absent", Severity::Info, 1.0, "No Content Credentials (C2PA)")
                .explain("The file carries no C2PA manifest. Most images today have none.")
                .limits("Absence is normal and says nothing about authenticity; manifests are also stripped by most platforms."),
        );
        return out;
    }
    if let Some(e) = &c.error {
        out.push(
            Finding::new(
                MODULE,
                "c2pa.unreadable",
                Severity::Medium,
                0.7,
                "Content Credentials present but unreadable",
            )
            .explain(format!("A C2PA manifest store seems present but could not be parsed: {e}"))
            .limits("Could be a newer C2PA version than this tool supports, or a damaged file."),
        );
        return out;
    }
    if c.failures.is_empty() {
        out.push(
            Finding::new(MODULE, "c2pa.valid", Severity::Info, 0.9, format!("Content Credentials intact (by {})", c.claim_generator))
                .explain("A C2PA manifest is present and its hashes and signature validate: the file has not changed since it was signed.")
                .limits("Validation proves integrity since signing, not truthfulness of the content. Signer identity is NOT checked against a trust list in offline mode.")
                .with("signer", format!("{} / {}", c.signer, c.issuer))
                .with("signed_at", &c.signed_at)
                .with("actions", c.actions.join(", ")),
        );
    } else {
        let codes: Vec<String> = c.failures.iter().map(|f| f.code.clone()).collect();
        let mut f = Finding::new(MODULE, "c2pa.invalid", Severity::High, 0.9, "Content Credentials fail validation")
            .explain(
                "The C2PA manifest does not match the file: the content or the manifest was altered after signing.",
            )
            .limits("Some failures stem from unsupported C2PA features or benign re-wrapping; inspect the codes.")
            .with("codes", codes.join(", "));
        for st in c.failures.iter().take(10) {
            f = f.with(st.code.clone(), &st.explanation);
        }
        out.push(f);
    }
    if c.untrusted_signer {
        out.push(
            Finding::new(MODULE, "c2pa.signer_unverified", Severity::Info, 1.0, "Signer not verified against a trust list")
                .explain("Forensica works offline and ships no C2PA trust list, so it cannot say whether the signer is a recognised organisation.")
                .limits("A valid signature from an unknown signer proves integrity, not identity.")
                .with("issuer", &c.issuer),
        );
    }
    let edits: Vec<&String> = c
        .actions
        .iter()
        .filter(|a| !matches!(a.as_str(), "c2pa.created" | "c2pa.opened" | "c2pa.placed" | "c2pa.published"))
        .collect();
    if !edits.is_empty() {
        out.push(
            Finding::new(
                MODULE,
                "c2pa.edit_actions",
                Severity::Low,
                0.95,
                format!("Credentials declare {} edit action(s)", edits.len()),
            )
            .explain("The signed manifest lists editing actions performed by the claim generator.")
            .limits("Actions are self-reported by the signing software.")
            .with("actions", edits.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")),
        );
    }
    for st in &c.source_types {
        if let Some(mut f) = generator::digital_source_type("c2pa.actions digitalSourceType", st) {
            f.module_id = MODULE.into();
            f.code = "c2pa.digital_source_type".into();
            out.push(f);
        }
    }
    // Strongest declaration anywhere in the earlier history (ingredients).
    let rank = |t: &str| match t.rsplit('/').next().unwrap_or(t) {
        "trainedAlgorithmicMedia" | "compositeWithTrainedAlgorithmicMedia" => 3,
        "algorithmicMedia" | "compositeSynthetic" => 2,
        _ => 0,
    };
    if let Some((dst, what)) = c.history_source_types.iter().max_by_key(|(d, _)| rank(d)).filter(|(d, _)| rank(d) >= 2)
    {
        let current = c.source_types.iter().map(|t| rank(t)).max().unwrap_or(0);
        if rank(dst) > current {
            out.push(
                Finding::new(MODULE, "c2pa.history_generative_ai", Severity::High, 0.9, "Content Credentials history includes generative AI")
                    .explain(format!(
                        "An earlier step in this file's signed provenance chain declares AI-generated content ({}: \"{}\"). The latest step only re-saved or enhanced it, which is why it looks innocuous on the surface. Typical of AI photo editors (e.g. objects added or removed with generative fill).",
                        dst.rsplit('/').next().unwrap_or(dst),
                        what
                    ))
                    .limits("A self-declaration by the signing software; it says AI was involved, not which pixels changed.")
                    .with("digital_source_type", dst)
                    .with("step", what),
            );
        }
    }
    if c.descriptions.iter().any(|d| d.to_ascii_lowercase().contains("synthid")) {
        out.push(
            Finding::new(MODULE, "c2pa.synthid_declared", Severity::Medium, 0.9, "Credentials declare an invisible SynthID watermark")
                .explain("The provenance chain states that Google's imperceptible SynthID watermark was added, which Google applies to content produced or edited with its generative AI.")
                .limits("Forensica cannot verify SynthID offline (that needs Google's detector); this relies on the signed declaration only.")
                .with("descriptions", c.descriptions.join(" | ")),
        );
    }
    if let Some(f) = generator::from_software("C2PA claim generator", &c.claim_generator) {
        let mut f = f;
        f.module_id = MODULE.into();
        out.push(f);
    }
    out
}

#[cfg(all(test, feature = "c2pa"))]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../testdata/c2pa/CA.jpg")).unwrap()
    }

    #[test]
    fn reads_valid_manifest() {
        let c = read(&fixture(), FileFormat::Jpeg);
        assert!(c.present, "{:?}", c.error);
        assert!(c.claim_generator.contains("make_test_images"));
        assert!(c.actions.iter().any(|a| a == "c2pa.color_adjustments"));
        assert!(c.untrusted_signer);
        assert!(c.failures.is_empty(), "{:?}", c.failures);
        let f = findings(&c);
        assert!(f.iter().any(|x| x.code == "c2pa.valid"));
        assert!(f.iter().any(|x| x.code == "c2pa.edit_actions"));
    }

    #[test]
    fn detects_tampering() {
        let mut b = fixture();
        let layout = forensica_core::container::jpeg_layout(&b).unwrap();
        let sos = layout.with_marker(0xDA).next().unwrap().entropy.clone().unwrap();
        let mid = (sos.start + sos.end) / 2;
        b[mid] = b[mid].wrapping_add(1);
        if b[mid] == 0xFF {
            b[mid] = 0x00;
        }
        let c = read(&b, FileFormat::Jpeg);
        assert!(c.present);
        assert!(!c.failures.is_empty());
        assert!(findings(&c).iter().any(|x| x.code == "c2pa.invalid"));
    }

    #[test]
    fn generative_ai_in_ingredient_chain_is_reported() {
        let act = |a: &str, d: &str, t: &str| serde_json::json!({"action": a, "description": d, "digitalSourceType": format!("http://cv.iptc.org/newscodes/digitalsourcetype/{t}")});
        let man = |acts: Vec<serde_json::Value>| serde_json::json!({"assertions": [{"label": "c2pa.actions.v2", "data": {"actions": acts}}]});
        let json = serde_json::json!({
            "active_manifest": "m3",
            "manifests": {
                "m1": man(vec![act("c2pa.created", "Created by Google Generative AI.", "trainedAlgorithmicMedia")]),
                "m2": man(vec![act("c2pa.edited", "Added imperceptible SynthId watermark.", "trainedAlgorithmicMedia")]),
                "m3": man(vec![act("c2pa.transcoded", "", "algorithmicallyEnhanced")]),
            }
        });
        let f = findings(&summarize(json));
        let codes: Vec<&str> = f.iter().map(|x| x.code.as_str()).collect();
        assert!(codes.contains(&"c2pa.history_generative_ai"), "{codes:?}");
        assert!(codes.contains(&"c2pa.synthid_declared"), "{codes:?}");
        let h = f.iter().find(|x| x.code == "c2pa.history_generative_ai").unwrap();
        assert_eq!(h.severity, Severity::High);
    }

    #[test]
    fn absent_manifest() {
        let c = read(&crate::exif_read::testutil::jpeg(8, 8, 90), FileFormat::Jpeg);
        assert!(!c.present);
        assert_eq!(findings(&c)[0].code, "c2pa.absent");
    }
}
