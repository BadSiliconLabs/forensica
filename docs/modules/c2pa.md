# Content Credentials / C2PA (`provenance`)

**Detects:** presence and integrity of a C2PA manifest store; declared actions, ingredients and digital source type.

## How
Uses the official `c2pa` Rust SDK (`rust_native_crypto`, **no default features**, so no HTTP client is compiled). A cheap byte scan for `jumb`+`c2pa` avoids invoking the SDK on files without manifests. Validation failure codes (hash mismatch, signature mismatch…) become a **high** finding. `signingCredential.untrusted` is reported separately as *not verified against a trust list*: Forensica ships no trust list and never fetches OCSP or remote manifests.

## Failure modes
- Integrity ≠ truth: a valid manifest proves the file did not change since signing, not that the scene is real.
- Unknown signers cannot be distinguished from recognised ones offline.
- Remote (cloud) manifests referenced by URL are not fetched and are reported as absent/unreadable.
- Newer C2PA versions may be partially unsupported.

## References
C2PA Technical Specification 2.x; <https://github.com/contentauth/c2pa-rs>.

## Provenance chain
Every manifest in the store is inspected, not only the active one. AI photo editors (e.g. Google's) record "Created by … Generative AI" with `digitalSourceType = trainedAlgorithmicMedia` in an *ingredient* manifest, while the latest manifest only says "opened / transcoded / algorithmicallyEnhanced". If an earlier step declares generative AI and the active one does not, Forensica reports **`c2pa.history_generative_ai` (high)**. Action descriptions mentioning SynthID produce `c2pa.synthid_declared` (medium); the watermark itself cannot be verified offline.

## Why pixel detectors can miss AI edits
Generative editors typically re-render and re-encode the *whole* image (often at a new size), so there is no locally inconsistent compression, noise or CFA region to find; the inserted object is consistent with the regenerated surroundings. When the provenance metadata is stripped, such edits are generally not detectable by the classical methods in this tool.
