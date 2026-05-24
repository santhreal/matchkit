/// GPU-internal match representation — 3×u32, `bytemuck`-compatible.
///
/// This type maps directly to the GPU output buffer layout where each
/// match occupies exactly 12 bytes (3 × `u32`). The fields are:
///
/// - `[0]`: pattern_id
/// - `[1]`: start offset
/// - `[2]`: end offset
///
/// Note: While some GPU architectures prefer 16-byte alignment, 12-byte
/// packing is used here to minimize VRAM bandwidth at internet scale.
///
/// # Example
///
/// ```rust
/// use matchkit::{GpuMatch, Match};
///
/// let gpu_match = GpuMatch::new(1, 10, 20);
/// let m: Match = gpu_match.into();
/// assert_eq!(m.pattern_id, 1);
/// ```
#[repr(C)]
#[non_exhaustive]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuMatch(
    /// Raw GPU match fields: `[pattern_id, start, end]`.
    pub [u32; 3],
);

/// A match result from pattern scanning.
///
/// Uses `u32` offsets for GPU buffer compatibility. For inputs larger
/// than 4 GiB, scan in chunks and add the chunk base offset.
///
/// Uses `repr(C)` for GPU buffer compatibility. 12 bytes per match.
///
/// # Example
///
/// ```rust
/// use matchkit::Match;
///
/// let m = Match::new(0, 5, 10);
/// assert_eq!(m.len(), 5);
/// assert!(m.contains(&Match::new(0, 6, 8)));
/// ```
#[repr(C)]
#[non_exhaustive]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Match {
    /// Index of the pattern that matched (0-based, in insertion order).
    /// Placed first for efficient GPU scatter grouping.
    pub pattern_id: u32,
    /// Byte offset where the match starts (inclusive).
    pub start: u32,
    /// Byte offset where the match ends (exclusive).
    pub end: u32,
}

impl PartialEq for Match {
    fn eq(&self, other: &Self) -> bool {
        self.pattern_id == other.pattern_id && self.start == other.start && self.end == other.end
    }
}

impl Eq for Match {}

impl std::hash::Hash for Match {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.pattern_id.hash(state);
        self.start.hash(state);
        self.end.hash(state);
    }
}

impl PartialOrd for Match {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Match {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.start
            .cmp(&other.start)
            .then(self.pattern_id.cmp(&other.pattern_id))
            .then(self.end.cmp(&other.end))
    }
}

impl Match {
    /// Construct a match from its public fields.
    #[must_use]
    pub const fn new(pattern_id: u32, start: u32, end: u32) -> Self {
        Self {
            pattern_id,
            start,
            end,
        }
    }

    /// Legacy constructor (compatibility with from_parts).
    #[must_use]
    pub const fn from_parts(pattern_id: u32, start: u32, end: u32) -> Self {
        Self::new(pattern_id, start, end)
    }

    /// Returns `true` if this match's byte range fully contains `other`.
    #[must_use]
    pub const fn contains(&self, other: &Match) -> bool {
        self.start <= other.start && self.end >= other.end
    }

    /// Returns `true` if this match's byte range overlaps with `other`.
    #[must_use]
    pub const fn overlaps(&self, other: &Match) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// Byte length of the matched region.
    #[must_use]
    pub const fn len(&self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    /// Returns `true` if the match has zero length.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

impl GpuMatch {
    /// Construct a GPU match from its public fields.
    #[must_use]
    pub const fn new(pattern_id: u32, start: u32, end: u32) -> Self {
        Self([pattern_id, start, end])
    }
}

impl From<GpuMatch> for Match {
    fn from(value: GpuMatch) -> Self {
        Self {
            pattern_id: value.0[0],
            start: value.0[1],
            end: value.0[2],
        }
    }
}

impl From<Match> for GpuMatch {
    fn from(value: Match) -> Self {
        Self([value.pattern_id, value.start, value.end])
    }
}

/// A batch of matches in Structure-of-Arrays (SoA) format.
///
/// This layout is significantly more efficient for SIMD operations and
/// GPU bandwidth when only a subset of fields (e.g., just `pattern_id`)
/// needs to be scanned.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MatchBatch {
    /// Vector of pattern IDs.
    pub pattern_ids: Vec<u32>,
    /// Vector of start offsets.
    pub starts: Vec<u32>,
    /// Vector of end offsets.
    pub ends: Vec<u32>,
}

impl MatchBatch {
    /// Create an empty batch.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a batch with pre-allocated capacity.
    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            pattern_ids: Vec::with_capacity(cap),
            starts: Vec::with_capacity(cap),
            ends: Vec::with_capacity(cap),
        }
    }

    /// Number of matches in the batch.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pattern_ids.len()
    }

    /// Whether the batch is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pattern_ids.is_empty()
    }

    /// Add a match to the batch.
    pub fn push(&mut self, m: Match) {
        self.pattern_ids.push(m.pattern_id);
        self.starts.push(m.start);
        self.ends.push(m.end);
    }

    /// Clear the batch.
    pub fn clear(&mut self) {
        self.pattern_ids.clear();
        self.starts.clear();
        self.ends.clear();
    }

    /// Convert AoS slice to SoA batch.
    #[must_use]
    pub fn from_slice(matches: &[Match]) -> Self {
        let mut batch = Self::with_capacity(matches.len());
        for m in matches {
            batch.push(*m);
        }
        batch
    }

    /// Convert SoA batch to AoS vector.
    #[must_use]
    pub fn into_vec(self) -> Vec<Match> {
        let mut matches = Vec::with_capacity(self.len());
        for i in 0..self.len() {
            matches.push(Match::new(
                self.pattern_ids[i],
                self.starts[i],
                self.ends[i],
            ));
        }
        matches
    }
}
