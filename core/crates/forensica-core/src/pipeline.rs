//! Parallel pipeline with progress streaming, cancellation and a result cache.

use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::report::{now_rfc3339, FileInfo, ModuleRun, ModuleStatus, Report};
use crate::{AnalysisModule, CancelToken, Context, Error, ImageInput, ModuleResult, Options};

/// Stage of a module within a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProgressStage {
    /// Module started.
    Started,
    /// Module reported intermediate progress.
    Running,
    /// Module finished successfully (or from cache).
    Finished,
    /// Module failed.
    Failed,
    /// Module was cancelled.
    Cancelled,
}

/// A progress event streamed to the UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressEvent {
    /// Module id.
    pub module_id: String,
    /// Stage.
    pub stage: ProgressStage,
    /// Module-local progress.
    pub module_fraction: f32,
    /// Number of modules completed so far.
    pub completed: usize,
    /// Total number of modules in this run.
    pub total: usize,
}

type CacheKey = (String, &'static str, u32, String);

/// The analysis pipeline: a registry of modules plus an in-memory cache keyed by
/// (file SHA-256, module id, module version, options).
pub struct Pipeline {
    modules: Vec<Arc<dyn AnalysisModule>>,
    cache: Mutex<HashMap<CacheKey, ModuleResult>>,
    cache_limit: usize,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl Pipeline {
    /// Empty pipeline.
    pub fn new() -> Self {
        Self { modules: Vec::new(), cache: Mutex::new(HashMap::new()), cache_limit: 256 }
    }

    /// Registers a module.
    pub fn register(&mut self, module: Arc<dyn AnalysisModule>) -> &mut Self {
        self.modules.push(module);
        self
    }

    /// Registered modules.
    pub fn modules(&self) -> &[Arc<dyn AnalysisModule>] {
        &self.modules
    }

    /// Drops all cached results.
    pub fn clear_cache(&self) {
        if let Ok(mut c) = self.cache.lock() {
            c.clear();
        }
    }

    fn selected<'a>(&'a self, input: &ImageInput, opts: &Options) -> Vec<&'a Arc<dyn AnalysisModule>> {
        self.modules
            .iter()
            .filter(|m| opts.modules.is_empty() || opts.modules.iter().any(|id| id == m.id()))
            .filter(|m| m.applies_to(input))
            .collect()
    }

    /// Runs every applicable module in parallel and assembles a report.
    pub fn run(
        &self,
        input: &ImageInput,
        options: &Options,
        cancel: &CancelToken,
        on_progress: &(dyn Fn(ProgressEvent) + Send + Sync),
    ) -> Report {
        let started_at = now_rfc3339();
        let t0 = Instant::now();
        let selected = self.selected(input, options);
        let total = selected.len();
        let completed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let opt_key = serde_json::to_string(options).unwrap_or_default();

        let runs: Vec<ModuleRun> = selected
            .par_iter()
            .map(|m| {
                let emit = |stage, frac| {
                    on_progress(ProgressEvent {
                        module_id: m.id().to_string(),
                        stage,
                        module_fraction: frac,
                        completed: completed.load(std::sync::atomic::Ordering::SeqCst),
                        total,
                    });
                };
                emit(ProgressStage::Started, 0.0);
                let run =
                    self.run_one(m.as_ref(), input, options, cancel, &opt_key, &|f| emit(ProgressStage::Running, f));
                completed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let stage = match run.status {
                    ModuleStatus::Ok => ProgressStage::Finished,
                    ModuleStatus::Cancelled => ProgressStage::Cancelled,
                    _ => ProgressStage::Failed,
                };
                emit(stage, 1.0);
                run
            })
            .collect();

        Report {
            file: FileInfo::from_input(input),
            engine_version: crate::ENGINE_VERSION.to_string(),
            started_at,
            duration_ms: t0.elapsed().as_millis() as u64,
            options: options.clone(),
            modules: runs,
        }
    }

    fn run_one(
        &self,
        m: &dyn AnalysisModule,
        input: &ImageInput,
        options: &Options,
        cancel: &CancelToken,
        opt_key: &str,
        progress: &(dyn Fn(f32) + Send + Sync),
    ) -> ModuleRun {
        let key: CacheKey = (input.sha256().to_string(), m.id(), m.version(), opt_key.to_string());
        let t0 = Instant::now();
        let mut run = ModuleRun {
            id: m.id().to_string(),
            name: m.name().to_string(),
            version: m.version(),
            status: ModuleStatus::Ok,
            error: None,
            duration_ms: 0,
            cached: false,
            result: ModuleResult::default(),
        };
        if let Some(hit) = self.cache.lock().ok().and_then(|c| c.get(&key).cloned()) {
            run.cached = true;
            run.result = hit;
            return run;
        }
        if cancel.is_cancelled() {
            run.status = ModuleStatus::Cancelled;
            return run;
        }
        let mut ctx = Context::new(options.clone(), cancel.clone()).with_progress(progress);
        let outcome = catch_unwind(AssertUnwindSafe(|| m.run(input, &mut ctx)));
        run.duration_ms = t0.elapsed().as_millis() as u64;
        match outcome {
            Ok(Ok(result)) => {
                if let Ok(mut c) = self.cache.lock() {
                    if c.len() >= self.cache_limit {
                        c.clear();
                    }
                    c.insert(key, result.clone());
                }
                run.result = result;
            }
            Ok(Err(Error::Cancelled)) => run.status = ModuleStatus::Cancelled,
            Ok(Err(e)) => {
                run.status = ModuleStatus::Failed;
                run.error = Some(e.to_string());
            }
            Err(_) => {
                run.status = ModuleStatus::Failed;
                run.error = Some("module panicked (internal error)".into());
            }
        }
        run
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Finding, Severity};

    struct Echo;
    impl AnalysisModule for Echo {
        fn id(&self) -> &'static str {
            "echo"
        }
        fn name(&self) -> &'static str {
            "Echo"
        }
        fn applies_to(&self, _: &ImageInput) -> bool {
            true
        }
        fn run(&self, input: &ImageInput, ctx: &mut Context<'_>) -> crate::Result<ModuleResult> {
            ctx.check_cancel()?;
            ctx.report(0.5);
            Ok(ModuleResult {
                findings: vec![
                    Finding::new("echo", "echo.len", Severity::Info, 1.0, "len").with("len", input.bytes().len())
                ],
                ..Default::default()
            })
        }
    }

    struct Boom;
    impl AnalysisModule for Boom {
        fn id(&self) -> &'static str {
            "boom"
        }
        fn name(&self) -> &'static str {
            "Boom"
        }
        fn applies_to(&self, _: &ImageInput) -> bool {
            true
        }
        fn run(&self, _: &ImageInput, _: &mut Context<'_>) -> crate::Result<ModuleResult> {
            panic!("boom")
        }
    }

    #[test]
    fn runs_modules_caches_and_isolates_panics() {
        let mut p = Pipeline::new();
        p.register(Arc::new(Echo)).register(Arc::new(Boom));
        let input = ImageInput::from_bytes("x.bin", vec![1u8, 2, 3]);
        let events = Mutex::new(Vec::new());
        let r = p.run(&input, &Options::default(), &CancelToken::new(), &|e| events.lock().unwrap().push(e));
        assert_eq!(r.modules.len(), 2);
        let echo = r.modules.iter().find(|m| m.id == "echo").unwrap();
        assert_eq!(echo.status, ModuleStatus::Ok);
        assert_eq!(echo.result.findings[0].evidence[0].1, "3");
        let boom = r.modules.iter().find(|m| m.id == "boom").unwrap();
        assert_eq!(boom.status, ModuleStatus::Failed);
        assert!(!events.lock().unwrap().is_empty());

        let r2 = p.run(&input, &Options::default(), &CancelToken::new(), &|_| {});
        assert!(r2.modules.iter().find(|m| m.id == "echo").unwrap().cached);
    }

    #[test]
    fn cancellation_is_honoured() {
        let mut p = Pipeline::new();
        p.register(Arc::new(Echo));
        let input = ImageInput::from_bytes("x.bin", vec![1u8]);
        let c = CancelToken::new();
        c.cancel();
        let r = p.run(&input, &Options::default(), &c, &|_| {});
        assert_eq!(r.modules[0].status, ModuleStatus::Cancelled);
    }

    #[test]
    fn module_filter() {
        let mut p = Pipeline::new();
        p.register(Arc::new(Echo)).register(Arc::new(Boom));
        let input = ImageInput::from_bytes("x.bin", vec![1u8]);
        let opts = Options { modules: vec!["echo".into()], ..Default::default() };
        let r = p.run(&input, &opts, &CancelToken::new(), &|_| {});
        assert_eq!(r.modules.len(), 1);
    }
}
