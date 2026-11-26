//! The plugin trait and the per-run context.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{Error, ImageInput, ModuleResult, Result};

/// Cooperative cancellation flag shared between the UI and running modules.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    /// New, non-cancelled token.
    pub fn new() -> Self {
        Self::default()
    }
    /// Requests cancellation.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    /// Whether cancellation was requested.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// User-tunable analysis options.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Options {
    /// Max pixels for pixel-level modules (0 = full resolution). Used for the Android degraded mode.
    pub max_pixels: u64,
    /// JPEG quality used for error level analysis.
    pub ela_quality: u8,
    /// Module ids to run; empty = all applicable.
    pub modules: Vec<String>,
    /// Free-form module parameters.
    pub extra: BTreeMap<String, String>,
}

impl Default for Options {
    fn default() -> Self {
        Self { max_pixels: 0, ela_quality: 90, modules: Vec::new(), extra: BTreeMap::new() }
    }
}

/// Progress callback type: receives a fraction in `0..=1`.
pub type ProgressFn<'a> = &'a (dyn Fn(f32) + Send + Sync);

/// Per-module execution context.
#[derive(Clone)]
pub struct Context<'a> {
    /// Options for this run.
    pub options: Options,
    cancel: CancelToken,
    progress: Option<ProgressFn<'a>>,
}

impl std::fmt::Debug for Context<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context").field("options", &self.options).finish()
    }
}

impl<'a> Context<'a> {
    /// Creates a context.
    pub fn new(options: Options, cancel: CancelToken) -> Self {
        Self { options, cancel, progress: None }
    }

    /// Context with default options, for tests and simple callers.
    pub fn simple() -> Context<'static> {
        Context { options: Options::default(), cancel: CancelToken::new(), progress: None }
    }

    /// Attaches a progress callback.
    #[must_use]
    pub fn with_progress(mut self, f: ProgressFn<'a>) -> Self {
        self.progress = Some(f);
        self
    }

    /// Returns `Err(Cancelled)` if the user cancelled. Modules call this between steps.
    pub fn check_cancel(&self) -> Result<()> {
        if self.cancel.is_cancelled() {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }

    /// Reports module-local progress (0..=1).
    pub fn report(&self, fraction: f32) {
        if let Some(p) = &self.progress {
            p(fraction.clamp(0.0, 1.0));
        }
    }

    /// The cancellation token.
    pub fn cancel_token(&self) -> &CancelToken {
        &self.cancel
    }
}

/// An analysis module ("plugin"). Implementations must be pure functions of the
/// input bytes and options: no network, no writes to the original file.
pub trait AnalysisModule: Send + Sync {
    /// Stable id, e.g. `metadata`.
    fn id(&self) -> &'static str;
    /// Human-readable name.
    fn name(&self) -> &'static str;
    /// Bump when the algorithm changes; part of the cache key.
    fn version(&self) -> u32 {
        1
    }
    /// One-line description.
    fn description(&self) -> &'static str {
        ""
    }
    /// Whether the module can analyse this input.
    fn applies_to(&self, input: &ImageInput) -> bool;
    /// Runs the analysis.
    fn run(&self, input: &ImageInput, ctx: &mut Context<'_>) -> Result<ModuleResult>;
}
