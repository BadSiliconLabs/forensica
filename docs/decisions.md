# Decision log

One entry per decision: context, choice, alternatives. Newest last.

## D1 — Repository layout and name
- **Context:** Prompt suggested `forensica/` with `core/` (Rust workspace) and `app/` (Flutter).
- **Choice:** Kept the name *Forensica* and the suggested layout. The repo root is the existing `~/Dev/app` directory (it was empty), pushed to private `BadSiliconLabs/forensica`.
- **Alternatives:** `gh repo create --clone` into a new folder; rejected because the working directory was already the intended project root.

## D2 — User-local Linux desktop toolchain (no sudo)
- **Context:** `clang`, `pkg-config` and GTK3 headers were missing and sudo needs a password.
- **Choice:** `apt-get download` the needed `.deb`s and extract them with `dpkg -x` into `~/.local/linuxdev`, rewriting `.pc` prefixes and symlinks. `~/.local/linuxdev/env.sh` sets `PATH`, `LD_LIBRARY_PATH` and `PKG_CONFIG_PATH`. Nothing outside `$HOME` was touched.
- **Alternatives:** asking for sudo (impossible overnight), building GTK from source (hours), Flatpak SDK (heavy).

## D3 — Shared container walkers in `forensica-core`
- **Context:** Both metadata and JPEG analysis need JPEG segment / PNG chunk walking.
- **Choice:** `forensica-core::container` holds bounds-checked walkers for JPEG, PNG, RIFF and ISO-BMFF; the other crates build on it.
- **Alternatives:** have `forensica-meta` depend on `forensica-jpeg` (creates an awkward coupling); duplicate parsers (rejected).

## D4 — `Context<'a>` carries a borrowed progress callback
- **Context:** Prompt's trait passes `ctx: &mut Context`. Progress must stream to Flutter while rayon runs modules in parallel.
- **Choice:** `Context<'a>` holds `&'a (dyn Fn(f32) + Send + Sync)`. Modules call `ctx.report(f)` and `ctx.check_cancel()?`. Added `version()` and `description()` to the trait (version is part of the cache key).
- **Alternatives:** `Arc<dyn Fn>` with `'static` bound (forces channels and boxing everywhere).

## D5 — EXIF via `kamadak-exif`, own parsers for the rest
- **Context:** Need EXIF/XMP/IPTC/ICC/MakerNotes across JPEG, PNG, WebP, TIFF, HEIF.
- **Choice:** `kamadak-exif` (BSD-2-Clause) for EXIF in all containers (and its experimental writer to synthesise test fixtures). XMP is found by scanning for `<x:xmpmeta` anywhere in the file (container-agnostic, also handles HEIF) plus JPEG Extended XMP reassembly, parsed with `quick-xml` (MIT). IPTC-IIM, Photoshop IRB, ICC header and MakerNote vendor identification are small hand-written parsers.
- **Alternatives:** `nom-exif` (less mature writer support), `rexiv2`/exiv2 (C++ GPL, needs system libs, painful on Android).

## D6 — C2PA via the official `c2pa` crate, no HTTP
- **Context:** Must read and validate Content Credentials, fully offline.
- **Choice:** `c2pa = { default-features = false, features = ["rust_native_crypto"] }`. With default features off no HTTP client crate is compiled at all; `ocsp_fetch` and `remote_manifest_fetch` also default to false. No trust list is bundled, so signer identity is reported as "not verified against a trust list".
- **Alternatives:** hand-written JUMBF parser (no signature validation).

## D7 — Findings are English-only in v1
- **Context:** UI must be EN + IT. Findings are produced in Rust.
- **Choice:** UI chrome localised via Flutter `gen-l10n`; finding texts are English, but every finding has a stable `code` so translations can be added later without touching the engine.
- **Alternatives:** locale parameter in the engine (doubles the amount of text in Rust right now).

## D8 — `forensica-engine` crate between bridge and modules
- **Context:** Both the Flutter bridge and the CLI need the module registry, sessions and interactive renderers.
- **Choice:** New crate `forensica-engine` (registry, sessions with LRU of 4 decoded images, previews, pixel inspector, interactive ELA/channel/bit-plane). The bridge is a thin DTO layer.
- **Alternatives:** CLI linking the bridge (pulls flutter_rust_bridge into the CLI).

## D9 — Bridge package named `forensica_bridge` (underscore)
- **Context:** cargokit derives the artifact file name from the *package* name without normalising `-`, so `forensica-bridge` looked for `libforensica-bridge.so`.
- **Choice:** Package name `forensica_bridge`; directory stays `crates/forensica-bridge`.

## D10 — No INTERNET permission even in debug/profile builds
- **Context:** Flutter templates add INTERNET to debug/profile manifests for hot reload. The user installs the *debug* APK; the offline principle is non-negotiable.
- **Choice:** Removed from all manifests and `tools:node="remove"` in the main manifest so no plugin can merge it back. CI fails if the APK requests it. Developers who need `flutter run` on a device add it locally without committing.
- **Alternatives:** separate "dev" flavour (more build complexity).

## D11 — Plain struct for streamed events (no `freezed`)
- **Context:** flutter_rust_bridge maps Rust enums with data to `freezed` classes, which adds build_runner + two dependencies.
- **Choice:** `AnalysisEvent` is a struct with `stage` and an optional `report`. IDs are `u32`/`i64` so Dart gets `int`, not `BigInt`.

## D12 — `material_ui` and older `desktop_drop`
- **Context:** Flutter 3.50 moved Material into the `material_ui` package (go_router already uses it). `desktop_drop` ≥0.9 needs `cross_file` 0.4 which conflicts with `file_picker` 13.
- **Choice:** import `package:material_ui/material_ui.dart`; use `desktop_drop` 0.7.x until both agree on `cross_file`.

## D13 — Honest JPEG fingerprints
- **Context:** The prompt asks for a built-in table of camera/software quantisation tables. No verified, redistributable dataset was available offline and inventing tables would be dishonest.
- **Choice:** Ship the IJG/libjpeg family (computed exactly for q = 1…100) plus structural signatures (Ducky, Adobe APP14, JFIF+EXIF, progressive, MPF). The fingerprint function is ready to accept a data table later.
