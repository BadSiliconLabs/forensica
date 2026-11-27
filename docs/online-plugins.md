# Optional online plugins — interface design (NOT implemented)

Forensica is offline. This document specifies how *future* online features (e.g. SynthID or other watermark checks, reverse image search, C2PA trust-list updates, OCSP) could be added **without** weakening that guarantee. Nothing here exists in the code today, and no network code may be added to the core crates or the default app.

## Rules
1. **Separate, opt-in build.** Online plugins live in a separate crate (`forensica-online`) behind a cargo feature `online` that is **off by default**, and in a separate Android product flavour (`online`) that is the *only* one allowed to declare `android.permission.INTERNET`. The default APK and the default Linux bundle must never contain network code (CI checks the permission).
2. **Disabled by default at runtime.** Even in the online build, every plugin is disabled until the user enables it in Settings, with a full privacy notice.
3. **Per-request consent.** Each network request requires an explicit confirmation dialog listing *exactly* what leaves the device (hash only / thumbnail / full image), to whom, and the provider's retention policy. Consent cannot be persisted for full images.
4. **Minimise data.** Prefer sending a hash or a perceptual hash; send pixels only when unavoidable; strip all metadata from anything sent.
5. **Results are findings.** Plugin output is a normal `ModuleResult` with the same confidence/explanation/limitations rules, clearly labelled as coming from a third-party service.
6. **No telemetry, no background calls, no auto-update checks.**

## Proposed Rust interface
```rust
/// What a plugin needs to send.
pub enum DataScope { Sha256, PerceptualHash, Thumbnail { max_side: u32 }, FullImage }

/// Shown verbatim to the user before enabling and before each request.
pub struct PrivacyNotice {
    pub provider: String,
    pub endpoint_host: String,
    pub data_sent: DataScope,
    pub retention: String,
    pub terms_url: String,
}

/// Proof that the user confirmed this specific request; only the UI layer can mint one
/// (constructor is private to the consent module and bound to the request's SHA-256).
pub struct ConsentToken { /* sha256, scope, timestamp, nonce */ }

pub trait OnlinePlugin: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn notice(&self) -> PrivacyNotice;
    /// Performs the request. Must fail if the token does not match the input and scope.
    fn run(&self, input: &forensica_core::ImageInput, consent: &ConsentToken)
        -> forensica_core::Result<forensica_core::ModuleResult>;
}
```

## UI flow
Settings → "Online plugins (off)" → per-plugin switch with notice → on the analysis screen an extra "Online checks" card lists enabled plugins; tapping one shows the consent dialog; results appear as findings tagged `online:<provider>` and are included in reports with the provider name.

## Candidate plugins (future)
| Plugin | Data sent | Notes |
|---|---|---|
| Watermark detection (e.g. SynthID) | full image | only if the provider offers a public API with clear retention terms |
| Reverse image search | thumbnail | many providers log queries; warn the user |
| C2PA trust list / OCSP | certificate data | no image data |
