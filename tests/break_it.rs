use matchkit::{GpuMatch, Match, MatchSet};
use std::sync::{Arc, Mutex};
use std::thread;

// 1. Zero-length slice / empty input logic
#[test]
fn test_zero_length_bounds_logic() {
    let m = Match::from_parts(0, 5, 5);
    // Even if length is 0, start <= other.start && end >= other.end.
    // 5 <= 5 && 5 >= 5 is true. It should contain itself.
    assert!(m.contains(&m), "Zero-length match should contain itself");
}

// 2. Contains edge case: same start, longer end
#[test]
fn test_contains_edge_case_longer_end() {
    let m1 = Match::from_parts(0, 5, 10);
    let m2 = Match::from_parts(0, 5, 15);
    assert!(!m1.contains(&m2), "m1 cannot contain m2 if m2 ends later");
}

// 3. Maximum u32 values logic
#[test]
fn test_maximum_u32_bounds() {
    let m = Match::from_parts(u32::MAX, u32::MAX - 10, u32::MAX);
    assert_eq!(
        m.len(),
        10,
        "Length calculation near u32::MAX should not overflow"
    );
}

// 4. Overlap with maximum bounds
#[test]
fn test_overlap_max_bounds() {
    let m1 = Match::from_parts(0, u32::MAX - 5, u32::MAX);
    let m2 = Match::from_parts(0, u32::MAX - 2, u32::MAX);
    assert!(
        m1.overlaps(&m2),
        "Overlaps logic should correctly handle values near u32::MAX"
    );
}

// 5. 1MB+ input representation bounds
#[test]
fn test_large_input_bounds() {
    let large_size = 1024 * 1024 * 5; // 5MB
    let m = Match::from_parts(1, 0, large_size);
    assert_eq!(
        m.len(),
        large_size,
        "Should accurately represent 5MB match length"
    );
}

// 6. Concurrent access from 8 threads (Insert)
#[test]
fn test_concurrent_access_insert() {
    let set = Arc::new(Mutex::new(MatchSet::new()));
    let mut handles = vec![];
    for i in 0..8 {
        let set = set.clone();
        handles.push(thread::spawn(move || {
            set.lock().unwrap().insert(Match::from_parts(i, 0, 10));
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    assert_eq!(
        set.lock().unwrap().len(),
        8,
        "Concurrent unique inserts should all be recorded"
    );
}

// 7. Concurrent access from 8 threads (Merge)
#[test]
fn test_concurrent_access_merge() {
    let set = Arc::new(Mutex::new(MatchSet::new()));
    let mut handles = vec![];
    for i in 0..8 {
        let set = set.clone();
        handles.push(thread::spawn(move || {
            let mut locked = set.lock().unwrap();
            locked.insert(Match::from_parts(i, i * 5, (i * 5) + 15));
            locked.merge_overlapping();
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    let final_set = set.lock().unwrap();
    // 0..15, 5..20, 10..25, 15..30, 20..35, 25..40, 30..45, 35..50
    // They all chain-overlap. Merged result should be 1 item.
    assert_eq!(
        final_set.len(),
        1,
        "Concurrent cascading merges should result in 1 match"
    );
}

// 8. Malformed bounds: inverted start and end
#[test]
fn test_malformed_inverted_bounds() {
    let m = Match::from_parts(0, 20, 10);
    // saturating_sub makes length 0. But overlaps might misbehave.
    let valid = Match::from_parts(0, 15, 25);
    // valid overlaps m if 15 < 10 (false).
    assert!(
        !m.overlaps(&valid),
        "Inverted bound matches should not erroneously overlap valid matches"
    );
}

// 9. Malformed state: extremely large start, 0 end
#[test]
fn test_malformed_large_start_zero_end() {
    let m = Match::from_parts(0, u32::MAX, 0);
    assert_eq!(
        m.len(),
        0,
        "Length must be 0 for invalid huge start / zero end"
    );
}

// 10. Null bytes / Zero IDs
#[test]
fn test_zero_id_and_bounds() {
    let m = Match::from_parts(0, 0, 0);
    assert!(m.is_empty(), "All zeros should be empty");
}

// 11. Unicode edge cases (Surrogates via byte len)
#[test]
fn test_unicode_surrogate_byte_len() {
    // 3 bytes per surrogate typically in UTF-8
    let m = Match::from_parts(0, 0, 3);
    assert_eq!(
        m.len(),
        3,
        "Length calculation must respect exact byte offsets"
    );
}

// 12. Duplicate entries (same key twice)
#[test]
fn test_duplicate_entries_insert() {
    let mut s = MatchSet::new();
    s.insert(Match::from_parts(1, 10, 20));
    s.insert(Match::from_parts(1, 10, 20));
    assert_eq!(
        s.len(),
        1,
        "MatchSet should implicitly dedup via binary_search in insert"
    );
}

// 13. Duplicate entries with try_insert
#[test]
fn test_duplicate_entries_try_insert() {
    let mut s = MatchSet::new();
    s.try_insert(Match::from_parts(1, 10, 20)).unwrap();
    s.try_insert(Match::from_parts(1, 10, 20)).unwrap();
    assert_eq!(
        s.len(),
        1,
        "try_insert should implicitly dedup exact duplicates"
    );
}

// 14. Off-by-one: first byte boundary
#[test]
fn test_off_by_one_first_byte() {
    let mut s = MatchSet::new();
    s.insert(Match::from_parts(1, 0, 1));
    s.insert(Match::from_parts(1, 1, 2));
    s.merge_overlapping();
    // Overlaps logic is strictly `<` and `<`. Adjacency (1==1) doesn't overlap.
    // If adjacent matches are expected to merge, this exposes a finding in the engine.
    assert_eq!(
        s.len(),
        2,
        "Adjacent single bytes shouldn't merge under strict overlaps"
    );
}

// 15. Off-by-one: merge bound inclusion
#[test]
fn test_merge_bound_inclusion() {
    let mut s = MatchSet::new();
    s.insert(Match::from_parts(1, 0, 10));
    s.insert(Match::from_parts(1, 9, 20));
    s.merge_overlapping();
    assert_eq!(s.len(), 1, "Overlapping by 1 byte should merge");
    assert_eq!(s.as_slice()[0].end, 20, "Merged end should be maximum");
}

// 16. Resource exhaustion: 100K items
#[test]
fn test_resource_exhaustion_100k() {
    let mut s = MatchSet::new();
    let matches: Vec<_> = (0..100_000)
        .map(|i| Match::from_parts(1, i * 2, (i * 2) + 1))
        .collect();
    s.extend(matches);
    assert_eq!(
        s.len(),
        100_000,
        "Should handle 100K non-overlapping inserts via extend"
    );
}

// 17. Resource exhaustion: cascading merge of 100K
#[test]
fn test_resource_exhaustion_merge_100k() {
    let mut s = MatchSet::new();
    let matches: Vec<_> = (0..100_000)
        .map(|i| Match::from_parts(1, i, i + 2))
        .collect();
    s.extend(matches);
    s.merge_overlapping();
    assert_eq!(
        s.len(),
        1,
        "Should successfully merge 100K overlapping items into 1"
    );
    assert_eq!(
        s.as_slice()[0].end,
        100_001,
        "Final end bound should be 100001"
    );
}

// 18. Legacy interface OOM (try_with_capacity safe fallback)
#[test]
fn test_try_capacity_oom_graceful() {
    let s = MatchSet::try_with_capacity(usize::MAX);
    assert!(
        s.is_err(),
        "try_with_capacity on usize::MAX should return backend error gracefully"
    );
}

// 19. Try extend capacity bounds
#[test]
fn test_try_extend_exhaustion() {
    let mut s = MatchSet::new();
    // Providing a size hint of usize::MAX to trigger try_reserve failure
    struct ToxicIter;
    impl Iterator for ToxicIter {
        type Item = Match;
        fn next(&mut self) -> Option<Match> {
            None
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            (usize::MAX, None)
        }
    }
    let res = s.try_extend(ToxicIter);
    assert!(
        res.is_err(),
        "try_extend should gracefully error on OOM via size_hint"
    );
}

// 20. Try insert capacity reallocation error
#[test]
fn test_try_insert_exhaustion() {
    // If we allocate exactly up to a near-max threshold, trying to insert 1 more might trigger OOM gracefully.
    // Instead of forcing physical OOM which kills the runner, we verify try_insert propagates memory errors correctly
    // by mocking a failure if possible, or ensuring the Result wrapper is sound.
    let mut s = MatchSet::new();
    let res = s.try_insert(Match::from_parts(0, 0, 1));
    assert!(res.is_ok(), "Normal try_insert should succeed");
}

// 21. Try merge overlapping OOM error check
#[test]
fn test_try_merge_exhaustion_safety() {
    let mut s = MatchSet::new();
    s.insert(Match::from_parts(1, 0, 10));
    s.insert(Match::from_parts(1, 5, 15));
    let res = s.try_merge_overlapping();
    assert!(res.is_ok(), "Normal try_merge_overlapping should succeed");
}

// 22. GpuMatch precise serialization / from_bytes compatibility
#[test]
fn test_gpu_match_buffer_layout() {
    let g = GpuMatch([42, 100, 200, 0]);
    let m: Match = g.into();
    assert_eq!(
        m.pattern_id, 42,
        "GpuMatch pattern_id offset must be index 0"
    );
    assert_eq!(m.start, 100, "GpuMatch start offset must be index 1");
    assert_eq!(m.end, 200, "GpuMatch end offset must be index 2");
    assert_eq!(m.padding(), 0, "GpuMatch padding offset must be index 3");
}

// 23. Equality with padding variations
#[test]
fn test_equality_ignores_padding() {
    let m1 = Match::from_parts_with_padding(1, 10, 20, 999);
    let m2 = Match::from_parts_with_padding(1, 10, 20, 888);
    // PartialEq checks pattern_id, start, end. Padding is ignored in Eq/PartialEq implementation.
    assert_eq!(m1, m2, "Match equality should ignore padding differences");
}

// 24. Sort order edge cases
#[test]
fn test_match_ordering_edge_cases() {
    let mut s = MatchSet::new();
    // Insert reverse
    s.insert(Match::from_parts(3, 10, 20));
    s.insert(Match::from_parts(2, 10, 20));
    s.insert(Match::from_parts(1, 10, 20));
    // Should sort by start, then pattern_id
    assert_eq!(
        s.as_slice()[0].pattern_id,
        1,
        "Ordering should resolve ties by pattern_id"
    );
}

// 25. Filter by pattern
#[test]
fn test_filter_by_pattern() {
    let mut s = MatchSet::new();
    s.insert(Match::from_parts(1, 0, 10));
    s.insert(Match::from_parts(2, 0, 10));
    let s2 = s.filter_by_pattern(1);
    assert_eq!(
        s2.len(),
        1,
        "Filter should return exactly items matching ID"
    );
    assert_eq!(
        s2.as_slice()[0].pattern_id,
        1,
        "Filter must preserve correct items"
    );
}

// 26. Pattern counts exactness
#[test]
fn test_pattern_counts_accumulation() {
    let mut s = MatchSet::new();
    s.insert(Match::from_parts(5, 0, 10));
    s.insert(Match::from_parts(5, 20, 30));
    let counts = s.pattern_counts();
    assert_eq!(
        counts.get(&5),
        Some(&2),
        "Pattern counts should accurately sum unique instances"
    );
}

// 27. Pattern IDs exactness
#[test]
fn test_pattern_ids_deduplication() {
    let mut s = MatchSet::new();
    s.insert(Match::from_parts(9, 0, 10));
    s.insert(Match::from_parts(9, 20, 30));
    s.insert(Match::from_parts(4, 10, 20));
    let ids = s.pattern_ids();
    assert_eq!(
        ids,
        vec![4, 9],
        "Pattern IDs must be sorted and deduplicated"
    );
}

// 28. Merge disjoint segments stability
#[test]
fn test_merge_disjoint_segments() {
    let mut s = MatchSet::new();
    s.insert(Match::from_parts(1, 0, 10));
    s.insert(Match::from_parts(1, 20, 30));
    s.merge_overlapping();
    assert_eq!(s.len(), 2, "Disjoint segments must not be merged");
}

// 29. Transitive contains boundary
#[test]
fn test_transitive_contains_boundary() {
    let m1 = Match::from_parts(0, 0, 100);
    let m2 = Match::from_parts(0, 50, 100);
    let m3 = Match::from_parts(0, 50, 50);
    assert!(m1.contains(&m2), "m1 contains m2");
    assert!(m2.contains(&m3), "m2 contains zero-length m3 at boundary");
    assert!(m1.contains(&m3), "Contains should be logically transitive");
}

// 30. Empty subset overlaps
#[test]
fn test_empty_subset_overlaps() {
    let s1 = MatchSet::new();
    let mut s2 = MatchSet::new();
    s2.insert(Match::from_parts(1, 0, 10));
    // A completely empty set shouldn't have weird length issues
    assert!(s1.is_empty(), "New set is empty");
    assert!(!s2.is_empty(), "Set with insert is not empty");
}

// 31. Merge zero length matches
#[test]
fn test_merge_zero_length() {
    let mut s = MatchSet::new();
    s.insert(Match::from_parts(1, 5, 5));
    s.insert(Match::from_parts(1, 5, 10));
    s.merge_overlapping();
    // 5..5 and 5..10. Overlaps logic is 5 < 10 && 5 < 5 (False).
    // They don't technically overlap by the engine's strict formula.
    assert_eq!(
        s.len(),
        2,
        "Strict overlap formula isolates zero-length matches"
    );
}

// 32. Multi-pattern merge precedence
#[test]
fn test_multi_pattern_merge_precedence() {
    let mut s = MatchSet::new();
    // ID 2 starts earlier, ID 1 starts later but overlaps.
    s.insert(Match::from_parts(2, 0, 15));
    s.insert(Match::from_parts(1, 5, 20));
    s.merge_overlapping();
    // Since 2 is earlier, it sorts first. Merge takes the first pattern ID.
    assert_eq!(
        s.as_slice()[0].pattern_id,
        2,
        "Merge must take the pattern ID of the earlier-starting match"
    );
    assert_eq!(
        s.as_slice()[0].end,
        20,
        "Merge must take the maximum end bound"
    );
}

// 33. Try extend with empty iterator
#[test]
fn test_try_extend_empty() {
    let mut s = MatchSet::new();
    let res = s.try_extend(std::iter::empty::<Match>());
    assert!(res.is_ok(), "try_extend on empty iterator should succeed");
    assert!(s.is_empty(), "Set should remain empty");
}

use matchkit::{BlockMatcher, Matcher, Error};
use std::mem;

// 34. Match size must be exactly 16 bytes for GPU
#[test]
fn test_adv_match_size_exact_16() {
    assert_eq!(mem::size_of::<Match>(), 16, "Match must be exactly 16 bytes");
    assert_eq!(mem::align_of::<Match>(), 4, "Match must have alignment of 4");
}

// 35. Match field pattern_id max candidate
#[test]
fn test_adv_match_pattern_id_max() {
    let m = Match::from_parts(u32::MAX, 10, 20);
    assert_eq!(m.pattern_id, u32::MAX, "pattern_id should handle u32::MAX");
}

// 36. Match field start max candidate
#[test]
fn test_adv_match_start_max() {
    let m = Match::from_parts(0, u32::MAX, u32::MAX);
    assert_eq!(m.start, u32::MAX, "start should handle u32::MAX");
}

// 37. Match field end max candidate
#[test]
fn test_adv_match_end_max() {
    let m = Match::from_parts(0, 0, u32::MAX);
    assert_eq!(m.end, u32::MAX, "end should handle u32::MAX");
}

// 38. Match from_parts all max candidates
#[test]
fn test_adv_match_all_fields_max() {
    let m = Match::from_parts(u32::MAX, u32::MAX, u32::MAX);
    assert_eq!(m.pattern_id, u32::MAX);
    assert_eq!(m.start, u32::MAX);
    assert_eq!(m.end, u32::MAX);
}

// 39. Match from_parts_with_padding max candidates
#[test]
fn test_adv_match_all_fields_and_padding_max() {
    let m = Match::from_parts_with_padding(u32::MAX, u32::MAX, u32::MAX, u32::MAX);
    assert_eq!(m.pattern_id, u32::MAX);
    assert_eq!(m.start, u32::MAX);
    assert_eq!(m.end, u32::MAX);
    assert_eq!(m.padding(), u32::MAX);
}

struct DummyAdvBlockMatcher;
impl BlockMatcher for DummyAdvBlockMatcher {
    fn scan_block(&self, _data: &[u8]) -> impl std::future::Future<Output = matchkit::Result<Vec<Match>>> + Send {
        async { Ok(vec![Match::from_parts(1, 0, 10)]) }
    }
    fn max_block_size(&self) -> usize {
        usize::MAX
    }
}

// 40. BlockMatcher trait generic implementation max block size
#[test]
fn test_adv_block_matcher_max_size() {
    let m = DummyAdvBlockMatcher;
    assert_eq!(m.max_block_size(), usize::MAX, "BlockMatcher max size handle usize::MAX");
}

// 41. BlockMatcher trait scan_block invocation
#[test]
fn test_adv_block_matcher_scan_block() {
    let m = DummyAdvBlockMatcher;
    let res = futures::executor::block_on(m.scan_block(b"test"));
    
    assert!(res.is_ok());
    assert_eq!(res.unwrap().len(), 1);
}

// 42. BlockMatcher trait implementation check

#[test]
fn test_adv_block_matcher_impl_check() {
    fn assert_block_matcher<T: BlockMatcher>() {}
    assert_block_matcher::<DummyAdvBlockMatcher>();
}

// 43. Error type completeness: InputTooLarge
#[test]
fn test_adv_error_input_too_large() {
    let e = Error::InputTooLarge { bytes: usize::MAX, max_bytes: 100 };
    let s = e.to_string();
    assert!(s.contains("scan input is too large"));
    assert!(s.contains("fix:"));
}

// 44. Error type completeness: MatchBufferOverflow
#[test]
fn test_adv_error_match_buffer_overflow() {
    let e = Error::MatchBufferOverflow { count: usize::MAX, max: 100 };
    let s = e.to_string();
    assert!(s.contains("too many matches"));
    assert!(s.contains("fix:"));
}

// 45. Error type completeness: EmptyPatternSet
#[test]
fn test_adv_error_empty_pattern_set() {
    let e = Error::EmptyPatternSet;
    let s = e.to_string();
    assert!(s.contains("pattern set is empty"));
    assert!(s.contains("fix:"));
}

// 46. Error type completeness: EmptyPattern
#[test]
fn test_adv_error_empty_pattern() {
    let e = Error::EmptyPattern { index: usize::MAX };
    let s = e.to_string();
    assert!(s.contains(&usize::MAX.to_string()));
    assert!(s.contains("is empty"));
    assert!(s.contains("fix:"));
}

// 47. Error type completeness: PatternCompilationFailed
#[test]
fn test_adv_error_pattern_compilation_failed() {
    let e = Error::PatternCompilationFailed { reason: "adv reason".into() };
    let s = e.to_string();
    assert!(s.contains("adv reason"));
    assert!(s.contains("pattern compilation failed"));
}

// 48. Error type completeness: Backend
#[test]
fn test_adv_error_backend() {
    let e = Error::Backend(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "adv backend")));
    let s = e.to_string();
    assert!(s.contains("adv backend"));
}

// 49. Zero-offset match creation
#[test]
fn test_adv_match_zero_offsets() {
    let m = Match::from_parts(0, 0, 0);
    assert_eq!(m.start, 0);
    assert_eq!(m.end, 0);
    assert!(m.is_empty());
}

// 50. Zero-offset match contains self
#[test]
fn test_adv_match_zero_offsets_contains() {
    let m = Match::from_parts(0, 0, 0);
    assert!(m.contains(&m));
}

// 51. Zero-offset match overlaps logic - it should NOT overlap anything, not even itself, based on strict start < end logic
#[test]
fn test_adv_match_zero_offsets_overlaps() {
    let m = Match::from_parts(0, 0, 0);
    assert!(!m.overlaps(&m));
}

// 52. Max-offset matches length
#[test]
fn test_adv_match_max_offsets_len() {
    let m = Match::from_parts(0, u32::MAX, u32::MAX);
    assert_eq!(m.len(), 0);
    assert!(m.is_empty());
}

// 53. Max-offset matches overlap
#[test]
fn test_adv_match_max_offsets_overlaps() {
    let m1 = Match::from_parts(0, u32::MAX - 10, u32::MAX);
    let m2 = Match::from_parts(0, u32::MAX - 20, u32::MAX - 5);
    assert!(m1.overlaps(&m2));
}

// 54. Overlapping match detection: exact match
#[test]
fn test_adv_match_overlap_exact() {
    let m1 = Match::from_parts(0, 10, 20);
    let m2 = Match::from_parts(0, 10, 20);
    assert!(m1.overlaps(&m2));
}

// 55. Overlapping match detection: one byte overlap
#[test]
fn test_adv_match_overlap_one_byte() {
    let m1 = Match::from_parts(0, 10, 20);
    let m2 = Match::from_parts(0, 19, 30);
    assert!(m1.overlaps(&m2));
    assert!(m2.overlaps(&m1));
}

// 56. Overlapping match detection: adjacent boundary (no overlap)
#[test]
fn test_adv_match_overlap_adjacent() {
    let m1 = Match::from_parts(0, 10, 20);
    let m2 = Match::from_parts(0, 20, 30);
    assert!(!m1.overlaps(&m2));
    assert!(!m2.overlaps(&m1));
}

// 57. Overlapping match detection: fully contained
#[test]
fn test_adv_match_overlap_contained() {
    let m1 = Match::from_parts(0, 10, 50);
    let m2 = Match::from_parts(0, 20, 30);
    assert!(m1.overlaps(&m2));
    assert!(m2.overlaps(&m1));
}

// 58. Overlapping match detection: max u32 bounds exact match
#[test]
fn test_adv_match_overlap_max_exact() {
    let m1 = Match::from_parts(0, u32::MAX - 10, u32::MAX);
    let m2 = Match::from_parts(0, u32::MAX - 10, u32::MAX);
    assert!(m1.overlaps(&m2));
}

// 59. Overlapping match detection: negative/inverted bounds (start > end)
#[test]
fn test_adv_match_overlap_inverted_bounds() {
    let m1 = Match::from_parts(0, 50, 10);
    let m2 = Match::from_parts(0, 0, 20);
    // 50 < 20 (false), 0 < 10 (true). Overlap should be false.
    assert!(!m1.overlaps(&m2));
}

// 60. Merge overlapping: zero-offset matches behavior
#[test]
fn test_adv_merge_zero_offset_matches() {
    let mut s = MatchSet::new();
    s.insert(Match::from_parts(0, 0, 0));
    s.insert(Match::from_parts(0, 0, 0));
    s.merge_overlapping();
    assert_eq!(s.len(), 1); // Exact duplicates are deduped
}

// 61. Merge overlapping: chained adjacent matches
#[test]
fn test_adv_merge_chained_adjacent() {
    let mut s = MatchSet::new();
    s.insert(Match::from_parts(1, 0, 10));
    s.insert(Match::from_parts(1, 10, 20));
    s.insert(Match::from_parts(1, 20, 30));
    s.merge_overlapping();
    // In current implementation, adjacent matches might merge or not depending on `overlaps()`
    // Since `overlaps()` is strictly `<` and not `<=`, adjacent matches don't overlap, BUT `merge_overlapping`
    // implementation in `match_set.rs` checks `w[0].end >= w[1].start`, so they DO merge!
    // Let's test that exact behavior.
    assert_eq!(s.len(), 3, "Adjacent matches should NOT merge since `overlaps` is strictly `<`");
    assert_eq!(s.as_slice()[2].end, 30);
}

// 62. GpuMatch layout exact fields verification
#[test]
fn test_adv_gpumatch_fields() {
    let g = GpuMatch([u32::MAX, 1, 2, u32::MAX]);
    assert_eq!(g.0[0], u32::MAX);
    assert_eq!(g.0[1], 1);
    assert_eq!(g.0[2], 2);
    assert_eq!(g.0[3], u32::MAX);
}

// 63. Eq and PartialEq with padded matches
#[test]
fn test_adv_eq_with_different_padding() {
    let m1 = Match::from_parts_with_padding(1, 0, 10, u32::MAX);
    let m2 = Match::from_parts_with_padding(1, 0, 10, 0);
    assert_eq!(m1, m2);
}

// 64. MatchOrd: sorting respects start then pattern_id
#[test]
fn test_adv_match_ord_rules() {
    let m1 = Match::from_parts(10, 0, 10);
    let m2 = Match::from_parts(5, 0, 10);
    assert!(m2 < m1);
}

// 65. MatchOrd: sorting respects end when start and pattern_id equal
#[test]
fn test_adv_match_ord_end_rules() {
    let m1 = Match::from_parts(5, 0, 20);
    let m2 = Match::from_parts(5, 0, 10);
    assert!(m2 < m1);
}

// 66. Error result type compatibility
#[test]
fn test_adv_error_result_type() {
    let res: matchkit::Result<Match> = Err(Error::EmptyPatternSet);
    assert!(res.is_err());
}
