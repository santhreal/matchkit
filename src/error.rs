//! Error types shared across all matching backends.
//!
//! These errors represent conditions that any matcher (CPU, GPU, SIMD)
//! can encounter, regardless of the specific backend implementation.

/// Errors that can occur during pattern matching.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Input data is larger than the backend supports for one scan.
    #[error("scan input is too large ({bytes} bytes, max {max_bytes}). fix: shard the input or use a streaming scanner")]
    InputTooLarge {
        /// Input size in bytes.
        bytes: usize,
        /// Maximum supported bytes.
        max_bytes: usize,
    },

    /// Match buffer overflow — too many matches for the configured buffer.
    #[error("too many matches ({count} exceeds {max}). fix: reduce pattern count, split input, or increase buffer size")]
    MatchBufferOverflow {
        /// Actual number of matches found.
        count: usize,
        /// Maximum matches supported by the buffer.
        max: usize,
    },

    /// A pattern set is empty (no patterns to match against).
    #[error("pattern set is empty. fix: add at least one pattern before scanning")]
    EmptyPatternSet,

    /// A specific pattern is empty (zero bytes).
    #[error("pattern {index} is empty. fix: provide a non-empty byte sequence")]
    EmptyPattern {
        /// Index of the empty pattern.
        index: usize,
    },

    /// Pattern compilation failed in a backend.
    #[error("pattern compilation failed: {reason}. fix: check pattern syntax and backend logs")]
    PatternCompilationFailed {
        /// The underlying error description.
        reason: String,
    },

    /// Allocation failed because the match buffer could not grow.
    #[error("allocation failed: {message}. fix: reduce match count or increase available memory")]
    OutOfMemory {
        /// Description of the allocation failure.
        message: String,
    },

    /// A backend-specific error not covered by the universal variants.
    #[error("{0}")]
    Backend(Box<dyn std::error::Error + Send + Sync>),
}

/// Result type alias using [`Error`].
pub type Result<T> = std::result::Result<T, Error>;
