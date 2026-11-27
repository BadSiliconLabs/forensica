# Changelog

All notable changes to this project are documented here. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning: SemVer.

## [Unreleased]

## [0.1.0] - 2026-11-27
### Added
- **Engine:** plugin trait, parallel pipeline with progress streaming, cancellation, result cache (SHA-256 + module version + options), panic isolation; magic-byte sniffing; immutable inputs with SHA-256.
- **Metadata:** EXIF/XMP/IPTC/ICC/MakerNote/GPS/thumbnail extraction and 20+ anomaly rules; C2PA validation (offline); generator hints (SD/ComfyUI/Midjourney/NovelAI/InvokeAI, IPTC DigitalSourceType).
- **JPEG:** structure parser, IJG quality estimation, appended-payload detection, baseline DCT coefficient decoder, aligned double-compression detection with primary-quality estimate and localisation map, 8×8 grid analysis.
- **Pixel:** ELA, noise residual + local noise map, FFT spectrum, channel/bit-plane viewers, resampling detection, copy-move with match lines, CFA consistency, illuminant map (experimental), PRNU, pHash/dHash + near-duplicate search, weak AI-generation heuristics.
- **App (Linux + Android):** file picker, drag-and-drop, CLI argument, Android open-with/share intents; responsive analysis screen; viewer with zoom/pan, overlay opacity, side-by-side, swipe, pixel inspector; findings, metadata, tools; PDF and JSON reports; batch mode (Linux); settings; English and Italian.
- **CLI:** analyze, batch, bench, synthetic samples.
- **Docs:** architecture, decision log, per-module documentation, online-plugin design.
### Security / privacy
- No INTERNET permission in any Android build type; backups disabled; shared files copied to app cache and deleted.
