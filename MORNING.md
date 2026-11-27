# Good morning 👋

_Status file for the overnight build, updated after every task. Last update: Phase 1 PR opened._

## TL;DR
- **Working end-to-end on Linux desktop:** open/drag-drop/CLI-arg a photo → SHA-256 → parallel analysis → findings, metadata tables, ELA/noise/spectrum layers, PDF/JSON export.
- **Android debug APK builds** (arm64, armv7, x86_64), with **no INTERNET permission**; not tested on a device (none attached).
- Phase 1 complete; Phase 2 mostly complete; Phases 3–5 in progress (see below).

## Status by phase
| Phase | Item | State |
|---|---|---|
| 1 | Scaffolding, CI, bridge (Linux + Android) | ✅ (CI workflow written; see PR checks) |
| 1 | File intake: picker, drag-drop, Android open-with/share, CLI arg, SHA-256, sniffing | ✅ |
| 1 | Metadata viewer (EXIF/XMP/IPTC/ICC/MakerNotes/GPS/thumbnail) | ✅ |
| 1 | Anomaly rules | ✅ 20+ rules, unit-tested |
| 1 | C2PA reader + validation (offline) | ✅ |
| 1 | JPEG structure + quant tables | ✅ (IJG family only — see D13) |
| 1 | PNG/WebP/HEIC text chunks + generator hints | ✅ (PNG/JPEG text; WebP/HEIC via XMP/EXIF) |
| 2 | ELA (adjustable), noise residual, FFT, channels, bit planes | ✅ |
| 2 | Viewer: zoom/pan, opacity, side-by-side, swipe, pixel inspector | ✅ |
| 3–5 | | ⏳ in progress |

## Build & run
One-time environment (already done on this machine; see docs/decisions.md D2):
```sh
source ~/.local/forensica-env.sh   # cargo, Flutter, user-local GTK/clang toolchain, Android SDK
```
Linux:
```sh
cd app
flutter run -d linux                   # or:
flutter build linux --release && ./build/linux/x64/release/bundle/forensica [photo.jpg]
```
Android (debug APK, ~200 MB because of Flutter debug artefacts):
```sh
cd app && flutter build apk --debug
adb install -r build/app/outputs/flutter-apk/app-debug.apk
```
Engine only:
```sh
cd core && cargo test && cargo clippy --all-targets -- -D warnings
```

## Manual test checklist
- [ ] Linux: open `testdata/c2pa/CA.jpg` → "Content Credentials intact", "Credentials declare 1 edit action".
- [ ] Linux: drag-and-drop a photo onto the home screen.
- [ ] Linux: `forensica photo.jpg` from a terminal opens it directly.
- [ ] Tools tab: ELA slider + Render, channel Cb/Cr, bit plane 0; switch Overlay / Side by side / Swipe; pixel inspector.
- [ ] Export PDF and JSON; check SHA-256 in both.
- [ ] Settings: dark/light, Italian, max resolution, disable a module and re-run.
- [ ] Android: install APK, open an image from Gallery via "Share → Forensica" and "Open with".
- [ ] Android: confirm Settings → Apps → Forensica shows **no network permission**.
- [ ] Try a photo edited in GIMP/Photoshop vs the camera original.

## Decisions worth reviewing
See `docs/decisions.md` (D1–D13). Most important: D2 (user-local toolchain), D10 (no INTERNET even in debug), D13 (no camera quant-table DB shipped).

## Broken / skipped
See `docs/blockers.md`.
