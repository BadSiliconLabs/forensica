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
