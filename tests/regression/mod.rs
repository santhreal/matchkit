//! regression tests for matchkit.
//! See TESTING.md for the Santh testing standard.
//!
//! Public-API regressions for bugs fixed in this crate's backlog plus the
//! OOM-safe allocation surface. Each asserts concrete values.

use matchkit::{Match, MatchBatch, MatchSet};

/// Regression: match_type.rs:127 / :115. An inverted range (start > end) has no
/// extent: it is empty, zero-length, and never overlaps a valid range.
#[test]
fn inverted_match_is_empty_zero_length_and_non_overlapping() {
    let inverted = Match::new(1, 10, 5);
    assert!(inverted.is_empty(), "start>end must be empty");
    assert_eq!(inverted.len(), 0, "start>end must have zero length");

    let valid = Match::new(1, 4, 8); // overlaps [5,10] numerically if not guarded
    assert!(
        !inverted.overlaps(&valid),
        "an inverted range must not overlap a valid one"
    );
}

/// Regression: match_type.rs:226. MatchBatch::into_vec must zip the three SoA
/// columns and round-trip an AoS slice exactly, not index one column by
/// another's length.
#[test]
fn match_batch_into_vec_round_trips_aos() {
    let matches = [Match::new(1, 0, 4), Match::new(2, 4, 9), Match::new(3, 9, 20)];
    let batch = MatchBatch::from_slice(&matches);
    assert_eq!(batch.len(), 3);
    let round_tripped = batch.into_vec();
    assert_eq!(round_tripped, matches.to_vec());
}

/// with_capacity with a sane hint preallocates and starts empty (the infallible
/// constructor is only for trusted sizes; hostile sizes go through
/// try_with_capacity, exercised below).
#[test]
fn with_capacity_sane_hint_starts_empty_with_capacity() {
    let batch = MatchBatch::with_capacity(1024);
    assert_eq!(batch.len(), 0);
    assert!(batch.is_empty());
    assert!(batch.pattern_ids.capacity() >= 1024);

    let set = MatchSet::with_capacity(1024);
    assert_eq!(set.len(), 0);
}

/// The OOM-safe constructors succeed for a reasonable size, and a HOSTILE size
/// resolves LOUDLY as `Error::OutOfMemory` instead of aborting the process (the
/// core reason try_with_capacity exists: an untrusted size must never reach the
/// infallible `Vec::with_capacity`, which aborts on allocation failure).
#[test]
fn try_with_capacity_is_reasonable_and_fails_loud_on_hostile_size() {
    let batch = MatchBatch::try_with_capacity(1024).expect("1024 batch capacity fits");
    assert_eq!(batch.len(), 0);

    let set = MatchSet::try_with_capacity(1024).expect("1024 set capacity fits");
    assert_eq!(set.len(), 0);

    // usize::MAX cannot be allocated: try_reserve returns an error, never aborts.
    assert!(
        matches!(
            MatchBatch::try_with_capacity(usize::MAX),
            Err(matchkit::Error::OutOfMemory { .. })
        ),
        "hostile MatchBatch::try_with_capacity must return a loud OutOfMemory"
    );
    assert!(
        matches!(
            MatchSet::try_with_capacity(usize::MAX),
            Err(matchkit::Error::OutOfMemory { .. })
        ),
        "hostile MatchSet::try_with_capacity must return a loud OutOfMemory"
    );
}

/// try_push must grow all three SoA columns atomically and keep them equal
/// length, matching the value a plain push would produce.
#[test]
fn try_push_keeps_soa_columns_consistent() {
    let mut batch = MatchBatch::new();
    for i in 0..50u32 {
        batch.try_push(Match::new(i, i, i + 1)).expect("push under memory pressure-free");
    }
    assert_eq!(batch.len(), 50);
    assert_eq!(batch.pattern_ids.len(), batch.starts.len());
    assert_eq!(batch.starts.len(), batch.ends.len());
    let v = batch.into_vec();
    assert_eq!(v[0], Match::new(0, 0, 1));
    assert_eq!(v[49], Match::new(49, 49, 50));
}
