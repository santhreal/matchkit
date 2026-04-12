#![allow(clippy::panic, missing_docs)]

use matchkit::{Match, MatchSet};
use proptest::prelude::*;

fn mk(pattern_id: u32, start: u32, end: u32) -> Match {
    Match::new(pattern_id, start, end)
}

fn normalized_match(pattern_id: u32, a: u32, b: u32) -> Match {
    let start = a.min(b);
    let end = a.max(b);
    mk(pattern_id, start, end)
}

fn ten_pattern_matches() -> Vec<Match> {
    (0..10)
        .flat_map(|pattern_id| {
            (0..3).map(move |offset| {
                mk(
                    pattern_id,
                    pattern_id * 10 + offset,
                    pattern_id * 10 + offset + 2,
                )
            })
        })
        .collect()
}

proptest! {
    #[test]
    fn extend_always_produces_sorted_deduped_slice(
        raw in prop::collection::vec((any::<u32>(), any::<u32>(), any::<u32>()), 0..256)
    ) {
        let mut set = MatchSet::new();
        let expected: Vec<_> = {
            let mut items: Vec<_> = raw
                .iter()
                .map(|&(pattern_id, start, end)| mk(pattern_id, start, end))
                .collect();
            items.sort_unstable();
            items.dedup();
            items
        };

        set.extend(raw.into_iter().map(|(pattern_id, start, end)| mk(pattern_id, start, end)));

        prop_assert_eq!(set.as_slice(), expected.as_slice());
    }

    #[test]
    fn merge_overlapping_never_leaves_overlaps(
        raw in prop::collection::vec((any::<u32>(), any::<u32>(), any::<u32>()), 0..128)
    ) {
        let mut set = MatchSet::new();
        set.extend(
            raw.into_iter()
                .map(|(pattern_id, start, end)| normalized_match(pattern_id, start, end))
        );

        set.merge_overlapping();

        for pair in set.as_slice().windows(2) {
            prop_assert!(!pair[0].overlaps(&pair[1]));
        }
    }
}

#[test]
fn filter_by_pattern_supports_ten_distinct_pattern_ids() {
    let mut set = MatchSet::new();
    set.extend(ten_pattern_matches());

    for pattern_id in 0..10 {
        let filtered = set.filter_by_pattern(pattern_id);
        assert_eq!(filtered.len(), 3);
        assert!(filtered
            .as_slice()
            .iter()
            .all(|m| m.pattern_id == pattern_id));
    }
}

#[test]
fn filter_by_pattern_missing_id_is_empty() {
    let mut set = MatchSet::new();
    set.extend(ten_pattern_matches());

    assert!(set.filter_by_pattern(99).is_empty());
}

#[test]
fn pattern_counts_are_correct_for_ten_pattern_ids() {
    let mut set = MatchSet::new();
    set.extend(ten_pattern_matches());

    let counts = set.pattern_counts();
    assert_eq!(counts.len(), 10);
    for pattern_id in 0..10 {
        assert_eq!(counts.get(&pattern_id), Some(&3));
    }
}

#[test]
fn pattern_counts_empty_set_is_empty() {
    assert!(MatchSet::new().pattern_counts().is_empty());
}

#[test]
fn pattern_ids_are_sorted_and_deduped() {
    let mut set = MatchSet::new();
    set.extend([
        mk(9, 9, 10),
        mk(3, 3, 5),
        mk(9, 11, 13),
        mk(1, 1, 2),
        mk(3, 8, 9),
    ]);

    assert_eq!(set.pattern_ids(), vec![1, 3, 9]);
}

#[test]
fn merge_overlapping_preserves_first_pattern_id_in_group() {
    let mut set = MatchSet::new();
    set.extend([mk(5, 10, 20), mk(8, 15, 25), mk(9, 30, 40)]);

    set.merge_overlapping();

    assert_eq!(set.as_slice(), &[mk(5, 10, 25), mk(9, 30, 40)]);
}

#[test]
fn merge_overlapping_keeps_adjacent_ranges_separate() {
    let mut set = MatchSet::new();
    set.extend([mk(1, 0, 5), mk(2, 5, 10), mk(3, 10, 12)]);

    set.merge_overlapping();

    assert_eq!(set.len(), 3);
}

#[test]
fn merge_overlapping_handles_nested_ranges() {
    let mut set = MatchSet::new();
    set.extend([mk(1, 0, 20), mk(2, 5, 10), mk(3, 8, 18)]);

    set.merge_overlapping();

    assert_eq!(set.as_slice(), &[mk(1, 0, 20)]);
}

#[test]
fn empty_set_operations_are_noops() {
    let mut set = MatchSet::new();
    set.merge_overlapping();

    assert!(set.is_empty());
    assert!(set.filter_by_pattern(0).is_empty());
    assert!(set.pattern_counts().is_empty());
    assert!(set.pattern_ids().is_empty());
}

#[test]
fn u32_max_offsets_are_preserved() {
    let mut set = MatchSet::new();
    set.insert(mk(7, u32::MAX - 1, u32::MAX));

    assert_eq!(set.as_slice(), &[mk(7, u32::MAX - 1, u32::MAX)]);
}

#[test]
fn u32_max_ordering_is_stable() {
    let mut set = MatchSet::new();
    set.extend([
        mk(1, u32::MAX, u32::MAX),
        mk(0, 0, 1),
        mk(2, u32::MAX - 2, u32::MAX - 1),
    ]);

    assert_eq!(set.as_slice()[0], mk(0, 0, 1));
    assert_eq!(set.as_slice()[2], mk(1, u32::MAX, u32::MAX));
}

#[test]
fn contains_and_overlaps_handle_u32_max_edges() {
    let outer = mk(1, u32::MAX - 10, u32::MAX);
    let inner = mk(2, u32::MAX - 5, u32::MAX);
    let separate = mk(3, 0, 1);

    assert!(outer.contains(&inner));
    assert!(outer.overlaps(&inner));
    assert!(!outer.overlaps(&separate));
}

#[test]
fn into_vec_after_merge_remains_sorted() {
    let mut set = MatchSet::new();
    set.extend([mk(2, 10, 15), mk(1, 0, 5), mk(3, 4, 12)]);
    set.merge_overlapping();

    let vec = set.into_vec();
    assert_eq!(vec, vec![mk(1, 0, 15)]);
}

#[test]
fn extend_handles_reverse_sorted_input() {
    let mut set = MatchSet::new();
    set.extend(
        (0..50u32)
            .rev()
            .map(|index| mk(index % 5, index, index + 1)),
    );

    assert!(set.as_slice().windows(2).all(|pair| pair[0] <= pair[1]));
}

#[test]
fn filter_on_empty_set_returns_empty_set() {
    let filtered = MatchSet::new().filter_by_pattern(42);
    assert!(filtered.is_empty());
}

proptest! {
    #[test]
    fn insert_always_produces_sorted_deduped_slice(
        raw in prop::collection::vec((any::<u32>(), any::<u32>(), any::<u32>()), 0..256)
    ) {
        let mut set = MatchSet::new();
        let expected: Vec<_> = {
            let mut items: Vec<_> = raw
                .iter()
                .map(|&(pattern_id, start, end)| mk(pattern_id, start, end))
                .collect();
            items.sort_unstable();
            items.dedup();
            items
        };

        for (pattern_id, start, end) in raw {
            set.insert(mk(pattern_id, start, end));
        }

        prop_assert_eq!(set.as_slice(), expected.as_slice());
    }

    #[test]
    fn merge_is_idempotent(
        raw in prop::collection::vec((any::<u32>(), any::<u32>(), any::<u32>()), 0..256)
    ) {
        let mut set = MatchSet::new();
        set.extend(
            raw.into_iter()
                .map(|(pattern_id, start, end)| normalized_match(pattern_id, start, end))
        );

        set.merge_overlapping();
        let first_merge_result = set.as_slice().to_vec();

        set.merge_overlapping();
        prop_assert_eq!(set.as_slice(), first_merge_result.as_slice());
    }

    #[test]
    fn insert_preserves_order_large(
        raw in prop::collection::vec((any::<u32>(), any::<u32>(), any::<u32>()), 0..1000)
    ) {
        let mut set = MatchSet::new();
        let expected: Vec<_> = {
            let mut items: Vec<_> = raw
                .iter()
                .map(|&(pattern_id, start, end)| mk(pattern_id, start, end))
                .collect();
            items.sort_unstable();
            items.dedup();
            items
        };

        for (pattern_id, start, end) in raw {
            set.insert(mk(pattern_id, start, end));
        }

        prop_assert_eq!(set.as_slice(), expected.as_slice());
    }

    #[test]
    fn filter_by_pattern_is_homogenous(
        raw in prop::collection::vec((0..100u32, any::<u32>(), any::<u32>()), 0..500)
    ) {
        let mut set = MatchSet::new();
        set.extend(raw.into_iter().map(|(id, start, end)| mk(id, start, end)));

        for i in 0..100 {
            let filtered = set.filter_by_pattern(i);
            prop_assert!(filtered.iter().all(|m| m.pattern_id == i));
        }
    }

    #[test]
    fn merge_overlapping_reduces_length_or_keeps_same(
        raw in prop::collection::vec((any::<u32>(), any::<u32>(), any::<u32>()), 0..500)
    ) {
        let mut set = MatchSet::new();
        set.extend(raw.into_iter().map(|(id, start, end)| mk(id, start, end)));
        let len_before = set.len();
        set.merge_overlapping();
        let len_after = set.len();

        prop_assert!(len_after <= len_before);
    }

    #[test]
    fn new_is_consistent(
        pattern_id in any::<u32>(), start in any::<u32>(), end in any::<u32>()
    ) {
        let m = Match::new(pattern_id, start, end);

        prop_assert_eq!(m.pattern_id, pattern_id);
        prop_assert_eq!(m.start, start);
        prop_assert_eq!(m.end, end);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn pattern_counts_sum_to_total_len(
        raw in prop::collection::vec((0..50u32, any::<u32>(), any::<u32>()), 0..1000)
    ) {
        let mut set = MatchSet::new();
        set.extend(raw.into_iter().map(|(id, start, end)| mk(id, start, end)));
        let counts = set.pattern_counts();
        let total_count: usize = counts.values().sum();

        prop_assert_eq!(set.len(), total_count);
    }
}
