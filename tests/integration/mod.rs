//! Internet-scale stress tests for matchkit.
//! These tests simulate conditions at 150M files / 10TB corpus scale.

use matchkit::{Match, MatchSet};

#[test]
fn matchset_handles_1m_matches() {
    let mut set = MatchSet::new();
    for i in 0u32..1_000_000 {
        set.insert(Match::from_parts(i % 1000, i, i + 10));
    }
    assert_eq!(set.len(), 1_000_000);
    // merge_overlapping should handle this without OOM
    set.merge_overlapping();
    assert!(set.len() < 1_000_000); // overlapping matches merged
}

#[test]
fn matchset_filter_on_large_set() {
    let mut set = MatchSet::new();
    for i in 0u32..100_000 {
        set.insert(Match::from_parts(i % 50, i * 100, i * 100 + 50));
    }
    // Filter should be fast even on 100K matches
    let filtered = set.filter_by_pattern(25);
    assert_eq!(filtered.len(), 2000); // 100K / 50 patterns
}

#[test]
fn match_at_u32_boundary() {
    let m = Match::from_parts(0, u32::MAX - 10, u32::MAX);
    assert_eq!(m.len(), 10);
    assert!(!m.is_empty());
}

#[test]
fn matchset_with_identical_matches_deduplicates() {
    let mut set = MatchSet::new();
    for _ in 0..10_000 {
        set.insert(Match::from_parts(0, 100, 200));
    }
    assert_eq!(set.len(), 1); // all duplicates removed
}

#[test]
fn hundred_thousand_matches_extend_and_dedup() {
    let mut set = MatchSet::with_capacity(100_000);
    let mut expected = std::collections::BTreeSet::new();
    let mut inputs = Vec::with_capacity(200_000);

    for index in 0..100_000u32 {
        let mat = Match::from_parts(index % 64, index, index + 1);
        expected.insert(mat);
        inputs.push(mat);
        inputs.push(mat);
    }

    set.extend(inputs);

    assert_eq!(set.len(), expected.len());
    assert_eq!(
        set.as_slice().first().copied(),
        Some(Match::from_parts(0, 0, 1))
    );
    assert_eq!(
        set.as_slice().last().copied(),
        Some(Match::from_parts(31, 99_999, 100_000))
    );
}

#[test]
fn hundred_thousand_matches_pattern_counts_sum_correctly() {
    let mut set = MatchSet::with_capacity(100_000);
    let mut expected: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();

    for index in 0..100_000u32 {
        let mat = Match::from_parts(index % 100, index, index + 2);
        *expected.entry(mat.pattern_id).or_insert(0) += 1;
        set.insert(mat);
    }

    assert_eq!(set.pattern_counts(), expected);
}

#[test]
fn simulate_streaming_scan_pipeline() {
    let mut total_set = MatchSet::new();
    let chunk_size = 1024;
    let num_chunks = 100;

    // Simulate getting matches back from multiple chunks sequentially
    for chunk_idx in 0..num_chunks {
        let base_offset = chunk_idx * chunk_size;

        let chunk_matches = vec![
            Match::from_parts(1, base_offset + 10, base_offset + 20),
            Match::from_parts(2, base_offset + 15, base_offset + 25), // Overlaps
            Match::from_parts(
                1,
                base_offset + chunk_size - 5,
                base_offset + chunk_size + 5,
            ), // Overlaps chunk boundary
        ];

        total_set.extend(chunk_matches);
    }

    // Total matches should be 3 * num_chunks before merging
    assert_eq!(total_set.len(), 3 * num_chunks as usize);

    total_set.merge_overlapping();

    // Because the last match in each chunk overlaps the first match of the next,
    // they should merge across chunk boundaries.
    // Within a chunk:
    // Match 1: 10..20 and Match 2: 15..25 overlap -> merged into 10..25
    // Match 3: 1019..1029 overlaps with next chunk's Match 1: 1034..1044? NO.
    // Chunk 0: M3 is 1019..1029. Chunk 1 base is 1024. M1 is 1034..1044.
    // They do not overlap (1029 < 1034).

    // Merged count: Each chunk yields two groups: (10..25) and (1019..1029).
    assert_eq!(total_set.len(), 2 * num_chunks as usize);
}

#[test]
fn massive_data_scale_simulation() {
    let mut set = MatchSet::new();
    let num_patterns = 10_000;

    // Create matches for 10,000 distinct patterns with sporadic overlaps
    let mut matches = Vec::with_capacity(1_000_000);
    for i in 0..500_000 {
        matches.push(Match::from_parts(i % num_patterns, i * 2, i * 2 + 5));
        matches.push(Match::from_parts(
            (i + 1) % num_patterns,
            i * 2 + 1,
            i * 2 + 6,
        ));
    }

    set.extend(matches);
    assert_eq!(set.len(), 1_000_000);

    set.merge_overlapping();
    // After merge, since every match overlaps with the next one by 4 bytes,
    // they should all merge into a few giant matches depending on pattern IDs.
    // However, merge_overlapping takes the first pattern_id in the overlapping group.
    // The entire range 0..1000005 should be covered.

    let slice = set.as_slice();
    assert!(slice.len() < 100);
    assert_eq!(slice[0].start, 0);
    assert!(slice[0].end >= 1_000_000);
}

#[test]
fn matchset_filter_and_merge_performance() {
    let mut set = MatchSet::new();
    let num_patterns = 100;

    // Create matches for 100 patterns with random overlaps
    let mut matches = Vec::with_capacity(500_000);
    for i in 0..250_000 {
        matches.push(Match::from_parts(i % num_patterns, i * 3, i * 3 + 10));
        matches.push(Match::from_parts(
            (i + 1) % num_patterns,
            i * 3 + 2,
            i * 3 + 12,
        ));
    }

    set.extend(matches);
    assert_eq!(set.len(), 500_000);

    // Filter out a single pattern
    let filtered = set.filter_by_pattern(50);
    assert_eq!(filtered.len(), 5000);

    // Merge overlapping on the full set
    set.merge_overlapping();

    // Because of the dense overlapping ranges, everything should merge
    assert!(set.len() < 10);

    // Verify properties
    let slice = set.as_slice();
    for pair in slice.windows(2) {
        assert!(pair[0] <= pair[1]);
        assert!(!pair[0].overlaps(&pair[1]));
    }
}
mod fault_injection;
