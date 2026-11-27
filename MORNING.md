# Good morning 👋

_Status file for the overnight build, updated after every task._

## TL;DR
- **All five phases have a working, tested implementation.** Linux desktop app verified end-to-end on a virtual display (screenshots taken, layers/overlays/settings exercised). Android APK builds (debug + release signed with the debug key); **not run on a device** (none attached).
- **No network anywhere:** no INTERNET permission in any Android build type (CI checks it), and `cargo deny` bans HTTP-client crates.
- **24 MP photo analysed in 1.9 s** on this desktop (target < 10 s).
- Rust: ~150 unit tests incl. an end-to-end test over a synthetic sample set; Flutter: unit + FFI integration test.

## Where things are
- **Public repo:** https://github.com/BadSiliconLabs/forensica — `main` = 3 commits (scaffold, phase 1, phases 2–5), CI green on the same tree.
- **Release v0.1.0:** https://github.com/BadSiliconLabs/forensica/releases/tag/v0.1.0 — Linux x64 tarball, debug-signed APK (arm64/armv7/x86_64), SHA256SUMS.
- History was rewritten once (authorised) so commits use only the noreply address. The old PR #21 merge commit (9489817) still exists on GitHub attached to PR #21; ask GitHub Support to purge it if needed.
- All roadmap issues (#1–#20) are closed; feature branches deleted.

## Status by phase
| Phase | Item | State |
|---|---|---|
| 1 | Scaffolding, CI, bridge (Linux + Android) | ✅ |
| 1 | Intake: picker, drag-drop, `forensica <file>`, Android open-with/share, SHA-256, sniffing | ✅ |
| 1 | Metadata viewer (EXIF/XMP/IPTC/ICC/MakerNotes/GPS/thumbnail) | ✅ |
| 1 | Anomaly rules (20+) | ✅ |
| 1 | C2PA (offline validation) | ✅ |
| 1 | JPEG structure + quant tables | ✅ (IJG family only, see D13) |
| 1 | Text chunks + generator hints | ✅ |
| 2 | ELA (adjustable), noise, FFT, channels, bit planes | ✅ |
| 2 | Viewer: zoom/pan, opacity, side-by-side, swipe, pixel inspector, legends, match lines | ✅ |
| 3 | Double JPEG (+ primary quality + localisation map), grid alignment | ✅ |
| 3 | Resampling, copy-move (match lines), CFA | ✅ |
| 3 | Lighting | ✅ experimental illuminant-colour map only (no 3-D lighting) |
| 4 | PRNU vs reference photos | ✅ (Tools tab) |
| 4 | AI-generation | ✅ weak heuristics only; **no ONNX model** (D14) |
| 4 | Perceptual hash + near-duplicate search | ✅ (Tools tab) |
| 5 | PDF + JSON reports | ✅ |
| 5 | Batch mode (Linux app + `forensica-cli batch`) | ✅ |
| 5 | Settings, EN + IT, accessibility | ✅ (findings text is English-only, D7) |
| + | Android portrait = simplified mode, landscape = complete mode (D17) | ✅ verified on phone-sized virtual screens |
| 5 | Release artefacts | see "Releases" below |
| 5 | Online-plugin interface doc | ✅ `docs/online-plugins.md` (design only) |

## Build & run
```sh
source ~/.local/forensica-env.sh        # cargo, Flutter, user-local GTK/clang toolchain, Android SDK (see D2)
```
Linux:
```sh
cd app && flutter run -d linux
# or release build:
flutter build linux --release && ./build/linux/x64/release/bundle/forensica [photo.jpg]
```
Android:
```sh
cd app && flutter build apk --release --target-platform android-arm64,android-arm,android-x64   # signed with debug key
adb install -r build/app/outputs/flutter-apk/app-release.apk
```
Engine / CLI:
```sh
cd core && cargo test && cargo clippy --all-targets -- -D warnings && cargo deny check licenses bans sources
cargo run --release -p forensica-cli -- samples /tmp/fs          # synthetic demo set, one per detector
cargo run --release -p forensica-cli -- analyze /tmp/fs/02-spliced-resaved.jpg
cargo run --release -p forensica-cli -- bench photo.jpg
```

## Manual test checklist
- [ ] `forensica-cli samples /tmp/fs`, then open each file in the app: 02 (splice → double-compression map lights up the square), 03 (copy-move lines), 04 (resampling), 05 (SD parameters), 06 (hidden ZIP), 07 (stale thumbnail), 08 (declared AI). 01 should only show low/info notes.
- [ ] Open `testdata/c2pa/CA.jpg` → "Content Credentials intact" + "1 edit action".
- [ ] Drag-and-drop on Linux; `forensica photo.jpg` from a terminal; Ctrl+O / Ctrl+R / Ctrl+W.
- [ ] Tools tab: ELA slider + Render; Cb/Cr channel; bit plane 0; Overlay/Side-by-side/Swipe; pixel inspector.
- [ ] Tools tab: near-duplicates on a folder; PRNU with ≥5 photos from your own camera (same resolution), then with photos from another camera.
- [ ] Export PDF + JSON; check SHA-256 in both.
- [ ] Settings: light/dark, Italiano, 12 MP cap, disable a module and re-run.
- [ ] Batch analysis (Linux toolbar icon) on a folder.
- [ ] Android: Share → Forensica and Open with → Forensica from the gallery; App info shows **no network permission**.
- [ ] Try real photos: camera original vs. the same photo edited in GIMP/Photoshop vs. a screenshot vs. a WhatsApp-forwarded copy.

## Decisions worth reviewing
`docs/decisions.md` D1–D16. Most important: D2 (user-local toolchain, no sudo), D10 (no INTERNET even in debug → `flutter run` hot reload on a phone needs a local, uncommitted permission), D13 (no camera quant-table DB), D14 (no ONNX AI classifier), D15 (copy-move false-positive controls), D16 (which modules ignore the resolution cap).

## Known limitations / skipped
- Thresholds of the pixel detectors were calibrated on **synthetic** images; real-world false-positive rates are unknown. Everything is labelled with limitations; severities are conservative.
- HEIC/AVIF: metadata only (no pixel decoding).
- Progressive JPEGs: no DCT-domain double-compression analysis (grid analysis still runs).
- Findings text is English-only (UI is EN/IT).
- PDF uses built-in fonts (Latin-1); non-Latin characters in metadata are replaced by `?` in the PDF only.
- See `docs/blockers.md`.
