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
/// set.insert(Match::from_parts(1, 0, 10));
/// set.insert(Match::from_parts(1, 5, 15));
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
        vec.try_reserve(cap).map_err(|e| {
            crate::error::Error::OutOfMemory {
                message: e.to_string(),
            }
        })?;
        Ok(Self { matches: vec })
    }

    /// Create a match set with pre-allocated capacity (legacy interface, may panic on OOM).
    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        // Clamp to the maximum representable capacity for a Vec<Match> to prevent
        // unbounded allocation requests from untrusted input.
        let max_cap = (isize::MAX as usize) / std::mem::size_of::<Match>();
        let cap = cap.min(max_cap);
        Self {
            matches: Vec::with_capacity(cap),
        }
    }

    /// Insert a match, maintaining sorted order. O(log n) search + O(n) shift.
    pub fn try_insert(&mut self, m: Match) -> crate::error::Result<()> {
        if let Err(pos) = self.matches.binary_search(&m) {
            self.matches.try_reserve(1).map_err(|e| {
                crate::error::Error::OutOfMemory {
                    message: e.to_string(),
                }
            })?;
            self.matches.insert(pos, m);
        }
        Ok(())
    }

    /// Insert a match, maintaining sorted order (legacy interface, may panic on OOM).
    pub fn insert(&mut self, m: Match) {
        match self.matches.binary_search(&m) {
            Ok(_) => {} // duplicate — skip
            Err(pos) => self.matches.insert(pos, m),
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
            self.matches.try_reserve(lower).map_err(|e| {
                crate::error::Error::OutOfMemory {
                    message: e.to_string(),
                }
            })?;
        }
        for m in iter {
            self.matches.push(m);
        }
        self.matches.sort_unstable();
        self.matches.dedup();
        Ok(())
    }

    /// Extend with multiple matches, then sort and dedup (legacy interface, may panic on OOM).
    pub fn extend(&mut self, iter: impl IntoIterator<Item = Match>) {
        self.matches.extend(iter);
        self.matches.sort_unstable();
        self.matches.dedup();
    }

    /// Merge overlapping matches into a minimal covering set.
    ///
    /// After merging, no two matches in the set overlap.
    /// Pattern ID is taken from the first match in each merged group.
    pub fn try_merge_overlapping(&mut self) -> crate::error::Result<()> {
        if self.matches.len() < 2 {
            return Ok(());
        }
        let mut merged = Vec::new();
        merged.try_reserve(self.matches.len()).map_err(|e| {
            crate::error::Error::OutOfMemory {
                message: e.to_string(),
            }
        })?;
        let mut current = self.matches[0];

        for m in &self.matches[1..] {
            if current.overlaps(m) || current.end == m.start {
                current.end = current.end.max(m.end);
            } else {
                merged.push(current);
                current = *m;
            }
        }
        merged.push(current);
        self.matches = merged;
        Ok(())
    }

    /// Merge overlapping matches into a minimal covering set (legacy interface, may panic on OOM).
    pub fn merge_overlapping(&mut self) {
        if self.matches.len() < 2 {
            return;
        }
        let mut merged: Vec<Match> = Vec::with_capacity(self.matches.len());
        let mut current = self.matches[0];

        for m in &self.matches[1..] {
            if current.overlaps(m)
                || (current.pattern_id == m.pattern_id && current.end == m.start)
            {
                // Extend current to cover both (overlapping, or adjacent with same pattern)
                current.end = current.end.max(m.end);
            } else {
                merged.push(current);
                current = *m;
            }
        }
        merged.push(current);
        self.matches = merged;
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

    /// Filter matches to only those with the given pattern ID, returning an error on OOM.
    pub fn try_filter_by_pattern(&self, pattern_id: u32) -> crate::error::Result<Self> {
        let mut matches = Vec::new();
        matches.try_reserve(self.matches.len()).map_err(|e| {
            crate::error::Error::OutOfMemory {
                message: e.to_string(),
            }
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

    /// Count matches for each pattern ID, returning an error on OOM.
    pub fn try_pattern_counts(&self) -> crate::error::Result<std::collections::HashMap<u32, usize>> {
        let mut counts = std::collections::HashMap::new();
        counts.try_reserve(self.matches.len()).map_err(|e| {
            crate::error::Error::OutOfMemory {
                message: e.to_string(),
            }
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

    /// Distinct pattern IDs in the set, returning an error on OOM.
    pub fn try_pattern_ids(&self) -> crate::error::Result<Vec<u32>> {
        let mut ids = Vec::new();
        ids.try_reserve(self.matches.len()).map_err(|e| {
            crate::error::Error::OutOfMemory {
                message: e.to_string(),
            }
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
