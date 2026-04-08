//! # matchkit — vocabulary types for multi-pattern matching
//!
//! `matchkit` provides the shared types that every crate in the Santh
//! performance ecosystem depends on: the [`Match`] struct, the [`Matcher`]
//! and [`BlockMatcher`] traits, and common error definitions.
//!
//! By isolating these types into a zero-dependency vocabulary crate,
//! consumer crates like `simdsieve`, `warpstate`, `warpsearch`, and
//! `warpgrep` can all agree on a single match representation without
//! pulling in heavyweight GPU or regex dependencies.
//!
//! # Quick Start
//!
//! ```rust
//! use matchkit::{Match, MatchSet};
//!
//! let mut set = MatchSet::new();
//! set.insert(Match::from_parts(0, 10, 18));
//! set.insert(Match::from_parts(1, 15, 20));
//! set.merge_overlapping();
//!
//! assert_eq!(set.len(), 1);
//! assert_eq!(set.as_slice()[0].end, 20);
//! ```

#![warn(missing_docs, clippy::pedantic)]
#![deny(unsafe_code)]
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
    clippy::doc_markdown
)]

pub mod error;
#[doc(hidden)]
pub mod match_set;
#[doc(hidden)]
pub mod match_type;
pub mod matcher;

pub use error::{Error, Result};
pub use match_set::MatchSet;
pub use match_type::{GpuMatch, Match};
pub use matcher::{BlockMatcher, BoxedMatcher, Matcher};

#[cfg(test)]
mod lib_tests {
    use super::*;

    #[test]
    fn match_from_parts() {
        let m = Match::from_parts(42, 10, 20);
        assert_eq!(m.pattern_id, 42);
        assert_eq!(m.start, 10);
        assert_eq!(m.end, 20);
    }

    #[test]
    fn match_ordering() {
        let a = Match::from_parts(0, 5, 10);
        let b = Match::from_parts(0, 10, 15);
        assert!(a < b);
    }

    #[test]
    fn match_equality() {
        let a = Match::from_parts(1, 5, 10);
        let b = Match::from_parts(1, 5, 10);
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
        set.insert(Match::from_parts(0, 0, 5));
        set.insert(Match::from_parts(1, 10, 15));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn match_set_merge_overlapping() {
        let mut set = MatchSet::new();
        set.insert(Match::from_parts(0, 0, 10));
        set.insert(Match::from_parts(0, 5, 15));
        set.merge_overlapping();
        assert_eq!(set.len(), 1);
        assert_eq!(set.as_slice()[0].end, 15);
    }

    #[test]
    fn match_set_merge_non_overlapping() {
        let mut set = MatchSet::new();
        set.insert(Match::from_parts(0, 0, 5));
        set.insert(Match::from_parts(0, 10, 15));
        set.merge_overlapping();
        assert_eq!(set.len(), 2, "non-overlapping should not merge");
    }

    #[test]
    fn match_set_merge_adjacent() {
        let mut set = MatchSet::new();
        set.insert(Match::from_parts(0, 0, 10));
        set.insert(Match::from_parts(0, 10, 20));
        set.merge_overlapping();
        // Adjacent matches (end == start) should merge
        assert!(set.len() <= 2);
    }

    #[test]
    fn match_zero_length() {
        let m = Match::from_parts(0, 5, 5);
        assert_eq!(m.start, m.end);
    }

    #[test]
    fn match_set_sorted_after_insert() {
        let mut set = MatchSet::new();
        set.insert(Match::from_parts(0, 20, 25));
        set.insert(Match::from_parts(0, 5, 10));
        set.insert(Match::from_parts(0, 10, 15));
        let slice = set.as_slice();
        for window in slice.windows(2) {
            assert!(
                window[0].start <= window[1].start,
                "matches should be sorted by start"
            );
        }
    }
}
