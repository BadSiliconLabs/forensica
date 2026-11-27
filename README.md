# Forensica

**Offline, privacy-first photo forensics for Linux desktop and Android.** Free software (AGPL-3.0-or-later).

Forensica examines a photo and reports *indicators* — metadata inconsistencies, editing-software traces, compression history, cloned or pasted regions, resampling, sensor fingerprints — each with a **confidence**, a **plain-language explanation** and its **known limitations**. It never says an image is "fake" or "real".

- 🔒 **100 % offline.** No telemetry, no network permission on Android, no network code in the engine.
- 🧾 **Originals are never modified.** The SHA-256 of the original is shown on screen and in every report.
- ⚙️ **Rust engine + Flutter UI**, parallel analysis, 24 MP photo analysed in ~2 s on a desktop.

## What it checks
| Area | Modules |
|---|---|
| Metadata | EXIF, XMP (incl. edit history), IPTC, ICC, MakerNotes, GPS (coordinates only), embedded thumbnail; 20+ consistency rules |
| Provenance | C2PA / Content Credentials, validated offline |
| Generators | Stable Diffusion / ComfyUI / Midjourney parameters, IPTC DigitalSourceType, AI tool names |
| JPEG | structure, quantisation tables & quality, hidden appended data, double compression with localisation map, 8×8 grid alignment |
| Pixels | ELA (adjustable), noise residual & noise-level map, FFT spectrum, channels, bit planes |
| Manipulation | copy-move with match lines, resampling (global/local), CFA demosaicing consistency, illuminant map (experimental) |
| Source | PRNU camera fingerprint vs. your reference photos, perceptual hashes + near-duplicate search |
| AI generation | weak, honest heuristics only (see `docs/modules/synthetic.md`) |

Every detector is documented in [`docs/modules/`](docs/modules) (how it works, failure modes, references).

## Using it
- **Linux:** open a file, drag & drop it onto the window, or run `forensica photo.jpg`. Batch mode (folder in → JSON + PDF reports out) is in the toolbar.
- **Android:** "Share → Forensica" or "Open with → Forensica" from the gallery, or open a file from the app. Large photos are analysed at 12 MP by default (configurable).
- **Reports:** export PDF or JSON from the analysis screen.
- **CLI:** `forensica-cli analyze|batch|bench|samples` (see `core/crates/forensica-cli`). `forensica-cli samples <dir>` writes a synthetic demo set exercising every detector.

## Building
Requirements: Rust stable, Flutter 3.50+, for Linux the GTK3 toolchain (clang, cmake, ninja, pkg-config, libgtk-3-dev), for Android the SDK + NDK 28.

```sh
cd core && cargo test                              # engine tests
cd app && flutter run -d linux                     # desktop
cd app && flutter build apk --debug --target-platform android-arm64,android-x64
```

## Project layout
```
core/   Rust workspace: forensica-core, -meta, -jpeg, -pixel, -engine, forensica_bridge, -cli
app/    Flutter app (Linux + Android)
docs/   architecture, decisions, blockers, per-module docs, online-plugin design
testdata/  licensed fixtures + SOURCES.md (most fixtures are synthesised in tests)
```

## Licence
AGPL-3.0-or-later. See `LICENSE`. Third-party dependencies are AGPL-compatible (checked with `cargo deny`).
