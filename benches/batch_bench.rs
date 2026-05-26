//! Criterion bench for MatchBatch SoA conversions.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use matchkit::{Match, MatchBatch};

fn bench_batch_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("matchbatch_roundtrip");
    for count in [100, 1_000, 10_000] {
        let matches: Vec<Match> = (0..count as u32)
            .map(|i| Match::new(i % 8, i * 2, i * 2 + 3))
            .collect();
        group.bench_with_input(BenchmarkId::from_parameter(count), &matches, |b, m| {
            b.iter(|| {
                let batch = MatchBatch::from_slice(m);
                batch.into_vec()
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_batch_roundtrip);
criterion_main!(benches);
