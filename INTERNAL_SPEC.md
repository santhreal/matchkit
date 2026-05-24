# matchkit — Internal Spec

> This file is gitignored. It exists for agents and internal development. Never committed to public repos.

## Identity
Vocabulary crate for multi-pattern matching — the shared `Match` struct, `Matcher` traits, and common errors used across the entire Santh matching ecosystem.

## Purpose
Without matchkit, every matching crate (warpstate, simdsieve, dfajit, vyre, yaragpu) would redefine its own match representation, making interoperability impossible and forcing dependency bloat.

## North Star
Be as fundamental and trusted as `std::time::Duration` or `http::StatusCode` — a universal vocabulary type that no Rust developer questions. Legendary means zero breaking changes, guaranteed GPU-compatible memory layout, and traits so ergonomic that third-party engines naturally implement them.

## Role in Ecosystem
- **Depends on:** async-trait, bytemuck, thiserror
- **Depended on by:** vyre, warpstate, dfajit, simdsieve, yaragpu, patterndb, secjit, tools/warpscan, tools/surgec, scanner/npmfeed, performance/pipeline/scanpipe, performance/gpu/gpudecode, performance/io/netshift, performance/pipeline/fusedpipe, tools/warpnet, tools/warpgrep, performance/gpu/gputokenize, performance/analysis/matchcorr, performance/io/scanwire, performance/analysis/reportkit
- **Relationship to warpscan:** warpscan imports matchkit directly for match types and also gets it transitively via warpstate.
- **Standalone value:** YES — essential for any Rust project doing multi-pattern matching within the Santh ecosystem or beyond.

## Invariants
- `Match` memory layout (`pattern_id: u32`, `start: u32`, `end: u32`, 12 bytes total) is frozen and must remain GPU-compatible.
- `MatchSet` is always sorted and deduplicated after `merge_overlapping`.
- `Match` and `GpuMatch` roundtrip losslessly.
- All public types are `Send + Sync`.

## Boundaries
- Does not implement any matching algorithm — only types and traits.
- Does not depend on GPU crates, regex crates, or JIT crates — strictly zero heavy dependencies.
- Does not handle serialization beyond what `bytemuck` provides for layout.

## Quality State
- Tests: 8 explicit test targets, 13 inline tests, 13 test files (~34 total)
- Lint preamble: yes (`#![warn(clippy::pedantic)]`, `#![forbid(unsafe_code)]`, `#![warn(missing_docs)]`, unwrap deny block)
- `#![forbid(unsafe_code)]`: yes
- Doc coverage: ~90% (small surface area, every item documented)
- Known issues: `BlockMatcher` trait could use async variant; no known bugs.
