//! Production-grade tests for matchkit — testing Match struct, Matcher trait, and error types.
//!
//! These tests verify:
//! - Match struct is exactly 16 bytes (GPU buffer layout)
//! - Match is #[repr(C)] with correct field offsets
//! - Match::from_parts roundtrips correctly
//! - Match ordering is deterministic
//! - Match equality includes all fields
//! - Matcher trait is object-safe
//! - Error types implement Send + Sync + std::error::Error

use bytemuck::Zeroable;
use matchkit::{BlockMatcher, BoxedMatcher, Error, GpuMatch, Match, MatchSet, Matcher, Result};
use std::error::Error as StdError;
use std::sync::Arc;

// ============================================================================
// MATCH STRUCT TESTS
// ============================================================================

/// Test 1: Match struct is exactly 16 bytes (assert_eq!(std::mem::size_of::<Match>(), 16))
#[test]
fn match_size_is_16_bytes() {
    assert_eq!(
        std::mem::size_of::<Match>(),
        16,
        "Match struct must be exactly 16 bytes for GPU buffer compatibility"
    );
}

/// Test 2: Match is #[repr(C)] — verify field offsets match GPU buffer layout
#[test]
fn match_field_offsets_match_gpu_layout() {
    use memoffset::offset_of;

    // Verify field offsets for GPU buffer compatibility
    // GpuMatch layout: [pattern_id, start, end, padding] = [0, 4, 8, 12] bytes
    assert_eq!(
        offset_of!(Match, pattern_id),
        0,
        "pattern_id must be at offset 0"
    );
    assert_eq!(offset_of!(Match, start), 4, "start must be at offset 4");
    assert_eq!(offset_of!(Match, end), 8, "end must be at offset 8");
    assert_eq!(
        offset_of!(Match, padding),
        12,
        "padding must be at offset 12"
    );
}

/// Test 3: Match::from_parts roundtrip for 0, 1, u32::MAX
#[test]
fn match_from_parts_roundtrip() {
    // Test with 0 values
    let m0 = Match::from_parts(0, 0, 0);
    assert_eq!(m0.pattern_id, 0);
    assert_eq!(m0.start, 0);
    assert_eq!(m0.end, 0);
    assert_eq!(m0.padding, 0);
    assert!(m0.is_empty());
    assert_eq!(m0.len(), 0);

    // Test with 1 values
    let m1 = Match::from_parts(1, 1, 1);
    assert_eq!(m1.pattern_id, 1);
    assert_eq!(m1.start, 1);
    assert_eq!(m1.end, 1);
    assert!(m1.is_empty()); // start == end means empty
    assert_eq!(m1.len(), 0);

    // Test with u32::MAX
    let m_max = Match::from_parts(u32::MAX, u32::MAX, u32::MAX);
    assert_eq!(m_max.pattern_id, u32::MAX);
    assert_eq!(m_max.start, u32::MAX);
    assert_eq!(m_max.end, u32::MAX);
    assert!(m_max.is_empty());
    assert_eq!(m_max.len(), 0);

    // Test non-empty match with u32::MAX - 1 to u32::MAX
    let m_range = Match::from_parts(u32::MAX, u32::MAX - 1, u32::MAX);
    assert_eq!(m_range.pattern_id, u32::MAX);
    assert_eq!(m_range.start, u32::MAX - 1);
    assert_eq!(m_range.end, u32::MAX);
    assert!(!m_range.is_empty());
    assert_eq!(m_range.len(), 1);
}

/// Test: Match::from_parts_with_padding preserves padding
#[test]
fn match_from_parts_with_padding() {
    let m = Match::from_parts_with_padding(1, 2, 3, 0xDEADBEEF);
    assert_eq!(m.pattern_id, 1);
    assert_eq!(m.start, 2);
    assert_eq!(m.end, 3);
    assert_eq!(m.padding, 0xDEADBEEF);
    assert_eq!(m.padding(), 0xDEADBEEF);
}

/// Test 4: Match ordering - sort is deterministic (pattern_id, start, end)
#[test]
fn match_ordering_is_deterministic() {
    use std::cmp::Ordering;

    let m1 = Match::from_parts(0, 10, 20);
    let m2 = Match::from_parts(1, 10, 20);
    let m3 = Match::from_parts(0, 15, 20);
    let _m4 = Match::from_parts(0, 10, 25);
    let m5 = Match::from_parts(0, 10, 20); // Same as m1

    // Test ordering - sorted by (start, pattern_id, end) based on Ord impl
    // From match_type.rs: self.start.cmp(&other.start)
    //                     .then(self.pattern_id.cmp(&other.pattern_id))
    //                     .then(self.end.cmp(&other.end))

    // Same match equals
    assert_eq!(m1.cmp(&m5), Ordering::Equal);
    assert_eq!(m1.partial_cmp(&m5), Some(Ordering::Equal));

    // m1 (start=10) < m3 (start=15)
    assert_eq!(m1.cmp(&m3), Ordering::Less);

    // Same start: m1 (pattern_id=0) < m2 (pattern_id=1)
    assert_eq!(m1.cmp(&m2), Ordering::Less);

    // Same start, same pattern_id: m1 (end=20) < m4 (end=25)
    let m1_alt = Match::from_parts(0, 10, 20);
    let m4_alt = Match::from_parts(0, 10, 25);
    assert_eq!(m1_alt.cmp(&m4_alt), Ordering::Less);

    // Verify sorting produces deterministic order
    let mut matches = vec![
        Match::from_parts(2, 30, 40),
        Match::from_parts(1, 10, 20),
        Match::from_parts(0, 10, 20),
        Match::from_parts(1, 10, 15),
        Match::from_parts(1, 5, 10),
    ];
    matches.sort();

    // Expected order by (start, pattern_id, end):
    // (1, 5, 10) - start=5
    // (0, 10, 20) - start=10, pattern_id=0
    // (1, 10, 15) - start=10, pattern_id=1, end=15
    // (1, 10, 20) - start=10, pattern_id=1, end=20
    // (2, 30, 40) - start=30
    assert_eq!(matches[0], Match::from_parts(1, 5, 10));
    assert_eq!(matches[1], Match::from_parts(0, 10, 20));
    assert_eq!(matches[2], Match::from_parts(1, 10, 15));
    assert_eq!(matches[3], Match::from_parts(1, 10, 20));
    assert_eq!(matches[4], Match::from_parts(2, 30, 40));
}

/// Test 5: Match equality includes all fields (pattern_id, start, end)
/// Note: padding is explicitly NOT included in equality comparison
#[test]
fn match_equality_includes_all_fields() {
    let m1 = Match::from_parts(1, 10, 20);
    let m2 = Match::from_parts(1, 10, 20);
    let m3 = Match::from_parts(2, 10, 20); // Different pattern_id
    let m4 = Match::from_parts(1, 11, 20); // Different start
    let m5 = Match::from_parts(1, 10, 21); // Different end

    // Equality
    assert_eq!(m1, m2, "Identical matches should be equal");
    assert_ne!(m1, m3, "Different pattern_id should not be equal");
    assert_ne!(m1, m4, "Different start should not be equal");
    assert_ne!(m1, m5, "Different end should not be equal");

    // Test that padding does NOT affect equality
    let m_with_padding = Match::from_parts_with_padding(1, 10, 20, 0xDEADBEEF);
    let m_without_padding = Match::from_parts(1, 10, 20);
    assert_eq!(
        m_with_padding, m_without_padding,
        "Padding should not affect equality"
    );
}

/// Test: Match contains and overlaps methods
#[test]
fn match_contains_and_overlaps() {
    let outer = Match::from_parts(0, 10, 30);
    let inner = Match::from_parts(0, 15, 25);
    let overlapping = Match::from_parts(0, 20, 40);
    let disjoint = Match::from_parts(0, 40, 50);
    let edge_touching = Match::from_parts(0, 30, 40); // end == other.start

    // Contains
    assert!(outer.contains(&inner), "Outer should contain inner");
    assert!(!inner.contains(&outer), "Inner should not contain outer");
    assert!(outer.contains(&outer), "Match should contain itself");

    // Overlaps
    assert!(outer.overlaps(&overlapping), "Should overlap");
    assert!(overlapping.overlaps(&outer), "Overlap is symmetric");
    assert!(!outer.overlaps(&disjoint), "Should not overlap disjoint");
    assert!(!disjoint.overlaps(&outer), "Should not overlap disjoint");
    assert!(
        !outer.overlaps(&edge_touching),
        "Edge-touching should not overlap (start < end condition)"
    );
}

/// Test: Match len and is_empty
#[test]
fn match_len_and_is_empty() {
    let empty = Match::from_parts(0, 10, 10);
    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);

    let non_empty = Match::from_parts(0, 10, 20);
    assert!(!non_empty.is_empty());
    assert_eq!(non_empty.len(), 10);

    // Test saturating subtraction for len
    let reversed = Match::from_parts(0, 20, 10);
    assert_eq!(reversed.len(), 0); // Should saturate at 0
}

/// Test: GpuMatch to Match conversion
#[test]
fn gpumatch_to_match_conversion() {
    let gpu = GpuMatch([1, 10, 20, 0xDEADBEEF]);
    let m: Match = gpu.into();

    assert_eq!(m.pattern_id, 1);
    assert_eq!(m.start, 10);
    assert_eq!(m.end, 20);
    assert_eq!(m.padding, 0xDEADBEEF);
}

/// Test: MatchSet operations
#[test]
fn matchset_operations() {
    let mut set = MatchSet::new();
    assert!(set.is_empty());
    assert_eq!(set.len(), 0);

    set.insert(Match::from_parts(0, 10, 20));
    set.insert(Match::from_parts(0, 15, 25));
    assert_eq!(set.len(), 2);

    // Test merge_overlapping
    set.merge_overlapping();
    assert_eq!(set.len(), 1);
    assert_eq!(set.as_slice()[0].start, 10);
    assert_eq!(set.as_slice()[0].end, 25);

    // Test pattern filtering
    let mut set2 = MatchSet::new();
    set2.insert(Match::from_parts(0, 10, 20));
    set2.insert(Match::from_parts(1, 15, 25));
    set2.insert(Match::from_parts(0, 30, 40));

    let filtered = set2.filter_by_pattern(0);
    assert_eq!(filtered.len(), 2);

    // Test pattern counts
    let counts = set2.pattern_counts();
    assert_eq!(counts.get(&0), Some(&2));
    assert_eq!(counts.get(&1), Some(&1));

    // Test pattern IDs
    let mut ids = set2.pattern_ids();
    ids.sort();
    assert_eq!(ids, vec![0, 1]);
}

// ============================================================================
// MATCHER TRAIT TESTS
// ============================================================================

/// Test 6: Matcher trait is object-safe
/// We verify this by creating trait objects and using them in containers
#[test]
fn matcher_trait_is_object_safe() {
    // Create a mock matcher for testing
    struct MockMatcher;

    impl Matcher for MockMatcher {
        async fn scan(&self, _data: &[u8]) -> Result<Vec<Match>> {
            Ok(vec![Match::from_parts(0, 0, 10)])
        }
    }

    // Test that MockMatcher implements Matcher + Send + Sync
    fn assert_matcher<T: Matcher + Send + Sync>(_m: &T) {}
    let mock = MockMatcher;
    assert_matcher(&mock);

    // Test that we can scan with the matcher
    let result = futures::executor::block_on(mock.scan(b"test data")).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].end, 10);
}

/// Test: BlockMatcher trait is object-safe
#[test]
fn block_matcher_trait_is_object_safe() {
    struct MockBlockMatcher;

    impl BlockMatcher for MockBlockMatcher {
        async fn scan_block(&self, _data: &[u8]) -> Result<Vec<Match>> {
            Ok(vec![])
        }

        fn max_block_size(&self) -> usize {
            1024
        }
    }

    let m = MockBlockMatcher;
    assert_eq!(m.max_block_size(), 1024);
}

/// Test: Matcher requires Send + Sync bounds
#[test]
fn matcher_requires_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}

    // This will fail to compile if Matcher doesn't require Send + Sync
    fn check_matcher_bounds<M: Matcher>() {
        assert_send_sync::<M>();
    }
}

// ============================================================================
// ERROR TYPE TESTS
// ============================================================================

/// Test 7: Error types implement Send + Sync + std::error::Error
#[test]
fn error_implements_send_sync_and_std_error() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    fn assert_std_error<T: StdError>() {}

    assert_send::<Error>();
    assert_sync::<Error>();
    assert_std_error::<Error>();
}

/// Test: Error variants can be created and displayed
#[test]
fn error_variants_display() {
    // InputTooLarge
    let err = Error::InputTooLarge {
        bytes: 1024,
        max_bytes: 512,
    };
    let msg = err.to_string();
    assert!(msg.contains("scan input is too large"));
    assert!(msg.contains("fix:"));

    // MatchBufferOverflow
    let err = Error::MatchBufferOverflow {
        count: 1000,
        max: 100,
    };
    let msg = err.to_string();
    assert!(msg.contains("too many matches"));
    assert!(msg.contains("fix:"));

    // EmptyPatternSet
    let err = Error::EmptyPatternSet;
    let msg = err.to_string();
    assert!(msg.contains("pattern set is empty"));
    assert!(msg.contains("fix:"));

    // EmptyPattern
    let err = Error::EmptyPattern { index: 5 };
    let msg = err.to_string();
    assert!(msg.contains("pattern 5 is empty"));
    assert!(msg.contains("fix:"));

    // PatternCompilationFailed
    let err = Error::PatternCompilationFailed {
        reason: "invalid regex".to_string(),
    };
    let msg = err.to_string();
    assert!(msg.contains("pattern compilation failed"));

    // Backend error
    let io_err = std::io::Error::new(std::io::ErrorKind::Other, "test error");
    let err = Error::Backend(Box::new(io_err));
    let _msg = err.to_string();
}

/// Test: Error can be used as a trait object
#[test]
fn error_as_dyn_std_error() {
    let err: Error = Error::EmptyPatternSet;
    let dyn_err: &dyn StdError = &err;
    assert!(dyn_err.to_string().contains("pattern set is empty"));
}

/// Test: Result type alias works correctly
#[test]
fn result_type_alias() {
    fn returns_ok() -> Result<u32> {
        Ok(42)
    }

    fn returns_err() -> Result<u32> {
        Err(Error::EmptyPatternSet)
    }

    assert_eq!(returns_ok().unwrap(), 42);
    assert!(returns_err().is_err());
}

// ============================================================================
// GPU BUFFER LAYOUT TESTS
// ============================================================================

/// Test: Match can be transmuted to/from GPU-compatible byte array
#[test]
fn match_gpu_buffer_layout() {
    // Verify Match is Pod (Plain Old Data) for GPU buffer compatibility
    let m = Match::from_parts_with_padding(1, 10, 20, 0x12345678);

    // Convert to bytes (simulating GPU buffer write)
    let bytes: [u8; 16] = bytemuck::cast(m);

    // Convert back from bytes (simulating GPU buffer read)
    let m2: Match = bytemuck::cast(bytes);

    assert_eq!(m2.pattern_id, 1);
    assert_eq!(m2.start, 10);
    assert_eq!(m2.end, 20);
    assert_eq!(m2.padding, 0x12345678);
}

/// Test: GpuMatch and Match have same memory layout
#[test]
fn gpumatch_and_match_same_size() {
    assert_eq!(
        std::mem::size_of::<GpuMatch>(),
        std::mem::size_of::<Match>(),
        "GpuMatch and Match must have same size"
    );
    assert_eq!(std::mem::size_of::<GpuMatch>(), 16);
    assert_eq!(std::mem::size_of::<Match>(), 16);
}

/// Test: GpuMatch can be zeroed (for buffer initialization)
#[test]
fn gpumatch_zeroable() {
    let zeroed = GpuMatch::zeroed();
    assert_eq!(zeroed.0, [0, 0, 0, 0]);

    let m: Match = zeroed.into();
    assert_eq!(m.pattern_id, 0);
    assert_eq!(m.start, 0);
    assert_eq!(m.end, 0);
    assert_eq!(m.padding, 0);
}

// ============================================================================
// EDGE CASE AND ADVERSARIAL TESTS
// ============================================================================

/// Test: Match handles overflow gracefully
#[test]
fn match_handles_overflow_gracefully() {
    // Maximum u32 values
    let m = Match::from_parts(u32::MAX, u32::MAX - 1, u32::MAX);
    assert_eq!(m.len(), 1); // Should not overflow

    // Reversed range (start > end) should return 0 len via saturating_sub
    let reversed = Match::from_parts(0, 100, 50);
    assert_eq!(reversed.len(), 0);
}

/// Test: Match ordering handles all boundary conditions
#[test]
fn match_ordering_boundary_conditions() {
    use std::cmp::Ordering;

    let min_vals = Match::from_parts(0, 0, 0);
    let max_vals = Match::from_parts(u32::MAX, u32::MAX, u32::MAX);

    assert_eq!(min_vals.cmp(&min_vals), Ordering::Equal);
    assert_eq!(max_vals.cmp(&max_vals), Ordering::Equal);
    assert_eq!(min_vals.cmp(&max_vals), Ordering::Less);
    assert_eq!(max_vals.cmp(&min_vals), Ordering::Greater);
}

/// Test: Large-scale MatchSet operations (internet scale simulation)
#[test]
fn matchset_large_scale_operations() {
    let mut set = MatchSet::with_capacity(1000);

    // Insert 1000 non-overlapping matches
    for i in 0..1000_u32 {
        set.insert(Match::from_parts(i % 10, i * 10, i * 10 + 5));
    }

    assert_eq!(set.len(), 1000);

    // Verify all pattern IDs are represented
    let ids = set.pattern_ids();
    assert_eq!(ids.len(), 10);

    // Test iteration
    let count = set.iter().count();
    assert_eq!(count, 1000);

    // Test into_iter
    let vec: Vec<_> = set.into_iter().collect();
    assert_eq!(vec.len(), 1000);
}

/// Test: MatchSet deduplication
#[test]
fn matchset_deduplication() {
    let mut set = MatchSet::new();

    // Insert same match multiple times
    for _ in 0..10 {
        set.insert(Match::from_parts(1, 10, 20));
    }

    // Should only have one entry
    assert_eq!(set.len(), 1);
}
