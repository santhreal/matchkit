use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use matchkit::{Match, MatchSet};

fn bench_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("matchset_insert");
    for count in [100, 1_000, 10_000] {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &n| {
            b.iter(|| {
                let mut set = MatchSet::with_capacity(n);
                for i in 0..n as u32 {
                    set.insert(Match::from_parts(i % 10, i, i + 5));
                }
                set
            });
        });
    }
    group.finish();
}

fn bench_merge_overlapping(c: &mut Criterion) {
    c.bench_function("merge_overlapping_10k", |b| {
        let matches: Vec<Match> = (0..10_000u32)
            .map(|i| Match::from_parts(0, i * 3, i * 3 + 5))
            .collect();
        b.iter(|| {
            let mut set = MatchSet::new();
            set.extend(matches.iter().copied());
            set.merge_overlapping();
            set
        });
    });
}

criterion_group!(benches, bench_insert, bench_merge_overlapping);
criterion_main!(benches);
