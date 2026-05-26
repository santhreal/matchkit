//! S-proptest-03 - matchkit mass proptest (p11-p34).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use matchkit::{GpuMatch, Match, MatchSet};
use proptest::prelude::*;

fn mk(pattern_id: u32, start: u32, end: u32) -> Match {
    Match::new(pattern_id, start, end)
}

fn normalized(pattern_id: u32, a: u32, b: u32) -> Match {
    let start = a.min(b);
    let end = a.max(b);
    mk(pattern_id, start, end)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn p11_match_new_fields(
        pattern_id in any::<u32>(),
        start in any::<u32>(),
        end in any::<u32>(),
    ) {
        let m = Match::new(pattern_id, start, end);
        prop_assert_eq!(m.pattern_id, pattern_id);
        prop_assert_eq!(m.start, start);
        prop_assert_eq!(m.end, end);
    }

    #[test]
    fn p12_match_len_saturating(start in any::<u32>(), end in any::<u32>()) {
        let m = Match::new(0, start, end);
        prop_assert_eq!(m.len(), end.saturating_sub(start));
    }

    #[test]
    fn p13_contains_reflexive(start in any::<u32>(), end in any::<u32>()) {
        let m = Match::new(1, start.min(end), start.max(end));
        prop_assert!(m.contains(&m));
    }

    #[test]
    fn p14_overlaps_reflexive(start in any::<u32>(), end in any::<u32>()) {
        let m = Match::new(1, start.min(end), start.max(end));
        prop_assert!(m.overlaps(&m));
    }

    #[test]
    fn p15_with_capacity_never_panics(cap in 0usize..10_000) {
        let set = MatchSet::with_capacity(cap);
        prop_assert!(set.is_empty());
    }

    #[test]
    fn p16_merge_reduces_or_equal(
        raw in prop::collection::vec((any::<u32>(), any::<u32>(), any::<u32>()), 0..64),
    ) {
        let mut set = MatchSet::new();
        set.extend(raw.into_iter().map(|(p, s, e)| normalized(p, s, e)));
        let before = set.len();
        set.merge_overlapping();
        prop_assert!(set.len() <= before);
    }

    #[test]
    fn p17_filter_missing_empty(pattern_id in any::<u32>()) {
        let filtered = MatchSet::new().filter_by_pattern(pattern_id);
        prop_assert!(filtered.is_empty());
    }

    #[test]
    fn p18_into_vec_len(
        raw in prop::collection::vec((any::<u32>(), any::<u32>(), any::<u32>()), 0..32),
    ) {
        let n = raw.len();
        let mut set = MatchSet::new();
        set.extend(raw.into_iter().map(|(p, s, e)| mk(p, s, e)));
        let vec = set.into_vec();
        prop_assert!(vec.len() <= n);
    }

    #[test]
    fn p19_u32_max_offsets(start in any::<u32>()) {
        let m = Match::new(0, start, u32::MAX);
        prop_assert_eq!(m.end, u32::MAX);
    }

    #[test]
    fn p20_disjoint_same_pattern_no_overlap(a in 0u32..1000, b in 1000u32..2000) {
        let left = Match::new(1, a, a + 10);
        let right = Match::new(1, b, b + 10);
        prop_assert!(!left.overlaps(&right));
    }

    #[test]
    fn p21_adjacent_no_overlap(a in 0u32..1000) {
        let left = Match::new(1, a, a + 5);
        let right = Match::new(1, a + 5, a + 10);
        prop_assert!(!left.overlaps(&right));
    }

    #[test]
    fn p22_nested_contains(outer_start in 0u32..100, inner_start in 10u32..20) {
        let outer = Match::new(1, outer_start, outer_start + 50);
        let inner = Match::new(1, inner_start, inner_start + 5);
        if inner.start >= outer.start && inner.end <= outer.end {
            prop_assert!(outer.contains(&inner));
        }
    }

    #[test]
    fn p23_dedup_insert_twice(p in any::<u32>(), s in any::<u32>(), e in any::<u32>()) {
        let mut set = MatchSet::new();
        let m = mk(p, s, e);
        set.insert(m);
        set.insert(m);
        prop_assert_eq!(set.len(), 1);
    }

    #[test]
    fn p24_extend_empty_noop(_unused in 0..1i32) {
        let mut set = MatchSet::new();
        set.extend(std::iter::empty::<Match>());
        prop_assert!(set.is_empty());
    }

    #[test]
    fn p25_pattern_counts_keys_unique(
        raw in prop::collection::vec((0u32..8, any::<u32>(), any::<u32>()), 0..64),
    ) {
        let mut set = MatchSet::new();
        set.extend(raw.into_iter().map(|(p, s, e)| mk(p, s, e)));
        let counts = set.pattern_counts();
        prop_assert!(counts.len() <= 8);
    }

    #[test]
    fn p26_gpu_match_size_roundtrip(p in any::<u32>(), s in any::<u32>(), e in any::<u32>()) {
        let m = mk(p, s, e);
        let gpu: GpuMatch = m.into();
        let back: Match = gpu.into();
        prop_assert_eq!(std::mem::size_of_val(&gpu), std::mem::size_of::<GpuMatch>());
        prop_assert_eq!(back.pattern_id, p);
    }

    #[test]
    fn p27_merge_empty_set_ok(_unused in 0..1i32) {
        let mut set = MatchSet::new();
        set.merge_overlapping();
        prop_assert!(set.is_empty());
    }

    #[test]
    fn p28_filter_preserves_len_bound(
        raw in prop::collection::vec((0u32..4, any::<u32>(), any::<u32>()), 0..32),
    ) {
        let mut set = MatchSet::new();
        set.extend(raw.into_iter().map(|(p, s, e)| mk(p, s, e)));
        for id in 0..4 {
            let filtered = set.filter_by_pattern(id);
            prop_assert!(filtered.len() <= set.len());
        }
    }

    #[test]
    fn p29_reverse_input_sorted(n in 1usize..50) {
        let mut set = MatchSet::new();
        set.extend((0..n as u32).rev().map(|i| mk(i % 3, i, i + 1)));
        for pair in set.as_slice().windows(2) {
            prop_assert!(pair[0] <= pair[1]);
        }
    }

    #[test]
    fn p30_all_same_pattern_merge(raw in prop::collection::vec(any::<u32>(), 1..32)) {
        let mut set = MatchSet::new();
        set.extend(raw.iter().map(|&s| normalized(7, s, s + 1)));
        set.merge_overlapping();
        prop_assert!(set.iter().all(|m| m.pattern_id == 7));
    }

    #[test]
    fn p31_duplicate_insert_len_one(p in any::<u32>(), s in any::<u32>(), e in any::<u32>()) {
        let mut set = MatchSet::new();
        let m = mk(p, s, e);
        set.insert(m);
        set.insert(m);
        prop_assert_eq!(set.len(), 1);
    }

    #[test]
    fn p32_large_batch_extend(
        raw in prop::collection::vec((any::<u32>(), any::<u32>(), any::<u32>()), 0..256),
    ) {
        let mut set = MatchSet::new();
        set.extend(raw.into_iter().map(|(p, s, e)| mk(p, s, e)));
        prop_assert!(set.len() <= 256);
    }

    #[test]
    fn p33_overlap_implies_contains_boundary(a in 0u32..100, b in 10u32..30) {
        let outer = Match::new(0, a, a + 40);
        let inner = Match::new(1, a + 5, b);
        if inner.end <= outer.end && inner.start >= outer.start {
            prop_assert!(outer.contains(&inner));
        }
    }

    #[test]
    fn p34_pattern_ids_subset_of_inserts(
        raw in prop::collection::vec((0u32..16, any::<u32>(), any::<u32>()), 1..48),
    ) {
        let mut set = MatchSet::new();
        set.extend(raw.into_iter().map(|(p, s, e)| mk(p, s, e)));
        let ids: std::collections::BTreeSet<_> = set.pattern_ids().into_iter().collect();
        for id in ids {
            prop_assert!(id < 16);
        }
    }
}
