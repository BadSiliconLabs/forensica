# Metadata (`metadata`)

**Detects:** traces left in EXIF, XMP, IPTC, ICC, MakerNotes and the embedded thumbnail by cameras and software.

## How
- EXIF from any container via `kamadak-exif`; XMP by scanning the whole file for `<x:xmpmeta>` packets (plus JPEG Extended XMP); IPTC-IIM from Photoshop IRB (`8BIM` 0x0404); ICC header + `desc`/`cprt`; MakerNote vendor detection by signature.
- Rules (`forensica-meta/src/rules.rs`), each with a stable code:

| Code | Severity | Rule |
|---|---|---|
| `meta.camera` / `meta.gps` | info | Declared device, GPS coordinates (no map, offline) |
| `meta.editing_software` | medium | Software/CreatorTool/OriginatingProgram/History names an editor (Photoshop, GIMP, Lightroom, Snapseed…) |
| `meta.resave_software` | low | Names a converter/platform/library (ImageMagick, WhatsApp, Pillow…) |
| `gen.ai_software_tag` | high | Names a generative-AI product |
| `meta.modified_after_capture` | medium | ModifyDate > DateTimeOriginal + 60 s |
| `meta.modified_before_capture` | medium | ModifyDate < DateTimeOriginal − 60 s |
| `meta.digitized_mismatch` | low | DateTimeOriginal ≠ DateTimeDigitized |
| `meta.bad_date` / `meta.impossible_value` | low/medium | Malformed dates, future dates, FNumber/exposure/ISO/focal/GPS/orientation out of range |
| `meta.thumbnail_mismatch` / `meta.thumbnail_aspect` | high/medium | EXIF thumbnail content or shape differs from the image |
| `meta.no_exif` / `meta.no_camera_id` / `meta.sparse_exif` | low/info | Missing or trimmed camera metadata |
| `meta.makernote_missing` / `meta.makernote_inconsistent` | low/medium | Brand normally writes a MakerNote / vendor signature ≠ Make |
| `meta.dimension_mismatch` | medium | EXIF PixelX/YDimension ≠ actual size |
| `meta.xmp_history` | medium/info | xmpMM:History edit steps |
| `meta.document_ancestors` | medium | photoshop:DocumentAncestors (placed/pasted documents) |
| `meta.derived_from` / `meta.raw_processing` | low/medium | Export lineage; Camera Raw settings incl. local retouching |
| `gen.digital_source_type` | info…high | IPTC DigitalSourceType (e.g. `trainedAlgorithmicMedia`) |
| `meta.extension_mismatch`, `meta.camera_exif_in_export`, `meta.photoshop_save`, `meta.xmp_malformed` | low | Container clues |

Thumbnail comparison: both images are reduced to 32×32 luma, normalised for brightness/contrast (z-score) and compared with mean absolute difference; letterboxed thumbnails are cropped first.

## Failure modes
- All metadata is trivially editable: a careful forger copies EXIF from a genuine photo; absence of findings means nothing.
- Platforms (WhatsApp, Instagram, Telegram…) strip metadata, which triggers `meta.no_exif` for perfectly innocent files.
- Timezone handling: EXIF dates are naive local time; XMP dates with offsets are compared naively.
- Thumbnail mismatch can be benign (in-camera effects, rotation without thumbnail update).

## References
- CIPA DC-008 (EXIF 2.32/3.0); Adobe XMP Specification Part 1–3; IPTC Photo Metadata Standard & NewsCodes Digital Source Type; ICC.1:2022.
- J. Kee, H. Farid, "Digital Image Authentication from Thumbnails", SPIE 2010.
