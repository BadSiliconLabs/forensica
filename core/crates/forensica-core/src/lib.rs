//! Forensica core: the analysis pipeline, the [`AnalysisModule`] plugin trait,
//! input handling (hashing, type sniffing, decoding) and result types.
//!
//! Principles enforced here:
//! * the original bytes are never modified (inputs are immutable `Arc<[u8]>`),
//! * every [`Finding`] carries a confidence, an explanation and limitations,
//! * no I/O other than reading the input file happens in this crate.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod container;
pub mod error;
pub mod input;
pub mod module;
pub mod pipeline;
pub mod report;
pub mod types;
pub mod viz;

pub use error::{Error, Result};
pub use input::{FileFormat, ImageInput};
pub use module::{AnalysisModule, CancelToken, Context, Options};
pub use pipeline::{Pipeline, ProgressEvent, ProgressStage};
pub use report::{ModuleRun, ModuleStatus, Report};
pub use types::{Finding, Legend, Line, ModuleResult, Severity, Visualization, VizKind};

/// Version of the Forensica analysis engine, embedded in every report.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
