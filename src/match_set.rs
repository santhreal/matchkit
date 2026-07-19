use crate::Match;

/// Sorted, deduplicated collection of matches with efficient insertion.
///
/// Ensures elements are consistently ordered and handles operations
/// like duplicate removal and overlapping match merges.
///
/// # Example
///
/// ```rust
/// use matchkit::{Match, MatchSet};
///
/// let mut set = MatchSet::new();
/// set.insert(Match::new(1, 0, 10));
/// set.insert(Match::new(1, 5, 15));
/// set.merge_overlapping();
///
/// assert_eq!(set.len(), 1);
/// assert_eq!(set.as_slice()[0].end, 15);
/// ```
#[derive(Debug, Clone, Default)]
pub struct MatchSet {
    matches: Vec<Match>,
}

impl MatchSet {
    /// Create an empty match set.
    #[must_use]
    pub fn new() -> Self {
        Self {
            matches: Vec::new(),
        }
    }

    /// Create a match set with pre-allocated capacity.
    pub fn try_with_capacity(cap: usize) -> crate::error::Result<Self> {
        let mut vec = Vec::new();
        vec.try_reserve(cap)
            .map_err(|e| crate::error::Error::OutOfMemory {
                message: e.to_string(),
            })?;
        Ok(Self { matches: vec })
    }

    /// Maximum element capacity for the backing `Vec<Match>`, i.e. the largest
    /// request that cannot trigger a capacity-overflow panic.
    #[must_use]
    fn max_capacity() -> usize {
        (isize::MAX as usize) / std::mem::size_of::<Match>()
    }

    /// Create a match set with pre-allocated capacity.
    ///
    /// `cap` is clamped to `max_capacity` to prevent a
    /// capacity-overflow panic from untrusted input. This constructor still
    /// aborts if the underlying allocation fails; for OOM-safe allocation of
    /// untrusted sizes use [`try_with_capacity`](Self::try_with_capacity).
    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        let cap = cap.min(Self::max_capacity());
        Self {
            matches: Vec::with_capacity(cap),
        }
    }

    /// Insert a match, maintaining sorted order. O(log n) search + O(n) shift.
    pub fn try_insert(&mut self, m: Match) -> crate::error::Result<()> {
        if let Err(pos) = self.matches.binary_search(&m) {
            self.matches
                .try_reserve(1)
                .map_err(|e| crate::error::Error::OutOfMemory {
                    message: e.to_string(),
                })?;
            self.matches.insert(pos, m);
        }
        Ok(())
    }

    /// Insert a match, maintaining sorted order.
    pub fn insert(&mut self, m: Match) {
        if let Err(pos) = self.matches.binary_search(&m) {
            self.matches.insert(pos, m);
        }
    }

    /// Extend with multiple matches, then sort and dedup.
    pub fn try_extend(
        &mut self,
        iter: impl IntoIterator<Item = Match>,
    ) -> crate::error::Result<()> {
        let iter = iter.into_iter();
        let (lower, _) = iter.size_hint();
        if lower > 0 {
            self.matches
                .try_reserve(lower)
                .map_err(|e| crate::error::Error::OutOfMemory {
                    message: e.to_string(),
                })?;
        }
        for m in iter {
            // size_hint's lower bound is only a hint; an iterator can yield more.
            // try_reserve(1) each push (a no-op when spare capacity exists) keeps
            // every element OOM-safe instead of letting the infallible `push`
            // realloc and abort past the reserved prefix.
            self.matches
                .try_reserve(1)
                .map_err(|e| crate::error::Error::OutOfMemory {
                    message: e.to_string(),
                })?;
            self.matches.push(m);
        }
        self.matches.sort_unstable();
        self.matches.dedup();
        Ok(())
    }

    /// Extend with multiple matches, then sort and dedup.
    pub fn extend(&mut self, iter: impl IntoIterator<Item = Match>) {
        self.matches.extend(iter);
        self.matches.sort_unstable();
        self.matches.dedup();
    }

    /// Merge overlapping matches into a minimal covering set in-place.
    ///
    /// After merging, no two matches in the set overlap.
    /// Pattern ID is taken from the first match in each merged group.
    ///
    /// Note: This operation is zero-allocation and O(n) time.
    #[allow(clippy::unnecessary_wraps)] // Result kept for try_* OOM-safe API symmetry
    pub fn try_merge_overlapping(&mut self) -> crate::error::Result<()> {
        self.merge_overlapping();
        Ok(())
    }

    /// Merge into a minimal covering set in place.
    ///
    /// Genuinely overlapping byte ranges are collapsed regardless of pattern id
    /// (the merged span keeps the earlier-starting match's id), so the result is
    /// the minimal set of intervals covering every matched byte. Matches that
    /// only *touch* (`prev.end == next.start`, including zero-length points) are
    /// fused only when they share a pattern, so distinct findings sitting at the
    /// same boundary are not silently coalesced.
    pub fn merge_overlapping(&mut self) {
        if self.matches.len() < 2 {
            return;
        }

        let mut write_idx = 0;
        let mut current = self.matches[0];

        for i in 1..self.matches.len() {
            let m = self.matches[i];
            // Minimal covering set (matches are sorted start-first): genuinely
            // overlapping byte ranges collapse regardless of pattern id, keeping
            // the earlier match's id. Merely touching/adjacent runs (`end ==
            // start`, which includes zero-length points) fuse only when they
            // share a pattern, so distinct patterns at one boundary stay separate.
            if current.overlaps(&m)
                || (current.end == m.start && current.pattern_id == m.pattern_id)
            {
                current.end = current.end.max(m.end);
            } else {
                self.matches[write_idx] = current;
                write_idx += 1;
                current = m;
            }
        }

        self.matches[write_idx] = current;
        self.matches.truncate(write_idx + 1);
    }

    /// Number of matches in the set.
    #[must_use]
    pub fn len(&self) -> usize {
        self.matches.len()
    }

    /// Whether the set is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }

    /// Get matches as a slice.
    #[must_use]
    pub fn as_slice(&self) -> &[Match] {
        &self.matches
    }

    /// Returns an iterator over the matches.
    pub fn iter(&self) -> std::slice::Iter<'_, Match> {
        self.matches.iter()
    }

    /// Consume the set into a Vec.
    #[must_use]
    pub fn into_vec(self) -> Vec<Match> {
        self.matches
    }

    /// Filter matches to only those with the given pattern ID.
    #[must_use]
    pub fn filter_by_pattern(&self, pattern_id: u32) -> Self {
        Self {
            matches: self
                .matches
                .iter()
                .copied()
                .filter(|m| m.pattern_id == pattern_id)
                .collect(),
        }
    }

    /// Filter matches to only those with the given pattern ID.
    pub fn try_filter_by_pattern(&self, pattern_id: u32) -> crate::error::Result<Self> {
        let mut matches = Vec::new();
        // Exact size is not known, but it won't be larger than current set.
        matches
            .try_reserve(self.matches.len())
            .map_err(|e| crate::error::Error::OutOfMemory {
                message: e.to_string(),
            })?;
        for m in &self.matches {
            if m.pattern_id == pattern_id {
                matches.push(*m);
            }
        }
        Ok(Self { matches })
    }

    /// Count matches for each pattern ID.
    #[must_use]
    pub fn pattern_counts(&self) -> std::collections::HashMap<u32, usize> {
        let mut counts = std::collections::HashMap::new();
        for m in &self.matches {
            *counts.entry(m.pattern_id).or_insert(0) += 1;
        }
        counts
    }

    /// Count matches for each pattern ID.
    pub fn try_pattern_counts(
        &self,
    ) -> crate::error::Result<std::collections::HashMap<u32, usize>> {
        let mut counts = std::collections::HashMap::new();
        // Worst case: every match is a different pattern.
        counts
            .try_reserve(self.matches.len())
            .map_err(|e| crate::error::Error::OutOfMemory {
                message: e.to_string(),
            })?;
        for m in &self.matches {
            *counts.entry(m.pattern_id).or_insert(0) += 1;
        }
        Ok(counts)
    }

    /// Distinct pattern IDs in the set.
    #[must_use]
    pub fn pattern_ids(&self) -> Vec<u32> {
        let mut ids: Vec<u32> = self.matches.iter().map(|m| m.pattern_id).collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    /// Distinct pattern IDs in the set.
    pub fn try_pattern_ids(&self) -> crate::error::Result<Vec<u32>> {
        let mut ids = Vec::new();
        ids.try_reserve(self.matches.len())
            .map_err(|e| crate::error::Error::OutOfMemory {
                message: e.to_string(),
            })?;
        for m in &self.matches {
            ids.push(m.pattern_id);
        }
        ids.sort_unstable();
        ids.dedup();
        Ok(ids)
    }
}

impl IntoIterator for MatchSet {
    type Item = Match;
    type IntoIter = std::vec::IntoIter<Match>;

    fn into_iter(self) -> Self::IntoIter {
        self.matches.into_iter()
    }
}

impl<'a> IntoIterator for &'a MatchSet {
    type Item = &'a Match;
    type IntoIter = std::slice::Iter<'a, Match>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
