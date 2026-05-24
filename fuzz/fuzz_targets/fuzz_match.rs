#![no_main]
use libfuzzer_sys::fuzz_target;
use matchkit::{Match, GpuMatch};

fuzz_target!(|data: &[u8]| {
    if data.len() < 16 { return; }

    // Construct Match from arbitrary bytes
    let pattern_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let start = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let end = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
    let padding = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);

    let m = Match::from_parts_with_padding(pattern_id, start, end, padding);

    // Verify round-trip through GpuMatch
    let gpu = GpuMatch([m.pattern_id, m.start, m.end, m.padding]);
    let restored: Match = gpu.into();
    assert_eq!(m, restored);

    // Verify bytemuck round-trip
    let bytes = bytemuck::bytes_of(&gpu);
    assert_eq!(bytes.len(), 16);
    let restored_gpu: &GpuMatch = bytemuck::from_bytes(bytes);
    assert_eq!(restored_gpu.0, gpu.0);

    // Equality ignores padding
    let m2 = Match::from_parts_with_padding(pattern_id, start, end, padding.wrapping_add(1));
    assert_eq!(m, m2);
});
