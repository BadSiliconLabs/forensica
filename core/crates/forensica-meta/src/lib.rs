//! Metadata analysis for Forensica.
//!
//! Provides three analysis modules:
//! * [`MetadataModule`]: EXIF/XMP/IPTC/ICC/MakerNotes/GPS/thumbnail extraction plus anomaly rules,
//! * [`ProvenanceModule`]: C2PA / Content Credentials reading and validation,
//! * [`GeneratorHintsModule`]: text chunks and AI-generator hints (PNG/WebP/JPEG comments).
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod c2pa_reader;
pub mod exif_read;
pub mod generator;
pub mod icc;
pub mod iptc;
pub mod makernote;
pub mod rules;
pub mod table;
pub mod textchunks;
pub mod xmp;

mod modules;

pub use modules::{GeneratorHintsModule, MetadataModule, ProvenanceModule};
