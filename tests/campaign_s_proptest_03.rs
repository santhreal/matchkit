//! S-proptest-03 — matchkit mass proptest: match-set invariants, no panic on arbitrary offsets.

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

macro_rules! match_cases {
    ($($name:ident => |$raw:ident| $body:block),+ $(,)?) => {
        $(
            proptest! {
                #![proptest_config(ProptestConfig::with_cases(64))]
                #[test]
                fn $name(
                    $raw in prop::collection::vec((any::<u32>(), any::<u32>(), any::<u32>()), 0..128),
                ) {
                    $body
                }
            }
        )+
    };
}

match_cases! {
    p00_insert_sorted_deduped => |raw| {
        let mut set = MatchSet::new();
        let mut expected: Vec<_> = raw.iter().map(|&(p, s, e)| mk(p, s, e)).collect();
        expected.sort_unstable();
        expected.dedup();
        for (p, s, e) in raw {
            set.insert(mk(p, s, e));
        }
        prop_assert_eq!(set.as_slice(), expected.as_slice());
    },
    p01_extend_sorted_deduped => |raw| {
        let mut set = MatchSet::new();
        set.extend(raw.into_iter().map(|(p, s, e)| mk(p, s, e)));
        for pair in set.as_slice().windows(2) {
            prop_assert!(pair[0] <= pair[1]);
        }
    },
    p02_merge_no_overlaps => |raw| {
        let mut set = MatchSet::new();
        set.extend(raw.into_iter().map(|(p, s, e)| normalized(p, s, e)));
        set.merge_overlapping();
        for pair in set.as_slice().windows(2) {
            prop_assert!(!pair[0].overlaps(&pair[1]));
        }
    },
    p03_merge_idempotent => |raw| {
        let mut set = MatchSet::new();
        set.extend(raw.into_iter().map(|(p, s, e)| normalized(p, s, e)));
        set.merge_overlapping();
        let once = set.as_slice().to_vec();
        set.merge_overlapping();
        prop_assert_eq!(set.as_slice(), once.as_slice());
    },
    p04_len_le_input => |raw| {
        let n = raw.len();
        let mut set = MatchSet::new();
        set.extend(raw.into_iter().map(|(p, s, e)| mk(p, s, e)));
        prop_assert!(set.len() <= n);
    },
    p05_pattern_counts_sum => |raw| {
        let mut set = MatchSet::new();
        set.extend(raw.into_iter().map(|(p, s, e)| mk(p, s, e)));
        let sum: usize = set.pattern_counts().values().sum();
        prop_assert_eq!(sum, set.len());
    },
    p06_filter_homogeneous => |raw| {
        let mut set = MatchSet::new();
        set.extend(raw.into_iter().map(|(p, s, e)| mk(p, s, e)));
        for id in 0..16u32 {
            let filtered = set.filter_by_pattern(id);
            prop_assert!(filtered.iter().all(|m| m.pattern_id == id));
        }
    },
    p07_gpu_roundtrip => |raw| {
        for (p, s, e) in raw.iter().take(8) {
            let m = mk(*p, *s, *e);
            let gpu: GpuMatch = m.into();
            let back: Match = gpu.into();
            prop_assert_eq!(back, m);
        }
    },
    p08_len_never_panics => |raw| {
        let mut set = MatchSet::new();
        set.extend(raw.into_iter().map(|(p, s, e)| mk(p, s, e)));
        let _ = set.len();
    },
    p09_is_empty_consistent => |raw| {
        let mut set = MatchSet::new();
        set.extend(raw.into_iter().map(|(p, s, e)| mk(p, s, e)));
        prop_assert_eq!(set.is_empty(), set.len() == 0);
    },
    p10_pattern_ids_sorted => |raw| {
        let mut set = MatchSet::new();
        set.extend(raw.into_iter().map(|(p, s, e)| mk(p, s, e)));
        let ids = set.pattern_ids();
        for pair in ids.windows(2) {
            prop_assert!(pair[0] <= pair[1]);
        }
    },
}

