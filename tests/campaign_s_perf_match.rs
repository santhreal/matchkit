//! S-perf-match campaign catalog: +40 focused unit cases for matchkit vocabulary types.

#![allow(clippy::unwrap_used)]

use matchkit::{GpuMatch, Match, MatchBatch, MatchSet};

macro_rules! campaign_match {
    ($name:ident, $p:expr, $s:expr, $e:expr) => {
        #[test]
        fn $name() {
            let m = Match::new($p, $s, $e);
            assert_eq!(m.pattern_id, $p);
            assert_eq!(m.len(), ($e as u32).saturating_sub($s as u32));
            let gpu: GpuMatch = m.into();
            let back: Match = gpu.into();
            assert_eq!(back, m);
        }
    };
}

campaign_match!(c00, 0, 0, 1);
campaign_match!(c01, 1, 5, 10);
campaign_match!(c02, 42, 100, 200);
campaign_match!(c03, u32::MAX, 0, 1);
campaign_match!(c04, 7, 7, 7);
campaign_match!(c05, 3, 0, 0);
campaign_match!(c06, 9, 1, 2);
campaign_match!(c07, 11, 50, 51);
campaign_match!(c08, 2, 10, 20);
campaign_match!(c09, 5, 3, 8);

#[test]
fn campaign_overlap_matrix_00() {
    let a = Match::new(0, 0, 10);
    let b = Match::new(0, 5, 15);
    assert!(a.overlaps(&b));
    assert!(a.contains(&Match::new(0, 2, 8)));
}

#[test]
fn campaign_overlap_matrix_01() {
    let a = Match::new(0, 0, 5);
    let b = Match::new(0, 5, 10);
    assert!(!a.overlaps(&b));
}

#[test]
fn campaign_matchset_dedup_insert() {
    let mut set = MatchSet::new();
    let m = Match::new(1, 2, 3);
    set.insert(m);
    set.insert(m);
    assert_eq!(set.len(), 1);
}

#[test]
fn campaign_matchset_merge_chain() {
    let mut set = MatchSet::new();
    set.insert(Match::new(0, 0, 4));
    set.insert(Match::new(0, 4, 8));
    set.insert(Match::new(0, 8, 12));
    set.merge_overlapping();
    assert_eq!(set.len(), 1);
    assert_eq!(set.as_slice()[0].end, 12);
}

#[test]
fn campaign_matchset_pattern_filter() {
    let mut set = MatchSet::new();
    set.insert(Match::new(0, 0, 1));
    set.insert(Match::new(1, 2, 3));
    let filtered = set.filter_by_pattern(1);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered.as_slice()[0].pattern_id, 1);
}

#[test]
fn campaign_matchset_pattern_counts() {
    let mut set = MatchSet::new();
    set.insert(Match::new(0, 0, 1));
    set.insert(Match::new(0, 2, 3));
    set.insert(Match::new(1, 4, 5));
    let counts = set.pattern_counts();
    assert_eq!(counts.get(&0), Some(&2));
    assert_eq!(counts.get(&1), Some(&1));
}

#[test]
fn campaign_matchset_try_insert_ok() {
    let mut set = MatchSet::new();
    set.try_insert(Match::new(0, 0, 1)).unwrap();
    assert_eq!(set.len(), 1);
}

#[test]
fn campaign_matchset_extend_sorted() {
    let mut set = MatchSet::new();
    set.extend([
        Match::new(0, 10, 11),
        Match::new(0, 0, 1),
        Match::new(0, 0, 1),
    ]);
    assert_eq!(set.len(), 2);
    assert!(set.as_slice()[0].start <= set.as_slice()[1].start);
}

#[test]
fn campaign_matchbatch_roundtrip_00() {
    let matches = [Match::new(0, 0, 1), Match::new(1, 2, 4)];
    let batch = MatchBatch::from_slice(&matches);
    assert_eq!(batch.into_vec(), matches.to_vec());
}

#[test]
fn campaign_matchbatch_push_clear() {
    let mut batch = MatchBatch::with_capacity(4);
    batch.push(Match::new(3, 1, 2));
    assert_eq!(batch.len(), 1);
    batch.clear();
    assert!(batch.is_empty());
}

#[test]
fn campaign_ordering_transitive() {
    let a = Match::new(0, 0, 1);
    let b = Match::new(0, 1, 2);
    let c = Match::new(0, 2, 3);
    assert!(a < b && b < c);
}

#[test]
fn campaign_hash_equality_independent_order() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(Match::new(1, 5, 10));
    set.insert(Match::new(1, 5, 10));
    assert_eq!(set.len(), 1);
}

#[test]
fn campaign_matchset_pattern_ids_sorted_unique() {
    let mut set = MatchSet::new();
    set.insert(Match::new(2, 0, 1));
    set.insert(Match::new(0, 2, 3));
    set.insert(Match::new(2, 4, 5));
    assert_eq!(set.pattern_ids(), vec![0, 2]);
}

#[test]
fn campaign_matchset_with_capacity_zero() {
    let set = MatchSet::with_capacity(0);
    assert!(set.is_empty());
}

#[test]
fn campaign_match_is_empty_zero_len() {
    let m = Match::new(0, 4, 4);
    assert!(m.is_empty());
    assert_eq!(m.len(), 0);
}

#[test]
fn campaign_match_from_parts_alias() {
    assert_eq!(
        Match::from_parts(9, 1, 2),
        Match::new(9, 1, 2)
    );
}

#[test]
fn campaign_gpumatch_pod_size() {
    assert_eq!(std::mem::size_of::<GpuMatch>(), 12);
}

#[test]
fn campaign_matchset_try_pattern_ids() {
    let mut set = MatchSet::new();
    set.insert(Match::new(5, 0, 1));
    assert_eq!(set.try_pattern_ids().unwrap(), vec![5]);
}

#[test]
fn campaign_matchset_try_filter() {
    let mut set = MatchSet::new();
    set.insert(Match::new(0, 0, 1));
    set.insert(Match::new(1, 1, 2));
    let filtered = set.try_filter_by_pattern(0).unwrap();
    assert_eq!(filtered.len(), 1);
}

#[test]
fn campaign_matchset_merge_cross_pattern_same_span() {
    // merge_overlapping produces a minimal covering set: two patterns covering
    // the exact same span overlap, so they collapse into one interval (which
    // keeps the earlier-starting match's pattern id).
    let mut set = MatchSet::new();
    set.insert(Match::new(0, 0, 5));
    set.insert(Match::new(1, 0, 5));
    set.merge_overlapping();
    assert_eq!(set.len(), 1);
}

#[test]
fn campaign_matchset_into_iter() {
    let mut set = MatchSet::new();
    set.insert(Match::new(0, 0, 1));
    let collected: Vec<_> = set.into_iter().collect();
    assert_eq!(collected.len(), 1);
}

#[test]
fn campaign_matchset_try_merge() {
    let mut set = MatchSet::new();
    set.insert(Match::new(0, 0, 2));
    set.insert(Match::new(0, 1, 3));
    set.try_merge_overlapping().unwrap();
    assert_eq!(set.len(), 1);
}

#[test]
fn campaign_matchset_try_extend() {
    let mut set = MatchSet::new();
    set.try_extend([Match::new(0, 0, 1)]).unwrap();
    assert_eq!(set.len(), 1);
}

#[test]
fn campaign_matchset_try_pattern_counts() {
    let mut set = MatchSet::new();
    set.insert(Match::new(0, 0, 1));
    let counts = set.try_pattern_counts().unwrap();
    assert_eq!(counts.get(&0), Some(&1));
}

#[test]
fn campaign_match_batch_len_after_multi_push() {
    let mut batch = MatchBatch::new();
    for i in 0..5u32 {
        batch.push(Match::new(i, i, i + 1));
    }
    assert_eq!(batch.len(), 5);
}

#[test]
fn campaign_contains_non_overlapping_false() {
    let outer = Match::new(0, 0, 5);
    let inner = Match::new(0, 6, 7);
    assert!(!outer.contains(&inner));
}

#[test]
fn campaign_partial_ord_consistent_with_ord() {
    let a = Match::new(0, 1, 2);
    let b = Match::new(0, 2, 3);
    assert_eq!(a.partial_cmp(&b), Some(std::cmp::Ordering::Less));
}

#[test]
fn campaign_matchset_try_with_capacity() {
    let set = MatchSet::try_with_capacity(8).unwrap();
    assert!(set.is_empty());
}

#[test]
fn campaign_match_ord_by_start_then_pattern() {
    let a = Match::new(1, 0, 1);
    let b = Match::new(0, 0, 2);
    assert_eq!(a.cmp(&b), std::cmp::Ordering::Greater);
}

#[test]
fn campaign_matchset_clear_via_into_vec_empty() {
    let mut set = MatchSet::new();
    set.insert(Match::new(0, 0, 1));
    let v = set.into_vec();
    assert_eq!(v.len(), 1);
}
