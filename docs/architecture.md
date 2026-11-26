# Forensica architecture

Forensica is an offline photo-forensics tool: a Rust analysis engine and a Flutter UI for Linux desktop and Android.

```
forensica/
  core/                     Rust workspace (engine)
    crates/
      forensica-core/       ImageInput, AnalysisModule trait, Pipeline, Report, viz helpers, container walkers
      forensica-meta/       EXIF/XMP/IPTC/ICC/MakerNotes/GPS/thumbnail rules, C2PA, generator hints
      forensica-jpeg/       JPEG structure, quantisation tables + fingerprints, trailing data, DCT decoding, double compression
      forensica-pixel/      ELA, noise residual, FFT, channels/bit planes, resampling, copy-move, CFA, lighting, PRNU, AI heuristics, pHash
      forensica-bridge/     flutter_rust_bridge v2 API consumed by the app
      forensica-cli/        headless batch tool (folder in, JSON reports out) and benchmark
  app/                      Flutter app (Linux + Android only)
    rust_builder/           cargokit glue that builds forensica-bridge for each target
  docs/                     this file, decisions, blockers, per-module docs
  testdata/                 small licensed fixtures (+ SOURCES.md); most fixtures are synthesised in tests
```

## Principles (enforced in code)
- **Offline.** No crate in the dependency tree performs network I/O (c2pa is built without HTTP features). The Android manifest has no `INTERNET` permission. No telemetry.
- **Originals are never modified.** `ImageInput` holds the bytes as an immutable `Arc<[u8]>`; the file is opened read-only once. SHA-256 is computed on load and printed in every view and report.
- **Indicators, not verdicts.** `Finding { severity, confidence, title, explanation, limitations, evidence }`. Confidence expresses how sure we are that the *indicator* is present, never that the image is "fake". The words fake/real are banned from UI strings.
- **Privacy.** Android share-intents are copied to the app cache directory, analysed, and deleted on close/startup.

## Engine

### Data flow
1. `ImageInput::from_path` reads bytes, hashes them (SHA-256) and sniffs the container from magic bytes.
2. Pixels are decoded lazily (once, shared by all modules) with the `image` crate. EXIF orientation is **not** applied: analysis happens on stored pixels, and the viewer shows the same orientation so overlays align.
3. `Pipeline::run` selects modules whose `applies_to` is true and runs them in parallel with rayon. Each module gets a `Context` with options, a cancellation token and a progress callback; progress events stream to Flutter.
4. Results are cached in memory by `(sha256, module id, module version, options)`.
5. A panic inside a module is caught and reported as a failed module; the rest of the report survives.

### Plugin trait
```rust
pub trait AnalysisModule: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn version(&self) -> u32 { 1 }
    fn description(&self) -> &'static str { "" }
    fn applies_to(&self, input: &ImageInput) -> bool;
    fn run(&self, input: &ImageInput, ctx: &mut Context<'_>) -> Result<ModuleResult>;
}
```
`ModuleResult { findings, visualizations, raw: serde_json::Value }`. Visualizations are PNG-encoded in Rust (max 4096 px per side) with a legend and optional line overlays (copy-move matches) in source-pixel coordinates.

### Large images / Android
`Options::max_pixels` caps the working resolution of pixel modules (default: unlimited on desktop, 12 MP on Android = "degraded mode"). Metadata/JPEG-structure modules always see the full file. Interactive tools (ELA quality slider, bit planes, pixel inspector) operate on a per-session decoded image kept in the bridge.

## Bridge
`forensica-bridge` exposes a small API (flutter_rust_bridge v2):
- `open_session(path) -> SessionInfo` (sha256, format, dimensions, preview RGBA)
- `analyze(session, options, sink: StreamSink<AnalysisEvent>)`: progress events then the final report
- `cancel(session)`
- interactive renderers: `render_ela`, `render_channel`, `render_bitplane`, `pixel_at`, ...
- `report_json(session)`
Rust types crossing the bridge are plain structs; `raw` module data crosses as a JSON string.

## App
Flutter 3, Material 3, Riverpod for state, go_router for navigation.
- Home: open file (file_picker), drag-and-drop on Linux (desktop_drop), Android "Open with"/"Share" intents (native Kotlin, copies the content URI into the cache dir).
- Analysis screen: responsive. Wide layouts show the image viewer with a side panel (findings / metadata / tools); narrow layouts use tabs and bottom sheets.
- Viewer: zoom/pan (InteractiveViewer), overlay with opacity, side-by-side and swipe compare, pixel inspector.
- Reports: PDF (`pdf` package) and JSON export.

## Online features
None. A future opt-in plugin interface is documented in `docs/online-plugins.md`; nothing is implemented.
