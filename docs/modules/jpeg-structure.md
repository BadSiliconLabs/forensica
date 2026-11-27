# JPEG structure (`jpeg`)

**Detects:** encoder fingerprints, re-encoding, appended/hidden payloads, unusual structure.

## How
- Bounds-checked marker walker (`forensica-core::container`), then DQT/DHT/SOF/SOS/DRI parsing.
- **Quality estimate:** each table is compared with the IJG (libjpeg) scaling of the Annex K tables for q = 1…100; the closest q and whether it matches *exactly* are reported.
- **Fingerprints:** built-in knowledge = IJG/libjpeg family (exact tables at any quality) and all-ones tables. *Camera-specific tables are not shipped*: no verified, redistributable dataset was available offline. The data model is ready for a table file to be added.
- **Trailing data:** bytes after EOI are classified (padding, JPEG/MPF, ZIP/RAR/7z/PDF/PNG, text, unknown).
- **Signatures:** JFIF+EXIF together, APP12 "Ducky" (Save for Web), APP14 "Adobe", APP after SOF, progressive mode, redefined tables, camera EXIF + IJG tables.
- **DCT coefficients:** `dct.rs` is a baseline Huffman entropy decoder that recovers the quantised coefficients (verified by reconstructing luma within <3 grey levels of a pixel decoder). Used by double-compression analysis.

## Failure modes
- Phones and cheap cameras may use libjpeg with standard tables → `jpeg.camera_with_software_tables` is only *low*.
- Some vendors legitimately append data (Samsung trailers, MPF depth maps).
- Progressive and arithmetic-coded files are not supported by the coefficient decoder.

## References
ITU-T T.81 (JPEG); CIPA DC-007 (MPF); H. Farid, "Digital Image Ballistics from JPEG Quantization", 2006; E. Kee, M. Johnson, H. Farid, "Digital Image Authentication from JPEG Headers", IEEE TIFS 2011.
