//! Pixel-level forensic analyses.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::sync::Arc;

use forensica_core::AnalysisModule;

pub mod cfa;
pub mod channels;
pub mod copymove;
pub mod ela;
pub mod gray;
pub mod illuminant;
pub mod noise;
pub mod resampling;
pub mod spectrum;

/// All modules provided by this crate.
pub fn modules() -> Vec<Arc<dyn AnalysisModule>> {
    vec![
        Arc::new(ela::ElaModule),
        Arc::new(noise::NoiseModule),
        Arc::new(spectrum::SpectrumModule),
        Arc::new(resampling::ResamplingModule),
        Arc::new(copymove::CopyMoveModule),
        Arc::new(cfa::CfaModule),
        Arc::new(illuminant::IlluminantModule),
    ]
}
