//! Detection of explicit AI-generator / editing-tool declarations in metadata.
//!
//! These are *declarations* written by software. They are strong when present,
//! but trivially removed or forged, and their absence says nothing.

use forensica_core::{Finding, Severity};

use crate::textchunks::TextChunk;

/// Module id used for findings.
pub const MODULE: &str = "generator";

/// Known generative-AI products, matched case-insensitively against software strings.
pub const AI_TOOLS: &[&str] = &[
    "midjourney",
    "dall-e",
    "dall·e",
    "dalle",
    "stable diffusion",
    "stablediffusion",
    "sdxl",
    "comfyui",
    "automatic1111",
    "invokeai",
    "novelai",
    "fooocus",
    "adobe firefly",
    "firefly",
    "imagen",
    "gemini",
    "ideogram",
    "leonardo.ai",
    "nightcafe",
    "craiyon",
    "bing image creator",
    "image creator from designer",
    "openai",
    "chatgpt",
    "gpt-4o",
    "flux.1",
    "black forest labs",
    "playground ai",
    "dreamstudio",
    "runway",
    "grok",
    "aurora",
];

/// Returns the matched AI tool name, if any.
pub fn match_ai_tool(s: &str) -> Option<&'static str> {
    let l = s.to_ascii_lowercase();
    AI_TOOLS.iter().copied().find(|t| {
        // Avoid matching "aurora"/"runway"/"grok"/"gemini" inside unrelated words by requiring word boundaries.
        l.match_indices(t).any(|(i, _)| {
            let before = l[..i].chars().next_back().is_none_or(|c| !c.is_ascii_alphanumeric());
            let after = l[i + t.len()..].chars().next().is_none_or(|c| !c.is_ascii_alphanumeric());
            before && after
        })
    })
}

fn finding(code: &str, conf: f32, title: String, tool: &str, key: &str, value: &str) -> Finding {
    let preview: String = value.chars().take(300).collect();
    Finding::new(MODULE, code, Severity::High, conf, title)
        .explain(format!(
            "The file contains metadata written by {tool}. Such fields are normally only produced when an image \
             is created or processed by that generator."
        ))
        .limits(
            "Metadata is easy to add, copy or strip. Its presence is a strong hint about the software that last \
             wrote the file, not proof of how the content was made; its absence says nothing.",
        )
        .with("field", key)
        .with("value", preview)
}

/// Scans text chunks for generator parameters.
pub fn from_text_chunks(chunks: &[TextChunk]) -> Vec<Finding> {
    let mut out = Vec::new();
    for c in chunks {
        let k = c.key.to_ascii_lowercase();
        let v = &c.value;
        let hit: Option<(&str, &str)> = if k == "parameters"
            && (v.contains("Steps:") || v.contains("Sampler:") || v.contains("CFG scale"))
        {
            Some((
                "gen.sd_parameters",
                if v.contains("Fooocus") { "Fooocus" } else { "Stable Diffusion WebUI (A1111/Forge)" },
            ))
        } else if (k == "prompt" || k == "workflow") && v.trim_start().starts_with('{') {
            Some(("gen.comfyui", "ComfyUI"))
        } else if k == "invokeai_metadata" || k == "sd-metadata" || k == "dream" {
            Some(("gen.invokeai", "InvokeAI"))
        } else if k == "software" && v.to_ascii_lowercase().contains("novelai") {
            Some(("gen.novelai", "NovelAI"))
        } else if k == "comment" && v.contains("\"sampler\"") && v.contains("\"steps\"") {
            Some(("gen.json_params", "an AI image generator (JSON parameters)"))
        } else if (k == "description" || k == "comment") && v.contains("Job ID") {
            Some(("gen.midjourney", "Midjourney"))
        } else {
            match_ai_tool(v)
                .filter(|_| {
                    matches!(k.as_str(), "software" | "author" | "source" | "comment" | "description" | "title" | "")
                })
                .map(|_| ("gen.ai_tool_text", "a generative-AI tool"))
        };
        if let Some((code, tool)) = hit {
            let tool_name = if code == "gen.ai_tool_text" { match_ai_tool(v).unwrap_or(tool) } else { tool };
            out.push(finding(
                code,
                0.95,
                format!("Generator parameters from {tool_name}"),
                tool_name,
                &format!("{} {}", c.source, c.key),
                v,
            ));
        }
    }
    out
}

/// Checks a software / creator-tool string.
pub fn from_software(field: &str, value: &str) -> Option<Finding> {
    let tool = match_ai_tool(value)?;
    Some(finding(
        "gen.ai_software_tag",
        0.9,
        format!("Software tag names a generative-AI tool ({tool})"),
        tool,
        field,
        value,
    ))
}

/// IPTC Digital Source Type vocabulary (as used in XMP `Iptc4xmpExt:DigitalSourceType` and C2PA).
pub fn digital_source_type(field: &str, value: &str) -> Option<Finding> {
    let term = value.rsplit('/').next().unwrap_or(value);
    let (sev, title, expl) = match term {
        "trainedAlgorithmicMedia" => (
            Severity::High,
            "Declared as AI-generated (IPTC trainedAlgorithmicMedia)",
            "The IPTC Digital Source Type declares the content was created by a trained generative model.",
        ),
        "compositeWithTrainedAlgorithmicMedia" => (
            Severity::High,
            "Declared as partly AI-generated composite",
            "The IPTC Digital Source Type declares a composite that includes generative-AI elements.",
        ),
        "algorithmicMedia" | "compositeSynthetic" => (
            Severity::Medium,
            "Declared as algorithmic / synthetic media",
            "The IPTC Digital Source Type declares the content was produced algorithmically or synthetically composited.",
        ),
        "composite" | "algorithmicallyEnhanced" | "digitalCreation" | "minorHumanEdits" => (
            Severity::Low,
            "Declared as edited / composite",
            "The IPTC Digital Source Type declares the image was edited, enhanced or composited.",
        ),
        "digitalCapture" | "computationalCapture" | "negativeFilm" | "positiveFilm" | "print" => (
            Severity::Info,
            "Declared as captured by a device",
            "The IPTC Digital Source Type declares the content was captured by a camera or scanner.",
        ),
        _ => return None,
    };
    Some(
        Finding::new(MODULE, "gen.digital_source_type", sev, 0.9, title)
            .explain(expl)
            .limits("This is a self-declaration written by software; it can be removed or set incorrectly.")
            .with("field", field)
            .with("value", value),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tc(key: &str, value: &str) -> TextChunk {
        TextChunk { source: "tEXt".into(), key: key.into(), value: value.into() }
    }

    #[test]
    fn detects_sd_comfy_midjourney() {
        let f = from_text_chunks(&[tc("parameters", "a cat\nSteps: 20, Sampler: Euler a, CFG scale: 7, Seed: 1")]);
        assert_eq!(f[0].code, "gen.sd_parameters");
        assert_eq!(from_text_chunks(&[tc("prompt", "{\"3\":{}}")])[0].code, "gen.comfyui");
        assert_eq!(from_text_chunks(&[tc("Description", "a dog --ar 3:2 Job ID: 123")])[0].code, "gen.midjourney");
        assert!(from_text_chunks(&[tc("Title", "Holiday")]).is_empty());
    }

    #[test]
    fn software_matching_uses_word_boundaries() {
        assert!(from_software("Software", "Adobe Firefly 2").is_some());
        assert!(from_software("Software", "DALL-E 3").is_some());
        assert!(from_software("Software", "Adobe Photoshop 25.0").is_none());
        assert!(match_ai_tool("Aurorahdr").is_none());
        assert!(match_ai_tool("Grok 2").is_some());
    }

    #[test]
    fn digital_source_types() {
        let f =
            digital_source_type("x", "http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia").unwrap();
        assert_eq!(f.severity, Severity::High);
        assert_eq!(digital_source_type("x", "digitalCapture").unwrap().severity, Severity::Info);
        assert!(digital_source_type("x", "whatever").is_none());
    }
}
