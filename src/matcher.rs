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

/// General trait for pattern matching across any backend.
///
/// Implementations must be `Send + Sync` to support concurrent scanning
/// across multiple threads or async tasks.
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
#[async_trait::async_trait]
pub trait Matcher: Send + Sync {
    /// Scan the provided data and return all found matches.
    async fn scan(&self, data: &[u8]) -> Result<Vec<Match>>;

    /// Scan the provided data, appending matches into `buf`.
    ///
    /// Returns the number of matches appended. The default implementation
    /// delegates to [`scan`](Self::scan) and extends `buf`, so specialized
    /// backends should override this to avoid the intermediate allocation.
    async fn scan_into(&self, data: &[u8], buf: &mut Vec<Match>) -> Result<usize> {
        let matches = self.scan(data).await?;
        let count = matches.len();
        buf.extend(matches);
        Ok(count)
    }

    /// Scan the provided data and return only the match count.
    ///
    /// The default implementation delegates to [`scan`](Self::scan).
    /// Specialized backends should override this to avoid allocation
    /// when only the count is needed.
    async fn scan_count(&self, data: &[u8]) -> Result<usize> {
        Ok(self.scan(data).await?.len())
    }
}

/// Dynamic trait object type for matchers when generic boxing is required.
pub type BoxedMatcher = Box<dyn Matcher + Send + Sync>;

/// Block-based matching for large continuous scans bounded by memory.
///
/// Implementations advertise their maximum block size so callers can
/// chunk input appropriately for streaming pipelines.
#[async_trait::async_trait]
pub trait BlockMatcher: Send + Sync {
    /// Submit a block of data for scanning.
    async fn scan_block(&self, data: &[u8]) -> Result<Vec<Match>>;

    /// Scan a block, appending matches into `buf`.
    ///
    /// Returns the number of matches appended. The default implementation
    /// delegates to [`scan_block`](Self::scan_block).
    async fn scan_block_into(&self, data: &[u8], buf: &mut Vec<Match>) -> Result<usize> {
        let matches = self.scan_block(data).await?;
        let count = matches.len();
        buf.extend(matches);
        Ok(count)
    }

    /// Scan a block and return only the match count.
    ///
    /// The default implementation delegates to [`scan_block`](Self::scan_block).
    async fn scan_block_count(&self, data: &[u8]) -> Result<usize> {
        Ok(self.scan_block(data).await?.len())
    }

    /// Return the maximum block size supported by the hardware or configuration.
    fn max_block_size(&self) -> usize;
}
