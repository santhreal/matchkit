//! unit tests for matchkit.
//! See TESTING.md for the Santh testing standard.

use matchkit::{BlockMatcher, Error, GpuMatch, Match, MatchBatch, MatchSet, Matcher};

#[test]
fn match_new() {
    let m = Match::new(5, 10, 20);
    assert_eq!(m.pattern_id, 5);
    assert_eq!(m.start, 10);
    assert_eq!(m.end, 20);
}

#[test]
fn match_inequality() {
    let a = Match::new(0, 0, 5);
    let b = Match::new(1, 0, 5);
    assert_ne!(a, b);
}

#[test]
fn gpu_match_conversion() {
    let gpu = GpuMatch::new(3, 100, 200);
    let m: Match = gpu.into();
    assert_eq!(m.pattern_id, 3);
    assert_eq!(m.start, 100);
    assert_eq!(m.end, 200);
}

#[test]
fn gpu_match_is_pod() {
    let gpu = GpuMatch::new(1, 2, 3);
    let bytes = bytemuck::bytes_of(&gpu);
    assert_eq!(bytes.len(), 12);
}

#[test]
fn match_struct_is_12_bytes() {
    assert_eq!(std::mem::size_of::<Match>(), 12);
    assert_eq!(std::mem::align_of::<Match>(), 4);
}

#[test]
fn match_zero_length() {
    let m = Match::new(0, 5, 5);
    assert_eq!(m.start, m.end);
}

#[test]
fn match_max_values() {
    let m = Match::new(u32::MAX, u32::MAX, u32::MAX);
    assert_eq!(m.pattern_id, u32::MAX);
    assert_eq!(m.start, u32::MAX);
    assert_eq!(m.end, u32::MAX);
}

#[test]
fn match_inequality_by_start() {
    let a = Match::new(0, 0, 5);
    let b = Match::new(0, 1, 5);
    assert_ne!(a, b);
}

#[test]
fn match_inequality_by_end() {
    let a = Match::new(0, 0, 5);
    let b = Match::new(0, 0, 6);
    assert_ne!(a, b);
}

#[test]
fn gpu_match_round_trip_bytes() {
    let original = GpuMatch::new(7, 100, 200);
    let bytes = bytemuck::bytes_of(&original);
    let restored: &GpuMatch = bytemuck::from_bytes(bytes);
    assert_eq!(restored.0, original.0);
}

#[test]
fn match_clone_is_equal() {
    let m = Match::new(3, 10, 20);
    let cloned = m;
    assert_eq!(m.pattern_id, cloned.pattern_id);
    assert_eq!(m.start, cloned.start);
    assert_eq!(m.end, cloned.end);
}

#[test]
fn match_debug_format() {
    let m = Match::new(1, 2, 3);
    let debug = format!("{m:?}");
    assert!(debug.contains("pattern_id: 1"));
    assert!(debug.contains("start: 2"));
    assert!(debug.contains("end: 3"));
}

#[test]
fn error_display_actionable() {
    let e = Error::InputTooLarge {
        bytes: 100,
        max_bytes: 50,
    };
    let msg = e.to_string();
    assert!(msg.contains("fix:"), "error message must be actionable");
    assert!(msg.contains("100"));
    assert!(msg.contains("50"));
}

#[test]
fn error_empty_pattern_set() {
    let e = Error::EmptyPatternSet;
    assert!(e.to_string().contains("fix:"));
}

#[test]
fn error_match_buffer_overflow() {
    let e = Error::MatchBufferOverflow {
        count: 1000,
        max: 500,
    };
    let msg = e.to_string();
    assert!(msg.contains("1000"));
    assert!(msg.contains("500"));
}

#[test]
fn match_contains_fully_enclosed() {
    let outer = Match::new(0, 0, 10);
    let inner = Match::new(0, 2, 8);
    assert!(outer.contains(&inner));
    assert!(!inner.contains(&outer));
}

#[test]
fn match_contains_same_range() {
    let a = Match::new(0, 5, 10);
    let b = Match::new(0, 5, 10);
    assert!(a.contains(&b));
}

#[test]
fn match_overlaps_partial() {
    let a = Match::new(0, 0, 5);
    let b = Match::new(0, 3, 8);
    assert!(a.overlaps(&b));
    assert!(b.overlaps(&a));
}

#[test]
fn match_overlaps_adjacent_no() {
    let a = Match::new(0, 0, 5);
    let b = Match::new(0, 5, 10);
    assert!(!a.overlaps(&b));
}

#[test]
fn match_len_and_is_empty() {
    let m = Match::new(0, 3, 7);
    assert_eq!(m.len(), 4);
    assert!(!m.is_empty());

    let empty = Match::new(0, 5, 5);
    assert_eq!(empty.len(), 0);
    assert!(empty.is_empty());
}

#[test]
fn match_set_insert_dedup() {
    let mut set = MatchSet::new();
    set.insert(Match::new(0, 1, 5));
    set.insert(Match::new(0, 1, 5)); // duplicate
    set.insert(Match::new(0, 0, 3));
    assert_eq!(set.len(), 2);
    assert_eq!(set.as_slice()[0].start, 0); // sorted by start
}

#[test]
fn match_set_extend_sorts_and_dedup() {
    let mut set = MatchSet::new();
    set.extend([
        Match::new(0, 10, 20),
        Match::new(0, 0, 5),
        Match::new(0, 10, 20),
    ]);
    assert_eq!(set.len(), 2);
    assert_eq!(set.as_slice()[0].start, 0);
    assert_eq!(set.as_slice()[1].start, 10);
}

#[test]
fn match_set_merge_overlapping() {
    let mut set = MatchSet::new();
    set.extend([
        Match::new(0, 0, 5),
        Match::new(1, 3, 8),
        Match::new(2, 10, 15),
    ]);
    set.merge_overlapping();
    assert_eq!(set.len(), 2);
    assert_eq!(set.as_slice()[0].start, 0);
    assert_eq!(set.as_slice()[0].end, 8);
    assert_eq!(set.as_slice()[1].start, 10);
}

#[test]
fn match_set_empty() {
    let set = MatchSet::new();
    assert!(set.is_empty());
    assert_eq!(set.len(), 0);
}

#[test]
fn match_set_into_vec() {
    let mut set = MatchSet::new();
    set.insert(Match::new(0, 0, 5));
    let v = set.into_vec();
    assert_eq!(v.len(), 1);
}

#[test]
fn match_set_extend_is_sorted_and_deduped() {
    let mut set = MatchSet::new();
    set.extend([
        Match::new(2, 50, 60),
        Match::new(0, 10, 20),
        Match::new(1, 30, 40),
        Match::new(0, 10, 20), // duplicate
    ]);
    let slice = set.as_slice();
    assert_eq!(slice.len(), 3);
    // Verify sorted
    for pair in slice.windows(2) {
        assert!(pair[0] <= pair[1]);
    }
}

#[test]
fn merge_overlapping_produces_no_overlaps() {
    let mut set = MatchSet::new();
    set.extend([
        Match::new(0, 0, 10),
        Match::new(0, 5, 15),
        Match::new(0, 20, 30),
        Match::new(0, 25, 35),
    ]);
    set.merge_overlapping();
    let slice = set.as_slice();
    for pair in slice.windows(2) {
        assert!(!pair[0].overlaps(&pair[1]), "overlap found after merge");
    }
}

#[test]
fn filter_by_pattern_isolates_correct_id() {
    let mut set = MatchSet::new();
    set.extend([
        Match::new(0, 0, 5),
        Match::new(1, 10, 15),
        Match::new(0, 20, 25),
        Match::new(2, 30, 35),
        Match::new(1, 40, 45),
    ]);
    let filtered = set.filter_by_pattern(1);
    assert_eq!(filtered.len(), 2);
    for m in filtered.as_slice() {
        assert_eq!(m.pattern_id, 1);
    }
}

#[test]
fn pattern_counts_correct() {
    let mut set = MatchSet::new();
    set.extend([
        Match::new(0, 0, 5),
        Match::new(0, 10, 15),
        Match::new(1, 20, 25),
    ]);
    let counts = set.pattern_counts();
    assert_eq!(counts[&0], 2);
    assert_eq!(counts[&1], 1);
}

#[test]
fn pattern_ids_returns_sorted_unique() {
    let mut set = MatchSet::new();
    set.extend([
        Match::new(5, 0, 1),
        Match::new(1, 2, 3),
        Match::new(5, 4, 5),
        Match::new(3, 6, 7),
    ]);
    assert_eq!(set.pattern_ids(), vec![1, 3, 5]);
}

#[test]
fn match_set_with_max_u32_offsets() {
    let m = Match::new(u32::MAX, u32::MAX - 1, u32::MAX);
    let mut set = MatchSet::new();
    set.insert(m);
    assert_eq!(set.len(), 1);
    assert_eq!(set.as_slice()[0].start, u32::MAX - 1);
}

#[test]
fn match_set_100k_entries() {
    let mut set = MatchSet::new();
    set.extend((0..100_000u32).map(|i| Match::new(i % 100, i, i + 1)));
    assert_eq!(set.len(), 100_000);
}

#[test]
fn empty_match_set_operations() {
    let set = MatchSet::new();
    assert!(set.is_empty());
    assert_eq!(set.len(), 0);
    assert!(set.as_slice().is_empty());
    assert!(set.into_vec().is_empty());
}

#[test]
fn match_len_saturating() {
    let m = Match::new(0, 100, 50); // start > end (invalid but shouldn't panic)
    assert_eq!(m.len(), 0); // saturating_sub
}

#[test]
fn matcher_trait_works_with_generics() {
    struct DummyMatcher;

    #[async_trait::async_trait]
    impl Matcher for DummyMatcher {
        async fn scan(&self, _data: &[u8]) -> matchkit::Result<Vec<Match>> {
            Ok(vec![])
        }
    }

    fn scan_generic<M: Matcher>(m: &M, data: &[u8]) -> matchkit::Result<Vec<Match>> {
        futures::executor::block_on(m.scan(data))
    }
    let result = scan_generic(&DummyMatcher, b"test").unwrap();
    assert_eq!(result.len(), 0);
}

#[test]
fn block_matcher_trait_works_with_generics() {
    struct DummyBlockMatcher;

    #[async_trait::async_trait]
    impl BlockMatcher for DummyBlockMatcher {
        async fn scan_block(&self, _data: &[u8]) -> matchkit::Result<Vec<Match>> {
            Ok(vec![])
        }
        fn max_block_size(&self) -> usize {
            1024
        }
    }

    fn scan_block_generic<M: BlockMatcher>(m: &M, data: &[u8]) -> matchkit::Result<Vec<Match>> {
        futures::executor::block_on(m.scan_block(data))
    }
    let m = DummyBlockMatcher;
    assert_eq!(m.max_block_size(), 1024);
    let result = scan_block_generic(&m, b"test").unwrap();
    assert_eq!(result.len(), 0);
}

#[test]
fn match_batch_so_a_operations() {
    let mut batch = MatchBatch::new();
    batch.push(Match::new(1, 10, 20));
    batch.push(Match::new(2, 30, 40));

    assert_eq!(batch.len(), 2);
    assert_eq!(batch.pattern_ids, vec![1, 2]);
    assert_eq!(batch.starts, vec![10, 30]);
    assert_eq!(batch.ends, vec![20, 40]);

    let matches = batch.into_vec();
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0], Match::new(1, 10, 20));
    assert_eq!(matches[1], Match::new(2, 30, 40));
}
