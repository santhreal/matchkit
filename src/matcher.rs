//! Trait definitions for pattern matching backends.
//!
//! These traits define the polymorphic interface that enables zero-cost
//! backend swaps (CPU, GPU, SIMD, multi-GPU) without consumer code changes.
//!
//! # Architecture
//!
//! ```text
//! Matcher (async scan)
//!   ├── CpuMatcher (Aho-Corasick + regex)
//!   ├── GpuMatcher (wgpu compute shaders)
//!   ├── AutoMatcher (routes by input size)
//!   └── MultiGpuMatcher (parallel GPU dispatch)
//!
//! BlockMatcher (scan + max_block_size)
//!   └── Used for streaming/chunked scanning
//! ```

use crate::error::Result;
use crate::Match;
use async_trait::async_trait;

/// General trait for pattern matching across any backend.
///
/// Implementations must be `Send + Sync` to support concurrent scanning
/// across multiple threads or async tasks.
///
/// This trait is dyn-compatible (object-safe) so it can be used as
/// `Box<dyn Matcher>` via [`BoxedMatcher`].
///
/// # Example
///
/// ```rust,ignore
/// use matchkit::{Matcher, Match};
///
/// async fn search(matcher: &dyn Matcher, data: &[u8]) {
///     let matches = matcher.scan(data).await.unwrap();
///     for m in matches {
///         println!("pattern {} matched at {}..{}", m.pattern_id, m.start, m.end);
///     }
/// }
/// ```
#[async_trait]
pub trait Matcher: Send + Sync {
    /// Scan the provided data and return all found matches.
    async fn scan(&self, data: &[u8]) -> Result<Vec<Match>>;
}

/// Dynamic trait object type for matchers when generic boxing is required.
pub type BoxedMatcher = Box<dyn Matcher + Send + Sync>;

/// Block-based matching for large continuous scans bounded by memory.
///
/// Implementations advertise their maximum block size so callers can
/// chunk input appropriately for streaming pipelines.
#[async_trait]
pub trait BlockMatcher: Send + Sync {
    /// Submit a block of data for scanning.
    async fn scan_block(&self, data: &[u8]) -> Result<Vec<Match>>;

    /// Return the maximum block size supported by the hardware or configuration.
    fn max_block_size(&self) -> usize;
}
