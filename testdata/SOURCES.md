# Test data sources

Most fixtures are **synthesised in code** inside the unit tests (JPEGs with crafted EXIF/XMP/IPTC/ICC, PNGs with text chunks, spliced/noisy/copy-moved images). Only the files below are committed.

| File | Source | Licence | Notes |
|---|---|---|---|
| `c2pa/CA.jpg` | <https://github.com/contentauth/c2pa-rs/tree/main/sdk/tests/fixtures> | MIT OR Apache-2.0 (c2pa-rs repository licence) | Test image signed with the C2PA test certificate; `c2pa.color_adjustments` action. Tampered variants are generated in tests by flipping an entropy-coded byte. |
