//! Criterion benchmarks for matchkit match set operations.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreadable_literal,
    clippy::doc_markdown,
    clippy::similar_names,
    clippy::ptr_as_ptr,
    clippy::borrow_as_ptr,
    clippy::ref_as_ptr,
    clippy::cast_ptr_alignment,
    clippy::useless_vec,
    clippy::items_after_statements,
    clippy::io_other_error,
    clippy::stable_sort_primitive,
    clippy::unnecessary_wraps,
    clippy::single_char_pattern,
    clippy::cast_sign_loss,
    clippy::uninlined_format_args,
    clippy::cast_possible_truncation,
    clippy::len_zero,
    clippy::elidable_lifetime_names,
    missing_docs
)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use matchkit::{Match, MatchSet};

/// Benchmark MatchSet insertion with varying input sizes.
fn bench_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("matchset_insert");
    for count in [100, 1_000, 10_000] {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &n| {
            b.iter(|| {
                let mut set = MatchSet::with_capacity(n);
                let n_u32 = u32::try_from(n).unwrap();
                for i in 0..n_u32 {
                    set.insert(Match::from_parts(i % 10, i, i + 5));
                }
                set
            });
        });
    }
    group.finish();
}

/// Benchmark merging 10,000 overlapping matches.
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
