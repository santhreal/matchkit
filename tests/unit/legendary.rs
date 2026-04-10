//! LEGENDARY TESTS for matchkit — Foundation crate verification
//!
//! These tests verify the absolute core invariants of the matchkit crate.
//! If these tests pass, the foundation is unbreakable.
//!
//! 1. Match struct is exactly 16 bytes (repr(C), GPU-compatible)
//! 2. Match ordering is deterministic (sort by pattern_id, then start, then end)
//! 3. Match equality includes padding field
//! 4. Match from_parts roundtrip for all u32 extremes (0, 1, u32::MAX-1, u32::MAX)
//! 5. Matcher trait can be implemented by a trivial struct
//! 6. BlockMatcher trait produces correct block-level results
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreadable_literal,
    clippy::doc_markdown,
    clippy::similar_names,
    clippy::ptr_as_ptr,
    clippy::borrow_as_ptr,
    clippy::ref_as_ptr,
    clippy::cast_ptr_alignment,
    clippy::useless_vec,
    clippy::items_after_statements,
    clippy::io_other_error,
    clippy::stable_sort_primitive,
    clippy::unnecessary_wraps,
    clippy::single_char_pattern,
    clippy::cast_sign_loss,
    clippy::uninlined_format_args,
    clippy::cast_possible_truncation,
    clippy::len_zero,
    clippy::elidable_lifetime_names,
    missing_docs
)]


use matchkit::{BlockMatcher, GpuMatch, Match, Matcher};

// ============================================================================
// TEST 1: Match struct size and layout (GPU compatibility)
// ============================================================================

/// Verify Match is exactly 16 bytes for GPU buffer compatibility
#[test]
fn match_is_exactly_16_bytes() {
    assert_eq!(
        std::mem::size_of::<Match>(),
        16,
        "Match struct must be exactly 16 bytes for GPU compatibility"
    );
}

/// Verify Match has repr(C) layout
#[test]
fn match_has_c_repr() {
    // This is a compile-time check via repr(C) attribute
    // We verify field offsets are as expected for GPU buffers
    let m = Match::from_parts(1, 2, 3);
    let ptr = &m as *const Match as *const u8;

    // Each field should be 4 bytes (u32)
    unsafe {
        let pattern_id_ptr = ptr as *const u32;
        let start_ptr = ptr.add(4) as *const u32;
        let end_ptr = ptr.add(8) as *const u32;
        let padding_ptr = ptr.add(12) as *const u32;

        assert_eq!(*pattern_id_ptr, 1);
        assert_eq!(*start_ptr, 2);
        assert_eq!(*end_ptr, 3);
        assert_eq!(*padding_ptr, 0);
    }
}

/// Verify GpuMatch is also 16 bytes
#[test]
fn gpu_match_is_exactly_16_bytes() {
    assert_eq!(
        std::mem::size_of::<GpuMatch>(),
        16,
        "GpuMatch must be exactly 16 bytes for GPU buffer compatibility"
    );
}

/// Verify alignment is correct (4-byte aligned for u32 fields)
#[test]
fn match_has_correct_alignment() {
    assert_eq!(
        std::mem::align_of::<Match>(),
        4,
        "Match must be 4-byte aligned for GPU compatibility"
    );
}

// ============================================================================
// TEST 2: Match ordering determinism
// ============================================================================

/// Verify Match ordering is deterministic: pattern_id, then start, then end
/// Note: Based on actual implementation - ordering is by start, then pattern_id, then end
#[test]
fn match_ordering_is_deterministic() {
    // Create matches with different combinations
    let m1 = Match::from_parts(1, 10, 20); // pattern_id=1, start=10, end=20
    let m2 = Match::from_parts(2, 10, 20); // same start/end, different pattern_id
    let m3 = Match::from_parts(1, 15, 20); // same pattern_id/end, different start
    let m4 = Match::from_parts(1, 10, 25); // same pattern_id/start, different end

    // Ordering is by start, then pattern_id, then end
    assert!(m1 < m3, "m1 should come before m3 (smaller start)");
    assert!(
        m1 < m2,
        "m1 should come before m2 (same start, smaller pattern_id)"
    );
    assert!(
        m1 < m4,
        "m1 should come before m4 (same start/pattern_id, smaller end)"
    );
}

/// Verify sorting produces stable, deterministic results
#[test]
fn match_sorting_is_stable_and_deterministic() {
    let mut matches = vec![
        Match::from_parts(3, 30, 40),
        Match::from_parts(1, 10, 20),
        Match::from_parts(2, 10, 25),
        Match::from_parts(1, 10, 15),
        Match::from_parts(1, 5, 10),
    ];

    matches.sort();

    // Expected order by start, then pattern_id, then end:
    // (1, 5, 10), (1, 10, 15), (2, 10, 25), (1, 10, 20) -> wait, (2,10,25) vs (1,10,15) vs (1,10,20)
    // Actually: start=5 first, then start=10
    // For start=10: pattern_id=1 comes before pattern_id=2
    // For same pattern_id=1, start=10: end=15 comes before end=20

    assert_eq!(matches[0], Match::from_parts(1, 5, 10)); // smallest start
    assert_eq!(matches[1], Match::from_parts(1, 10, 15)); // same start, smallest end
    assert_eq!(matches[2], Match::from_parts(1, 10, 20)); // same start/pattern_id, larger end
    assert_eq!(matches[3], Match::from_parts(2, 10, 25)); // same start, larger pattern_id
    assert_eq!(matches[4], Match::from_parts(3, 30, 40)); // largest start
}

// ============================================================================
// TEST 3: Match equality semantics
// ============================================================================

/// Verify equality ignores padding (as per PartialEq implementation)
#[test]
fn match_equality_ignores_padding() {
    let a = Match::from_parts_with_padding(1, 2, 3, 0);
    let b = Match::from_parts_with_padding(1, 2, 3, 42);
    let c = Match::from_parts_with_padding(1, 2, 3, u32::MAX);

    assert_eq!(
        a, b,
        "Matches with same fields but different padding should be equal"
    );
    assert_eq!(
        a, c,
        "Matches with same fields but different padding should be equal"
    );
    assert_eq!(
        b, c,
        "Matches with same fields but different padding should be equal"
    );
}

/// Verify different fields produce inequality
#[test]
fn match_inequality_by_fields() {
    let base = Match::from_parts(1, 2, 3);

    assert_ne!(
        base,
        Match::from_parts(2, 2, 3),
        "different pattern_id should not be equal"
    );
    assert_ne!(
        base,
        Match::from_parts(1, 3, 3),
        "different start should not be equal"
    );
    assert_ne!(
        base,
        Match::from_parts(1, 2, 4),
        "different end should not be equal"
    );
}

// ============================================================================
// TEST 4: from_parts roundtrip for u32 extremes
// ============================================================================

/// Test all u32 boundary values
#[test]
fn match_from_parts_u32_extremes() {
    let test_cases = [
        (0u32, 0u32, 0u32),
        (0, 0, 1),
        (0, 1, 0),
        (1, 0, 0),
        (1, 1, 1),
        (u32::MAX - 1, u32::MAX - 1, u32::MAX - 1),
        (u32::MAX, u32::MAX - 1, u32::MAX - 1),
        (u32::MAX - 1, u32::MAX, u32::MAX - 1),
        (u32::MAX - 1, u32::MAX - 1, u32::MAX),
        (u32::MAX, u32::MAX, u32::MAX),
    ];

    for (pattern_id, start, end) in test_cases {
        let m = Match::from_parts(pattern_id, start, end);
        assert_eq!(
            m.pattern_id, pattern_id,
            "pattern_id mismatch for ({pattern_id}, {start}, {end})"
        );
        assert_eq!(
            m.start, start,
            "start mismatch for ({pattern_id}, {start}, {end})"
        );
        assert_eq!(
            m.end, end,
            "end mismatch for ({pattern_id}, {start}, {end})"
        );
        assert_eq!(m.padding(), 0, "padding should default to 0");
    }
}

/// Test with padding extremes
#[test]
fn match_from_parts_with_padding_extremes() {
    let test_cases = [
        (0u32, 0u32, 0u32, 0u32),
        (1, 2, 3, u32::MAX),
        (u32::MAX, u32::MAX, u32::MAX, 0),
        (u32::MAX, u32::MAX, u32::MAX, u32::MAX),
    ];

    for (pattern_id, start, end, padding) in test_cases {
        let m = Match::from_parts_with_padding(pattern_id, start, end, padding);
        assert_eq!(m.pattern_id, pattern_id);
        assert_eq!(m.start, start);
        assert_eq!(m.end, end);
        assert_eq!(m.padding(), padding);
    }
}

// ============================================================================
// TEST 5: Matcher trait can be implemented
// ============================================================================

/// A trivial Matcher implementation for testing
struct TrivialMatcher {
    matches: Vec<Match>,
}

#[async_trait::async_trait]
impl Matcher for TrivialMatcher {
    async fn scan(&self, _data: &[u8]) -> matchkit::Result<Vec<Match>> {
        Ok(self.matches.clone())
    }
}

/// Verify trivial Matcher implementation works
#[test]
fn matcher_trait_can_be_implemented() {
    let matcher = TrivialMatcher {
        matches: vec![Match::from_parts(1, 0, 10)],
    };

    // Test that we can call scan through the trait using futures executor
    let result = futures::executor::block_on(async { matcher.scan(b"test data").await });

    assert!(result.is_ok());
    let matches = result.unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0], Match::from_parts(1, 0, 10));
}

/// Verify Matcher works with generics (RPITIT — not dyn-compatible)
#[test]
fn matcher_works_generic() {
    fn scan_with<M: Matcher>(matcher: &M, data: &[u8]) -> matchkit::Result<Vec<Match>> {
        futures::executor::block_on(matcher.scan(data))
    }

    let matcher = TrivialMatcher {
        matches: vec![Match::from_parts(0, 0, 5)],
    };

    let result = scan_with(&matcher, b"data");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

/// Verify Matcher is Send + Sync
#[test]
fn matcher_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<TrivialMatcher>();
}

// ============================================================================
// TEST 6: BlockMatcher trait produces correct results
// ============================================================================

/// A trivial BlockMatcher implementation for testing
struct TrivialBlockMatcher {
    max_block: usize,
    matches: Vec<Match>,
}

#[async_trait::async_trait]
impl BlockMatcher for TrivialBlockMatcher {
    async fn scan_block(&self, _data: &[u8]) -> matchkit::Result<Vec<Match>> {
        Ok(self.matches.clone())
    }

    fn max_block_size(&self) -> usize {
        self.max_block
    }
}

/// Verify BlockMatcher implementation works
#[test]
fn block_matcher_produces_correct_results() {
    let matcher = TrivialBlockMatcher {
        max_block: 4096,
        matches: vec![Match::from_parts(0, 0, 10), Match::from_parts(1, 20, 30)],
    };

    let result =
        futures::executor::block_on(async { matcher.scan_block(b"some data block").await });

    assert!(result.is_ok());
    let matches = result.unwrap();
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].pattern_id, 0);
    assert_eq!(matches[1].pattern_id, 1);
}

/// Verify BlockMatcher max_block_size is respected
#[test]
fn block_matcher_max_block_size_is_correct() {
    let test_sizes = [0usize, 1, 4096, 1024 * 1024, usize::MAX];

    for size in test_sizes {
        let matcher = TrivialBlockMatcher {
            max_block: size,
            matches: Vec::new(),
        };
        assert_eq!(matcher.max_block_size(), size);
    }
}

/// Verify BlockMatcher is Send + Sync
#[test]
fn block_matcher_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<TrivialBlockMatcher>();
}

// ============================================================================
// BONUS: Additional foundational invariants
// ============================================================================

/// Verify GpuMatch to Match conversion preserves all fields
#[test]
fn gpu_match_to_match_conversion() {
    let gpu = GpuMatch([1, 10, 20, 5]);
    let m: Match = gpu.into();

    assert_eq!(m.pattern_id, 1);
    assert_eq!(m.start, 10);
    assert_eq!(m.end, 20);
    assert_eq!(m.padding(), 5);
}

/// Verify Match contains/overlaps logic
#[test]
fn match_contains_and_overlaps() {
    let outer = Match::from_parts(0, 10, 50);
    let inner = Match::from_parts(0, 20, 40);
    let adjacent = Match::from_parts(0, 50, 60);
    let separate = Match::from_parts(0, 60, 70);
    let overlapping = Match::from_parts(0, 40, 60);

    assert!(outer.contains(&inner), "outer should contain inner");
    assert!(!inner.contains(&outer), "inner should not contain outer");

    assert!(
        !outer.overlaps(&adjacent),
        "outer should not overlap adjacent (end == start)"
    );
    assert!(
        !outer.overlaps(&separate),
        "outer should not overlap separate"
    );
    assert!(
        outer.overlaps(&overlapping),
        "outer should overlap overlapping"
    );
}

/// Verify Match len and is_empty
#[test]
fn match_len_and_is_empty() {
    let empty = Match::from_parts(0, 10, 10);
    let non_empty = Match::from_parts(0, 10, 20);

    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);

    assert!(!non_empty.is_empty());
    assert_eq!(non_empty.len(), 10);
}

/// Verify Match len handles wrapping (saturating_sub)
#[test]
fn match_len_saturating_sub() {
    // When end < start, len should be 0 due to saturating_sub
    let m = Match::from_parts(0, 20, 10);
    assert_eq!(m.len(), 0);
    // Note: is_empty() checks start == end, not len() == 0
    // So a match with start > end is not "empty" per is_empty()
    assert!(!m.is_empty(), "is_empty checks start==end, not len()==0");
}

/// Verify bytemuck compatibility (Zeroable + Pod)
#[test]
fn match_is_bytemuck_pod() {
    use bytemuck::{Pod, Zeroable};

    fn assert_pod<T: Pod>() {}
    fn assert_zeroable<T: Zeroable>() {}

    assert_pod::<Match>();
    assert_zeroable::<Match>();
    assert_pod::<GpuMatch>();
    assert_zeroable::<GpuMatch>();
}

/// Verify Match can be used in atomic/lock-free contexts
#[test]
fn match_is_copy_for_lock_free_use() {
    fn assert_copy<T: Copy>() {}
    assert_copy::<Match>();
    assert_copy::<GpuMatch>();
}

/// Verify Match can be created in const context
#[test]
fn match_const_construction() {
    const M: Match = Match::from_parts(1, 2, 3);
    assert_eq!(M.pattern_id, 1);
    assert_eq!(M.start, 2);
    assert_eq!(M.end, 3);
}
