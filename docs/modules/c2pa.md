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
