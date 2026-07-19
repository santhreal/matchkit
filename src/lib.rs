//! # matchkit, vocabulary types for multi-pattern matching
//!
//! `matchkit` provides the shared types that every crate in the Santh
//! performance ecosystem depends on: the [`Match`] struct, the [`Matcher`]
//! and [`BlockMatcher`] traits, and common error definitions.
//!
//! By isolating these types into a zero-dependency vocabulary crate,
//! consumer crates like `simdsieve`, `vyre`, `warpsearch`, and
//! `warpgrep` can all agree on a single match representation without
//! pulling in heavyweight GPU or regex dependencies.
//!
//! # Quick Start
//!
//! ```rust
//! use matchkit::{Match, MatchSet};
//!
//! let mut set = MatchSet::new();
//! set.insert(Match::new(0, 10, 18));
//! set.insert(Match::new(1, 15, 20));
//! set.merge_overlapping();
//!
//! assert_eq!(set.len(), 1);
//! assert_eq!(set.as_slice()[0].end, 20);
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::todo,
        clippy::unimplemented,
        clippy::panic
    )
)]
#![allow(
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::missing_errors_doc
)]

/// Shared error types for all matching backends.
pub mod error;
/// Internal implementation of the match set type.
#[doc(hidden)]
pub mod match_set;
/// Internal implementation of the match and GPU match types.
#[doc(hidden)]
pub mod match_type;
/// Trait definitions for pattern matching backends.
pub mod matcher;

/// Re-export of the universal error type.
pub use error::Error;
/// Re-export of the result type alias.
pub use error::Result;
/// Re-export of the sorted, deduplicated match collection.
pub use match_set::MatchSet;
/// Re-export of the GPU-internal match representation.
pub use match_type::GpuMatch;
/// Re-export of the match result struct.
pub use match_type::Match;
/// Re-export of the Structure-of-Arrays match batch.
pub use match_type::MatchBatch;
/// Re-export of the block-based matcher trait.
pub use matcher::BlockMatcher;
/// Re-export of the boxed matcher type alias.
pub use matcher::BoxedMatcher;
/// Re-export of the general matcher trait.
pub use matcher::Matcher;

#[cfg(test)]
mod lib_tests {
    use super::*;

    #[test]
    fn match_new() {
        let m = Match::new(42, 10, 20);
        assert_eq!(m.pattern_id, 42);
        assert_eq!(m.start, 10);
        assert_eq!(m.end, 20);
    }

    #[test]
    fn match_ordering() {
        let a = Match::new(0, 5, 10);
        let b = Match::new(0, 10, 15);
        assert!(a < b);
    }

    #[test]
    fn match_equality() {
        let a = Match::new(1, 5, 10);
        let b = Match::new(1, 5, 10);
        assert_eq!(a, b);
    }

    #[test]
    fn match_set_new_empty() {
        let set = MatchSet::new();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn match_set_insert_and_len() {
        let mut set = MatchSet::new();
        set.insert(Match::new(0, 0, 5));
        set.insert(Match::new(1, 10, 15));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn match_set_merge_overlapping() {
        let mut set = MatchSet::new();
        set.insert(Match::new(0, 0, 10));
        set.insert(Match::new(0, 5, 15));
        set.merge_overlapping();
        assert_eq!(set.len(), 1);
        assert_eq!(set.as_slice()[0].end, 15);
    }

    #[test]
    fn match_set_merge_non_overlapping() {
        let mut set = MatchSet::new();
        set.insert(Match::new(0, 0, 5));
        set.insert(Match::new(0, 10, 15));
        set.merge_overlapping();
        assert_eq!(set.len(), 2, "non-overlapping should not merge");
    }

    #[test]
    fn match_set_merge_adjacent() {
        let mut set = MatchSet::new();
        set.insert(Match::new(0, 0, 10));
        set.insert(Match::new(0, 10, 20));
        set.merge_overlapping();
        assert_eq!(set.len(), 1);
        assert_eq!(set.as_slice()[0], Match::new(0, 0, 20));
    }

    #[test]
    fn match_zero_length() {
        let m = Match::new(0, 5, 5);
        assert_eq!(m.start, m.end);
    }

    #[test]
    fn match_field_offsets_are_gpu_compatible() {
        assert_eq!(std::mem::offset_of!(Match, pattern_id), 0);
        assert_eq!(std::mem::offset_of!(Match, start), 4);
        assert_eq!(std::mem::offset_of!(Match, end), 8);
        assert_eq!(std::mem::size_of::<Match>(), 12);
    }

    #[test]
    fn match_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Match>();
    }

    #[test]
    fn match_roundtrips_to_gpumatch() {
        let original = Match::new(7, 100, 200);
        let gpu: GpuMatch = original.into();
        let roundtrip: Match = gpu.into();
        assert_eq!(roundtrip, original);
    }

    #[test]
    fn match_batch_conversion() {
        let matches = vec![Match::new(0, 0, 10), Match::new(1, 5, 15)];
        let batch = MatchBatch::from_slice(&matches);
        assert_eq!(batch.len(), 2);
        assert_eq!(batch.pattern_ids[0], 0);
        assert_eq!(batch.pattern_ids[1], 1);
        assert_eq!(batch.into_vec(), matches);
    }
}
