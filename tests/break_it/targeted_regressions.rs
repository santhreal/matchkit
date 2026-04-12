#![allow(clippy::panic, clippy::unwrap_used)]

use matchkit::{GpuMatch, Match, MatchSet};

#[test]
fn matchset_with_u32_max_matches_keeps_extreme_bounds() {
    let mut set = MatchSet::new();
    set.extend([
        Match::new(u32::MAX, u32::MAX - 4, u32::MAX - 1),
        Match::new(u32::MAX - 1, u32::MAX - 3, u32::MAX),
    ]);

    assert_eq!(set.len(), 2, "extreme-offset matches were dropped");
    assert_eq!(set.as_slice()[0].start, u32::MAX - 4);
    assert_eq!(set.as_slice()[1].end, u32::MAX);
}

#[test]
fn merge_adjacent_but_non_overlapping_coalesces_single_span() {
    let mut set = MatchSet::new();
    set.insert(Match::new(7, 0, 5));
    set.insert(Match::new(7, 5, 10));

    set.merge_overlapping();

    assert_eq!(
        set.len(),
        1,
        "adjacent ranges should coalesce into one span"
    );
    assert_eq!(set.as_slice()[0], Match::new(7, 0, 10));
}

#[test]
fn zero_length_match_is_absorbed_by_covering_range() {
    let mut set = MatchSet::new();
    set.insert(Match::new(11, 5, 5));
    set.insert(Match::new(11, 0, 10));

    set.merge_overlapping();

    assert_eq!(
        set.len(),
        1,
        "zero-length boundary marker should be absorbed"
    );
    assert_eq!(set.as_slice()[0], Match::new(11, 0, 10));
}

#[test]
fn gpu_match_roundtrip() {
    let original = Match::new(42, 123, 456);
    let gpu: GpuMatch = original.into();
    let restored: Match = gpu.into();

    assert_eq!(restored, original, "GPU roundtrip corrupted match fields");
}
