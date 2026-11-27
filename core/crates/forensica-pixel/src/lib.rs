//! Pixel-level forensic analyses.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::sync::Arc;

use forensica_core::AnalysisModule;

pub mod channels;
pub mod ela;
pub mod gray;
pub mod noise;
pub mod spectrum;

/// All modules provided by this crate.
pub fn modules() -> Vec<Arc<dyn AnalysisModule>> {
    vec![Arc::new(ela::ElaModule), Arc::new(noise::NoiseModule), Arc::new(spectrum::SpectrumModule)]
}
