//! Exhaustive adversarial tests for matchkit vocabulary types.
//!
//! These tests verify: GPU buffer layout contracts, trait object safety,
//! MatchSet behavior on edge-case inputs, error message quality, and
//! bytemuck serialization round-trips.

use matchkit::error::Error;
use matchkit::{BlockMatcher, GpuMatch, Match, MatchSet, Matcher};
use std::collections::HashMap;

// ============================================================================
// Match struct — GPU buffer layout is CRITICAL
// ============================================================================

#[test]
fn match_new_zero() {
    let m = Match::new(0, 0, 0);
    assert_eq!(m.pattern_id, 0, "pattern_id should be 0 for zero match");
    assert_eq!(m.start, 0, "start should be 0 for zero match");
    assert_eq!(m.end, 0, "end should be 0 for zero match");
    assert!(m.is_empty(), "zero-length match should report is_empty");
    assert_eq!(m.len(), 0, "zero-length match len should be 0");
}

#[test]
fn match_new_max_values() {
    let m = Match::new(u32::MAX, u32::MAX, u32::MAX);
    assert_eq!(m.pattern_id, u32::MAX, "pattern_id should hold max u32");
    assert_eq!(m.start, u32::MAX, "start should hold max u32");
    assert_eq!(m.end, u32::MAX, "end should hold max u32");
}

#[test]
fn match_is_exactly_12_bytes() {
    assert_eq!(
        std::mem::size_of::<Match>(),
        12,
        "Match MUST be exactly 12 bytes for optimal VRAM usage"
    );
}

#[test]
fn match_alignment_is_4() {
    assert_eq!(
        std::mem::align_of::<Match>(),
        4,
        "Match MUST be 4-byte aligned for GPU buffer compatibility"
    );
}

#[test]
fn match_fields_publicly_accessible() {
    let m = Match::new(7, 12, 34);
    assert_eq!(
        m.pattern_id, 7,
        "pattern_id field should be publicly accessible"
    );
    assert_eq!(m.start, 12, "start field should be publicly accessible");
    assert_eq!(m.end, 34, "end field should be publicly accessible");
}

#[test]
fn match_len_saturating_sub() {
    // Adversarial: start > end is invalid input but must not underflow
    let m = Match::new(0, 100, 50);
    assert_eq!(
        m.len(),
        0,
        "len must use saturating_sub to prevent underflow"
    );
}

#[test]
fn match_is_empty_when_start_equals_end() {
    let m = Match::new(0, 42, 42);
    assert!(m.is_empty(), "start == end must mean is_empty == true");
    assert_eq!(m.len(), 0, "start == end must produce len == 0");
}

#[test]
fn match_contains_same_range() {
    let a = Match::new(0, 5, 10);
    let b = Match::new(0, 5, 10);
    assert!(a.contains(&b), "identical ranges must contain each other");
    assert!(
        b.contains(&a),
        "identical ranges must contain each other symmetrically"
    );
}

#[test]
fn match_contains_fully_enclosed() {
    let outer = Match::new(0, 0, 100);
    let inner = Match::new(0, 10, 90);
    assert!(outer.contains(&inner), "outer must contain inner");
    assert!(!inner.contains(&outer), "inner must NOT contain outer");
}

#[test]
fn match_contains_not_contained() {
    let a = Match::new(0, 0, 10);
    let b = Match::new(0, 20, 30);
    assert!(
        !a.contains(&b),
        "disjoint ranges must not contain each other"
    );
    assert!(
        !b.contains(&a),
        "disjoint ranges must not contain each other symmetrically"
    );
}

#[test]
fn match_overlaps_partial() {
    let a = Match::new(0, 0, 5);
    let b = Match::new(0, 3, 8);
    assert!(a.overlaps(&b), "partial overlap must be detected");
    assert!(b.overlaps(&a), "overlaps must be symmetric");
}

#[test]
fn match_overlaps_adjacent_no_overlap() {
    let a = Match::new(0, 0, 5);
    let b = Match::new(0, 5, 10);
    assert!(
        !a.overlaps(&b),
        "adjacent ranges [0,5) and [5,10) must NOT overlap"
    );
    assert!(!b.overlaps(&a), "non-overlap must be symmetric");
}

#[test]
fn match_overlaps_one_inside_other() {
    let outer = Match::new(0, 0, 100);
    let inner = Match::new(0, 10, 20);
    assert!(
        outer.overlaps(&inner),
        "outer must overlap with contained inner"
    );
    assert!(inner.overlaps(&outer), "overlap must be symmetric");
}

// ============================================================================
// Matcher trait
// ============================================================================

struct DummyMatcher;

#[async_trait::async_trait]
impl Matcher for DummyMatcher {
    async fn scan(&self, _data: &[u8]) -> matchkit::Result<Vec<Match>> {
        Ok(vec![])
    }
}

#[test]
fn matcher_trait_is_send_sync() {
    fn assert_send_sync<T: Matcher + Send + Sync>() {}
    assert_send_sync::<DummyMatcher>();
}

// ============================================================================
// BlockMatcher trait
// ============================================================================

struct DummyBlockMatcher;

#[async_trait::async_trait]
impl BlockMatcher for DummyBlockMatcher {
    async fn scan_block(&self, _data: &[u8]) -> matchkit::Result<Vec<Match>> {
        Ok(vec![])
    }

    fn max_block_size(&self) -> usize {
        4096
    }
}

#[test]
fn block_matcher_trait_is_send_sync() {
    fn assert_send_sync<T: BlockMatcher + Send + Sync>() {}
    assert_send_sync::<DummyBlockMatcher>();
}

// ============================================================================
// MatchSet
// ============================================================================

#[test]
fn matchset_empty() {
    let set = MatchSet::new();
    assert!(set.is_empty(), "new MatchSet must be empty");
    assert_eq!(set.len(), 0, "empty MatchSet must have len 0");
    assert!(
        set.as_slice().is_empty(),
        "empty MatchSet slice must be empty"
    );
    assert!(
        set.into_vec().is_empty(),
        "empty MatchSet into_vec must be empty"
    );
}

#[test]
fn matchset_single_match() {
    let mut set = MatchSet::new();
    set.insert(Match::new(0, 0, 5));
    assert!(
        !set.is_empty(),
        "MatchSet with one element must not be empty"
    );
    assert_eq!(set.len(), 1, "MatchSet must report len == 1");
    assert_eq!(
        set.as_slice()[0],
        Match::new(0, 0, 5),
        "single match must be preserved"
    );
}

#[test]
fn matchset_ten_thousand_matches() {
    let mut set = MatchSet::new();
    let matches: Vec<Match> = (0..10_000u32)
        .map(|i| Match::new(i % 100, i, i + 1))
        .collect();
    set.extend(matches);
    assert_eq!(set.len(), 10_000, "MatchSet must hold 10K distinct matches");
    let slice = set.as_slice();
    assert!(
        slice.windows(2).all(|w| w[0] <= w[1]),
        "10K matches must be sorted after extend"
    );
}

#[test]
fn matchset_insert_dedups_duplicates() {
    let mut set = MatchSet::new();
    set.insert(Match::new(0, 5, 10));
    set.insert(Match::new(0, 5, 10));
    set.insert(Match::new(0, 5, 10));
    assert_eq!(set.len(), 1, "duplicate insertions must be deduplicated");
}

#[test]
fn matchset_sorts_by_position() {
    let mut set = MatchSet::new();
    set.insert(Match::new(0, 30, 40));
    set.insert(Match::new(0, 10, 20));
    set.insert(Match::new(0, 20, 30));
    let slice = set.as_slice();
    assert_eq!(slice[0].start, 10, "first match must have smallest start");
    assert_eq!(
        slice[1].start, 20,
        "matches must be sorted by start position"
    );
    assert_eq!(
        slice[2].start, 30,
        "matches must be sorted by start position"
    );
}

#[test]
fn matchset_extend_sorts_and_dedups() {
    let mut set = MatchSet::new();
    set.extend([
        Match::new(0, 50, 60),
        Match::new(0, 10, 20),
        Match::new(0, 50, 60), // duplicate
        Match::new(0, 30, 40),
    ]);
    assert_eq!(set.len(), 3, "extend must deduplicate and sort");
    let slice = set.as_slice();
    assert_eq!(slice[0].start, 10);
    assert_eq!(slice[1].start, 30);
    assert_eq!(slice[2].start, 50);
}

#[test]
fn matchset_merge_overlapping() {
    let mut set = MatchSet::new();
    set.extend([
        Match::new(0, 0, 5),
        Match::new(1, 3, 8),
        Match::new(2, 10, 15),
        Match::new(3, 12, 18),
    ]);
    set.merge_overlapping();
    let slice = set.as_slice();
    assert_eq!(slice.len(), 2, "overlapping groups must be merged");
    assert_eq!(slice[0].start, 0, "first merged group starts at 0");
    assert_eq!(
        slice[0].end, 8,
        "first merged group ends at max of overlaps"
    );
    assert_eq!(slice[1].start, 10, "second merged group starts at 10");
    assert_eq!(slice[1].end, 18, "second merged group ends at 18");
    for pair in slice.windows(2) {
        assert!(
            !pair[0].overlaps(&pair[1]),
            "no overlaps must remain after merge"
        );
    }
}

#[test]
fn matchset_merge_overlapping_empty_no_panic() {
    let mut set = MatchSet::new();
    set.merge_overlapping();
    assert!(set.is_empty(), "merge on empty set must remain empty");
}

#[test]
fn matchset_merge_overlapping_single_no_panic() {
    let mut set = MatchSet::new();
    set.insert(Match::new(0, 0, 5));
    set.merge_overlapping();
    assert_eq!(set.len(), 1, "merge on single element must preserve it");
}

#[test]
fn matchset_filter_by_pattern_empty() {
    let set = MatchSet::new();
    let filtered = set.filter_by_pattern(0);
    assert!(
        filtered.is_empty(),
        "filtering empty set must produce empty set"
    );
}

#[test]
fn matchset_filter_by_pattern_no_matches() {
    let mut set = MatchSet::new();
    set.insert(Match::new(0, 0, 5));
    set.insert(Match::new(1, 10, 15));
    let filtered = set.filter_by_pattern(99);
    assert!(
        filtered.is_empty(),
        "filtering nonexistent pattern must produce empty set"
    );
}

#[test]
fn matchset_pattern_counts_empty() {
    let set = MatchSet::new();
    let counts: HashMap<u32, usize> = set.pattern_counts();
    assert!(
        counts.is_empty(),
        "pattern_counts on empty set must be empty"
    );
}

#[test]
fn matchset_pattern_ids_empty() {
    let set = MatchSet::new();
    let ids = set.pattern_ids();
    assert!(ids.is_empty(), "pattern_ids on empty set must be empty");
}

#[test]
fn matchset_into_vec_consumes_correctly() {
    let mut set = MatchSet::new();
    set.extend([Match::new(0, 0, 5), Match::new(1, 10, 15)]);
    let vec = set.into_vec();
    assert_eq!(vec.len(), 2, "into_vec must return all matches");
    assert_eq!(vec[0], Match::new(0, 0, 5));
    assert_eq!(vec[1], Match::new(1, 10, 15));
}

// ============================================================================
// Error types — all variants constructible and actionable
// ============================================================================

#[test]
fn error_input_too_large_actionable() {
    let e = Error::InputTooLarge {
        bytes: 100,
        max_bytes: 50,
    };
    let msg = e.to_string();
    assert!(
        msg.contains("fix:"),
        "InputTooLarge error must contain actionable 'fix:' hint"
    );
    assert!(msg.contains("100"), "error must mention actual bytes");
    assert!(msg.contains("50"), "error must mention max bytes");
}

#[test]
fn error_match_buffer_overflow_actionable() {
    let e = Error::MatchBufferOverflow {
        count: 1000,
        max: 500,
    };
    let msg = e.to_string();
    assert!(
        msg.contains("fix:"),
        "MatchBufferOverflow error must contain actionable 'fix:' hint"
    );
    assert!(msg.contains("1000"), "error must mention actual count");
    assert!(msg.contains("500"), "error must mention max count");
}

#[test]
fn error_empty_pattern_set_actionable() {
    let e = Error::EmptyPatternSet;
    let msg = e.to_string();
    assert!(
        msg.contains("fix:"),
        "EmptyPatternSet error must contain actionable 'fix:' hint"
    );
}

#[test]
fn error_empty_pattern_actionable() {
    let e = Error::EmptyPattern { index: 3 };
    let msg = e.to_string();
    assert!(
        msg.contains("fix:"),
        "EmptyPattern error must contain actionable 'fix:' hint"
    );
    assert!(msg.contains("3"), "error must mention pattern index");
}

#[test]
fn error_pattern_compilation_failed_actionable() {
    let e = Error::PatternCompilationFailed {
        reason: "bad regex".into(),
    };
    let msg = e.to_string();
    assert!(
        msg.contains("bad regex"),
        "error must include underlying reason"
    );
}

#[test]
fn error_backend_actionable() {
    let inner = std::io::Error::other("gpu timeout");
    let e = Error::Backend(Box::new(inner));
    let msg = e.to_string();
    assert!(
        msg.contains("gpu timeout"),
        "Backend error must surface underlying message"
    );
}

// ============================================================================
// Serialization — bytemuck round-trips
// ============================================================================

#[test]
fn match_roundtrips_through_bytemuck() {
    let original = Match::new(7, 100, 200);
    let bytes = bytemuck::bytes_of(&original);
    assert_eq!(bytes.len(), 12, "Match must serialize to exactly 12 bytes");

    let restored: &Match = bytemuck::from_bytes(bytes);
    assert_eq!(
        restored.pattern_id, 7,
        "round-trip must preserve pattern_id"
    );
    assert_eq!(restored.start, 100, "round-trip must preserve start");
    assert_eq!(restored.end, 200, "round-trip must preserve end");
}

#[test]
fn match_slice_casts_to_byte_slice() {
    let matches = [Match::new(0, 0, 5), Match::new(1, 5, 10)];
    let bytes: &[u8] = bytemuck::cast_slice(&matches);
    assert_eq!(bytes.len(), 24, "2 matches × 12 bytes must equal 24 bytes");

    // Verify native-endian layout: first 4 bytes are pattern_id of first match
    let pattern_id_bytes = &bytes[0..4];
    let expected = 0u32.to_ne_bytes();
    assert_eq!(
        pattern_id_bytes, expected,
        "byte layout must match native endianness"
    );
}

#[test]
fn match_slice_casts_from_byte_slice() {
    let original = [Match::new(3, 10, 20), Match::new(4, 30, 40)];
    let bytes: &[u8] = bytemuck::cast_slice(&original);
    let restored: &[Match] = bytemuck::cast_slice(bytes);
    assert_eq!(restored.len(), 2, "cast back must yield 2 matches");
    assert_eq!(restored[0], Match::new(3, 10, 20));
    assert_eq!(restored[1], Match::new(4, 30, 40));
}

#[test]
fn gpumatch_and_match_layout_equivalent() {
    assert_eq!(
        std::mem::size_of::<GpuMatch>(),
        std::mem::size_of::<Match>(),
        "GpuMatch and Match must have identical size for layout equivalence"
    );
    assert_eq!(
        std::mem::align_of::<GpuMatch>(),
        std::mem::align_of::<Match>(),
        "GpuMatch and Match must have identical alignment for layout equivalence"
    );
}

#[test]
fn gpumatch_to_match_conversion_preserves_all_fields() {
    let gpu = GpuMatch::new(1, 2, 3);
    let m: Match = gpu.into();
    assert_eq!(
        m.pattern_id, 1,
        "conversion must map field [0] to pattern_id"
    );
    assert_eq!(m.start, 2, "conversion must map field [1] to start");
    assert_eq!(m.end, 3, "conversion must map field [2] to end");
}

#[test]
fn adversarial_match_end_before_start() {
    let m = Match::new(0, 100, 50); // end < start
    assert_eq!(m.len(), 0); // saturating sub handles it
    assert!(!m.is_empty()); // start != end
}

#[test]
fn adversarial_matchset_out_of_order_insertion() {
    let mut set = MatchSet::new();
    // Test that extend handles totally random inserts and still dedups
    set.extend([
        Match::new(2, 50, 60),
        Match::new(0, 10, 20),
        Match::new(1, 30, 40),
        Match::new(0, 10, 20), // duplicate
    ]);
    let slice = set.as_slice();
    assert_eq!(slice.len(), 3);
    for pair in slice.windows(2) {
        assert!(pair[0] <= pair[1]);
    }
}

#[test]
fn match_overlap_overflow() {
    let a = Match::new(0, u32::MAX - 10, u32::MAX - 5);
    let b = Match::new(0, u32::MAX - 8, u32::MAX);
    assert!(
        a.overlaps(&b),
        "must correctly detect overlap near u32::MAX"
    );
    assert!(b.overlaps(&a), "overlap near u32::MAX must be symmetric");

    let c = Match::new(0, u32::MAX - 5, u32::MAX);
    assert!(
        !a.overlaps(&c),
        "must handle adjacent boundaries at u32::MAX properly"
    );
}

#[test]
fn match_len_overflow() {
    let a = Match::new(0, u32::MAX - 100, u32::MAX);
    assert_eq!(
        a.len(),
        100,
        "length near u32::MAX must be correctly calculated"
    );

    let b = Match::new(0, 10, u32::MAX);
    assert_eq!(
        b.len(),
        u32::MAX - 10,
        "large length spanning to u32::MAX must not overflow"
    );
}

#[test]
fn matchset_merge_overflow() {
    let mut set = MatchSet::new();
    set.extend([
        Match::new(0, u32::MAX - 20, u32::MAX - 10),
        Match::new(1, u32::MAX - 15, u32::MAX - 5),
        Match::new(2, u32::MAX - 8, u32::MAX),
    ]);
    set.merge_overlapping();
    let slice = set.as_slice();
    assert_eq!(
        slice.len(),
        1,
        "overlapping regions near u32::MAX should all merge into one"
    );
    assert_eq!(slice[0].start, u32::MAX - 20);
    assert_eq!(slice[0].end, u32::MAX);
}

#[test]
fn match_u32_max_boundary_overflows() {
    // Tests behavior when matching limits directly hit u32 boundaries
    let m1 = Match::new(u32::MAX, u32::MAX - 10, u32::MAX);
    let m2 = Match::new(u32::MAX, u32::MAX - 5, u32::MAX);

    assert_eq!(m1.len(), 10, "m1 length should not overflow");
    assert_eq!(m2.len(), 5, "m2 length should not overflow");

    assert!(m1.overlaps(&m2), "Overlaps near u32 boundary must work");
    assert!(m1.contains(&m2), "Contains near u32 boundary must work");
    assert!(
        !m2.contains(&m1),
        "Reverse contains near u32 boundary must work properly"
    );

    let mut set = MatchSet::new();
    set.insert(m1);
    set.insert(m2);
    set.merge_overlapping();
    assert_eq!(set.len(), 1, "Boundary overlaps should merge correctly");
    assert_eq!(
        set.as_slice()[0].end,
        u32::MAX,
        "Merged boundary should hit exact u32 limit"
    );
}

#[test]
fn matchset_adversarial_zero_length() {
    let mut set = MatchSet::new();
    // Inserting hundreds of zero-length matches
    for i in 0..100 {
        set.insert(Match::new(i % 10, 50, 50));
    }
    set.merge_overlapping();
    assert_eq!(
        set.len(),
        10,
        "Should only deduplicate by identical pattern/start/end"
    );
    let counts = set.pattern_counts();
    for i in 0..10 {
        assert_eq!(
            *counts.get(&i).unwrap(),
            1,
            "Each pattern should be present exactly once"
        );
    }
}

#[test]
fn matchset_adversarial_pattern_limits() {
    // Exact pattern count limits at 8, 16, 256
    for &limit in &[8, 16, 256] {
        let mut set = MatchSet::new();
        for i in 0..limit {
            set.insert(Match::new(i as u32, i as u32 * 10, i as u32 * 10 + 5));
        }
        assert_eq!(
            set.len(),
            limit as usize,
            "MatchSet should handle exactly {} distinct patterns",
            limit
        );
        let ids = set.pattern_ids();
        assert_eq!(
            ids.len(),
            limit as usize,
            "Should extract exactly {} distinct pattern IDs",
            limit
        );
        assert_eq!(
            ids.last().unwrap(),
            &(limit as u32 - 1),
            "Highest pattern ID must match"
        );
    }
}

#[test]
fn matchset_adversarial_bytes_all_zero_and_ones() {
    let mut set = MatchSet::new();
    // Simulate inputs from scanning an all zero buffer or an all 0xFF buffer
    let m1 = Match::new(0, 0, 0); // 0 bytes match
    let m2 = Match::new(0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF); // All 1s

    set.insert(m1);
    set.insert(m2);
    set.merge_overlapping();

    assert_eq!(
        set.len(),
        2,
        "Disjoint edge-case bytes must merge as distinct matches"
    );

    // Simulate maximizing hash collisions or identical bytes pattern counts
    for i in 0..500 {
        // Pattern ID is alternating
        let pattern_id = if i % 2 == 0 { 0x00000000 } else { 0xFFFFFFFF };
        set.insert(Match::new(pattern_id, 10, 20));
    }
    set.merge_overlapping();
    assert_eq!(
        set.len(),
        3,
        "Alternating patterns on exact same range must only dedup to 2 extra distinct entries"
    );
}
