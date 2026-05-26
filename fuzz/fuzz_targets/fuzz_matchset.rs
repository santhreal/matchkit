#![no_main]
use libfuzzer_sys::fuzz_target;
use matchkit::{Match, MatchSet};

fuzz_target!(|data: &[u8]| {
    if data.len() < 12 {
        return;
    }
    let mut set = MatchSet::with_capacity(data.len() / 12);
    for chunk in data.chunks_exact(12) {
        let pattern_id = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        let start = u32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
        let end = u32::from_le_bytes([chunk[8], chunk[9], chunk[10], chunk[11]]);
        set.insert(Match::new(pattern_id, start, end));
    }
    set.merge_overlapping();
    let _ = set.pattern_ids();
    let _ = set.pattern_counts();
});
