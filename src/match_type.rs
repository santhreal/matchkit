/// GPU-internal match representation — 4×u32, `bytemuck`-compatible.
///
/// This type maps directly to the GPU output buffer layout where each
/// match occupies exactly 16 bytes (4 × `u32`). The fields are:
///
/// - `[0]`: pattern_id
/// - `[1]`: start offset
/// - `[2]`: end offset
/// - `[3]`: reserved (padding / flags)
///
/// # Example
///
/// ```rust
/// use matchkit::{GpuMatch, Match};
///
/// let gpu_match = GpuMatch([1, 10, 20, 0]);
/// let m: Match = gpu_match.into();
/// assert_eq!(m.pattern_id, 1);
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuMatch(
    /// Raw GPU match fields: `[pattern_id, start, end, padding]`.
    pub [u32; 4],
);

/// A match result from pattern scanning.
///
/// Uses `u32` offsets for GPU buffer compatibility. For inputs larger
/// than 4 GiB, scan in chunks and add the chunk base offset.
///
/// Uses `repr(C)` for GPU buffer compatibility. 16 bytes per match.
///
/// # Example
///
/// ```rust
/// use matchkit::Match;
///
/// let m = Match::from_parts(0, 5, 10);
/// assert_eq!(m.len(), 5);
/// assert!(m.contains(&Match::from_parts(0, 6, 8)));
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Match {
    /// Index of the pattern that matched (0-based, in insertion order).
    pub pattern_id: u32,
    /// Byte offset where the match starts (inclusive).
    pub start: u32,
    /// Byte offset where the match ends (exclusive).
    pub end: u32,
    /// Padding for GPU alignment. Ignored in equality comparisons.
    pub padding: u32,
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
    pub const fn from_parts(pattern_id: u32, start: u32, end: u32) -> Self {
        Self {
            pattern_id,
            start,
            end,
            padding: 0,
        }
    }

    /// Returns the padding field (reserved for future use / GPU flags).
    #[must_use]
    pub const fn padding(&self) -> u32 {
        self.padding
    }

    /// Create a match with explicit padding value.
    ///
    /// Used internally by GPU backends that pack flags into the padding field.
    #[must_use]
    pub const fn from_parts_with_padding(
        pattern_id: u32,
        start: u32,
        end: u32,
        padding: u32,
    ) -> Self {
        Self {
            pattern_id,
            start,
            end,
            padding,
        }
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

impl From<GpuMatch> for Match {
    fn from(value: GpuMatch) -> Self {
        Self {
            pattern_id: value.0[0],
            start: value.0[1],
            end: value.0[2],
            padding: value.0[3],
        }
    }
}

impl From<Match> for GpuMatch {
    fn from(value: Match) -> Self {
        Self([value.pattern_id, value.start, value.end, value.padding])
    }
}
