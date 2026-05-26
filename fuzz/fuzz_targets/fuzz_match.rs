#![no_main]
use libfuzzer_sys::fuzz_target;
use matchkit::{GpuMatch, Match};

fuzz_target!(|data: &[u8]| {
    if data.len() < 12 {
        return;
    }

    let pattern_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let start = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let end = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);

    let m = Match::new(pattern_id, start, end);

    let gpu: GpuMatch = m.into();
    let restored: Match = gpu.into();
    assert_eq!(m, restored);

    let bytes = bytemuck::bytes_of(&gpu);
    assert_eq!(bytes.len(), 12);
    let restored_gpu: &GpuMatch = bytemuck::from_bytes(bytes);
    assert_eq!(restored_gpu.0, gpu.0);
});
