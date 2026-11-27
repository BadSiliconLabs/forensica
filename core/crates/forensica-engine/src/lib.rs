//! Forensica engine: the default module registry and per-file sessions.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use forensica_core::{CancelToken, ImageInput, Options, Pipeline, ProgressEvent, Report, Result};

pub mod interactive;

/// Builds a pipeline with every built-in module.
pub fn build_pipeline() -> Pipeline {
    let mut p = Pipeline::new();
    p.register(Arc::new(forensica_meta::MetadataModule))
        .register(Arc::new(forensica_meta::ProvenanceModule))
        .register(Arc::new(forensica_meta::GeneratorHintsModule));
    for m in forensica_jpeg::modules() {
        p.register(m);
    }
    for m in forensica_pixel::modules() {
        p.register(m);
    }
    p
}

static PIPELINE: LazyLock<Pipeline> = LazyLock::new(build_pipeline);

/// The process-wide pipeline (with its result cache).
pub fn pipeline() -> &'static Pipeline {
    &PIPELINE
}

/// An open file.
pub struct Session {
    /// Session id.
    pub id: u64,
    /// Path the file was read from.
    pub path: String,
    /// Immutable input.
    pub input: Arc<ImageInput>,
    cancel: Mutex<CancelToken>,
    report: Mutex<Option<Report>>,
}

impl Session {
    /// Runs the pipeline. A previous run's cancel token is replaced.
    pub fn analyze(&self, options: &Options, on_progress: &(dyn Fn(ProgressEvent) + Send + Sync)) -> Report {
        let token = CancelToken::new();
        if let Ok(mut c) = self.cancel.lock() {
            *c = token.clone();
        }
        let r = pipeline().run(&self.input, options, &token, on_progress);
        if let Ok(mut slot) = self.report.lock() {
            *slot = Some(r.clone());
        }
        r
    }

    /// Cancels a running analysis.
    pub fn cancel(&self) {
        if let Ok(c) = self.cancel.lock() {
            c.cancel();
        }
    }

    /// Last report, if any.
    pub fn last_report(&self) -> Option<Report> {
        self.report.lock().ok().and_then(|r| r.clone())
    }
}

static SESSIONS: LazyLock<Mutex<HashMap<u64, Arc<Session>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
/// Keep at most this many sessions alive (decoded images are large).
const MAX_SESSIONS: usize = 4;

/// Opens a file read-only and registers a session.
pub fn open(path: &Path) -> Result<Arc<Session>> {
    let input = Arc::new(ImageInput::from_path(path)?);
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let s = Arc::new(Session {
        id,
        path: path.to_string_lossy().into_owned(),
        input,
        cancel: Mutex::new(CancelToken::new()),
        report: Mutex::new(None),
    });
    if let Ok(mut map) = SESSIONS.lock() {
        while map.len() >= MAX_SESSIONS {
            let Some(oldest) = map.keys().min().copied() else { break };
            map.remove(&oldest);
        }
        map.insert(id, Arc::clone(&s));
    }
    Ok(s)
}

/// Looks up a session.
pub fn get(id: u64) -> Option<Arc<Session>> {
    SESSIONS.lock().ok().and_then(|m| m.get(&id).cloned())
}

/// Closes a session, freeing its memory.
pub fn close(id: u64) {
    if let Ok(mut m) = SESSIONS.lock() {
        m.remove(&id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_unique_ids() {
        let p = build_pipeline();
        let mut ids: Vec<_> = p.modules().iter().map(|m| m.id()).collect();
        let n = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), n);
        assert!(n >= 3);
    }
}
