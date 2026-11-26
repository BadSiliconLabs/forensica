//! Error type shared by all Forensica crates.

use thiserror::Error;

/// Errors produced by the analysis engine.
#[derive(Debug, Error)]
pub enum Error {
    /// Reading the input failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// The file could not be decoded as an image.
    #[error("image decode error: {0}")]
    Decode(String),
    /// A file structure (JPEG markers, PNG chunks, ...) is malformed.
    #[error("malformed {format} data: {reason}")]
    Malformed {
        /// Container format name.
        format: &'static str,
        /// Human-readable reason.
        reason: String,
    },
    /// The module does not support this input.
    #[error("unsupported input: {0}")]
    Unsupported(String),
    /// Analysis was cancelled by the user.
    #[error("cancelled")]
    Cancelled,
    /// Encoding a visualization failed.
    #[error("encode error: {0}")]
    Encode(String),
    /// Invalid argument passed to an API.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}

/// Convenience alias.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Shorthand to build a [`Error::Malformed`].
    pub fn malformed(format: &'static str, reason: impl Into<String>) -> Self {
        Self::Malformed { format, reason: reason.into() }
    }
}
