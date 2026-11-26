# Forensica

**Offline, privacy-first photo forensics for Linux desktop and Android.** Free software (AGPL-3.0-or-later).

Forensica examines an image and reports *indicators* (metadata inconsistencies, editing-software traces, compression history, pixel-level anomalies), each with a confidence, a plain-language explanation and its known limitations. It never claims an image is "fake" or "real".

- 100% offline: no telemetry, no network permission, no cloud.
- Originals are never modified; SHA-256 of the original is shown everywhere.
- Rust analysis engine + Flutter UI.

> Status: early development. See `CHANGELOG.md` and `MORNING.md`.

## Building
Requirements: Rust (stable), Flutter 3.x, for Linux the GTK3 dev toolchain (clang, cmake, ninja, pkg-config, libgtk-3-dev), for Android the SDK + NDK and `cargo-ndk`.

```sh
# engine tests
cd core && cargo test
# Linux app
cd app && flutter run -d linux
# Android debug APK
cd app && flutter build apk --debug
```

## Licence
AGPL-3.0-or-later. See `LICENSE`.
