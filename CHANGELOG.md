# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2025-04-08

### Added

- Initial release of `matchkit`.
- `Match` struct for representing pattern matches with offset and length.
- `MatchSet` for collecting, deduplicating, filtering, and merging overlapping matches.
- `Matcher` and `BlockMatcher` traits for synchronous and asynchronous multi-pattern matching engines.
- `GpuMatch` struct for GPU-accelerated match representations.
- Shared `Error` type using `thiserror` for backend propagation.
