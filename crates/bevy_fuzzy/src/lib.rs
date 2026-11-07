#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]
#![no_std]

//! A small fuzzy matcher for tools such as inspectors, pickers and command palettes.
//!
//! A [`FuzzyPattern`] is split on whitespace into atoms, and a haystack matches when every
//! atom matches it as a case-insensitive subsequence. Matches are scored so that consecutive
//! characters, word starts and camelCase humps rank higher than scattered characters.
//!
//! ```
//! use bevy_fuzzy::{segments, FuzzyPattern};
//!
//! let names = ["Sampler", "PointLight", "Transform"];
//! let pattern = FuzzyPattern::new("pl");
//! let ranked = pattern.rank(&names, |name| *name);
//!
//! assert_eq!(ranked.len(), 2);
//! assert_eq!(names[ranked[0].index], "PointLight");
//! assert_eq!(names[ranked[1].index], "Sampler");
//!
//! let highlighted: Vec<_> = segments(names[ranked[1].index], &ranked[1].matched.indices).collect();
//! assert_eq!(highlighted, [("Sam", false), ("pl", true), ("er", false)]);
//! ```

extern crate alloc;

use alloc::{vec, vec::Vec};
use core::cmp::Reverse;

/// The maximum number of haystack chars considered when matching.
///
/// Chars past this limit are ignored, so a pattern that only matches beyond it does not match.
pub const MAX_HAYSTACK_CHARS: usize = 1024;

const SCORE_MATCH: i32 = 16;
const GAP_START: i32 = 6;
const GAP_EXTEND: i32 = 2;
const GAP_MAX: i32 = 14;
const BONUS_BOUNDARY: i32 = 8;
const BONUS_CAMEL: i32 = 7;
const BONUS_CONSECUTIVE: i32 = 6;
const FIRST_CHAR_MULTIPLIER: i32 = 2;

const UNREACHABLE: i32 = i32::MIN / 2;
const NO_PREDECESSOR: u32 = u32::MAX;

/// A parsed fuzzy search pattern.
///
/// Whitespace splits the pattern into atoms, and every atom must match. Matching ignores case
/// by comparing the first char of each char's [`char::to_lowercase`] mapping, with Greek final
/// sigma treated as sigma. An empty or whitespace-only pattern matches everything with a score
/// of 0.
///
/// Each char folds to exactly one char, so that matched indices stay valid for the haystack.
/// Case foldings that change the length do not apply: the German sharp s does not match `ss`,
/// and a ligature does not match its separate letters.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FuzzyPattern {
    atoms: Vec<Vec<char>>,
}

/// A successful match of a [`FuzzyPattern`] against a haystack.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FuzzyMatch {
    /// The match quality. Higher is better.
    pub score: u32,
    /// The sorted, de-duplicated char indices of the matched haystack chars.
    pub indices: Vec<usize>,
}

/// An item that matched in [`FuzzyPattern::rank`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RankedMatch {
    /// The index of the item in the ranked slice.
    pub index: usize,
    /// The match for the item's key.
    pub matched: FuzzyMatch,
}

impl FuzzyPattern {
    /// Parses a pattern, splitting it into atoms on whitespace.
    pub fn new(pattern: &str) -> Self {
        Self {
            atoms: pattern
                .split_whitespace()
                .map(|atom| atom.chars().map(fold).collect())
                .collect(),
        }
    }

    /// Returns `true` if the pattern has no atoms and so matches everything.
    pub fn is_empty(&self) -> bool {
        self.atoms.is_empty()
    }

    /// Returns the score of the best match in `haystack`, or `None` if it does not match.
    pub fn score(&self, haystack: &str) -> Option<u32> {
        self.run(haystack, None)
    }

    /// Matches `haystack`, returning the score and the matched char indices.
    pub fn match_str(&self, haystack: &str) -> Option<FuzzyMatch> {
        let mut indices = Vec::new();
        let score = self.run(haystack, Some(&mut indices))?;
        indices.sort_unstable();
        indices.dedup();
        Some(FuzzyMatch { score, indices })
    }

    /// Matches every item's key and returns the matching items.
    ///
    /// Results are sorted by score descending, then by shorter key, then by original order. An
    /// empty pattern matches every item and keeps the original order.
    pub fn rank<T>(&self, items: &[T], key: impl Fn(&T) -> &str) -> Vec<RankedMatch> {
        if self.is_empty() {
            return (0..items.len())
                .map(|index| RankedMatch {
                    index,
                    matched: FuzzyMatch::default(),
                })
                .collect();
        }
        let mut ranked: Vec<(usize, RankedMatch)> = items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let haystack = key(item);
                let matched = self.match_str(haystack)?;
                Some((haystack.chars().count(), RankedMatch { index, matched }))
            })
            .collect();
        ranked.sort_by_key(|(len, ranked)| (Reverse(ranked.matched.score), *len, ranked.index));
        ranked.into_iter().map(|(_, ranked)| ranked).collect()
    }

    fn run(&self, haystack: &str, mut indices: Option<&mut Vec<usize>>) -> Option<u32> {
        if self.atoms.is_empty() {
            return Some(0);
        }
        let haystack = Haystack::new(haystack);
        let mut total = 0u32;
        for atom in &self.atoms {
            total = total.saturating_add(haystack.align(atom, indices.as_deref_mut())?);
        }
        Some(total)
    }
}

/// Splits `haystack` into consecutive slices, each paired with whether its chars are matched.
///
/// `indices` are sorted char indices, as in [`FuzzyMatch::indices`]. Concatenating the slices
/// gives back `haystack`.
pub fn segments<'a>(haystack: &'a str, indices: &'a [usize]) -> Segments<'a> {
    Segments {
        rest: haystack,
        char_index: 0,
        indices,
    }
}

/// The iterator returned by [`segments`].
#[derive(Clone, Debug)]
pub struct Segments<'a> {
    rest: &'a str,
    char_index: usize,
    indices: &'a [usize],
}

impl Segments<'_> {
    fn is_matched(&mut self, char_index: usize) -> bool {
        while let Some((&first, rest)) = self.indices.split_first() {
            if first >= char_index {
                return first == char_index;
            }
            self.indices = rest;
        }
        false
    }
}

impl<'a> Iterator for Segments<'a> {
    type Item = (&'a str, bool);

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }
        let matched = self.is_matched(self.char_index);
        let mut end = self.rest.len();
        for (byte, _) in self.rest.char_indices() {
            if self.is_matched(self.char_index) != matched {
                end = byte;
                break;
            }
            self.char_index += 1;
        }
        let (segment, rest) = self.rest.split_at(end);
        self.rest = rest;
        Some((segment, matched))
    }
}

struct Haystack {
    chars: Vec<char>,
    bonus: Vec<i32>,
}

impl Haystack {
    fn new(haystack: &str) -> Self {
        let mut chars = Vec::new();
        let mut bonus = Vec::new();
        let mut prev = None;
        let mut rest = haystack.chars().take(MAX_HAYSTACK_CHARS).peekable();
        while let Some(c) = rest.next() {
            chars.push(fold(c));
            bonus.push(char_bonus(prev, c, rest.peek().copied()));
            prev = Some(c);
        }
        Self { chars, bonus }
    }

    /// Finds the best alignment of `atom` and pushes its char indices to `indices`, if given.
    ///
    /// Each cell holds the best score with atom char `i` matched at haystack char `j`, either
    /// directly after a match at `j - 1` or after a gap.
    fn align(&self, atom: &[char], indices: Option<&mut Vec<usize>>) -> Option<u32> {
        let n = self.chars.len();
        let mut remaining = atom.iter();
        let mut next = remaining.next();
        for c in &self.chars {
            if next == Some(c) {
                next = remaining.next();
            }
        }
        if next.is_some() {
            return None;
        }

        let tracing = indices.is_some();
        let mut trace = Vec::new();
        if tracing {
            trace.reserve(atom.len() * n);
        }
        let mut prev = vec![UNREACHABLE; n];
        let mut prev_run = vec![0; n];
        let mut cur = vec![UNREACHABLE; n];
        let mut cur_run = vec![0; n];

        for (i, &c) in atom.iter().enumerate() {
            let mut gap_best = UNREACHABLE;
            let mut gap_from = NO_PREDECESSOR;
            let mut max_prev = UNREACHABLE;
            let mut max_from = NO_PREDECESSOR;
            for j in 0..n {
                if i > 0 && j >= 2 {
                    if gap_best > UNREACHABLE {
                        gap_best -= GAP_EXTEND;
                    }
                    let p = prev[j - 2];
                    if p > UNREACHABLE {
                        if p - GAP_START > gap_best {
                            gap_best = p - GAP_START;
                            gap_from = (j - 2) as u32;
                        }
                        if p > max_prev {
                            max_prev = p;
                            max_from = (j - 2) as u32;
                        }
                    }
                }

                let mut best = UNREACHABLE;
                let mut from = NO_PREDECESSOR;
                let mut run = 0;
                if self.chars[j] == c {
                    let bonus = self.bonus[j];
                    if i == 0 {
                        best = SCORE_MATCH + bonus * FIRST_CHAR_MULTIPLIER;
                        run = bonus;
                    } else {
                        if j > 0 && prev[j - 1] > UNREACHABLE {
                            let chunk = prev_run[j - 1].max(bonus);
                            best = prev[j - 1] + SCORE_MATCH + chunk.max(BONUS_CONSECUTIVE);
                            from = (j - 1) as u32;
                            run = chunk;
                        }
                        let (gap, gap_source) = if max_prev - GAP_MAX > gap_best {
                            (max_prev - GAP_MAX, max_from)
                        } else {
                            (gap_best, gap_from)
                        };
                        if gap > UNREACHABLE && gap + SCORE_MATCH + bonus > best {
                            best = gap + SCORE_MATCH + bonus;
                            from = gap_source;
                            run = bonus;
                        }
                    }
                }
                cur[j] = best;
                cur_run[j] = run;
                if tracing {
                    trace.push(from);
                }
            }
            core::mem::swap(&mut prev, &mut cur);
            core::mem::swap(&mut prev_run, &mut cur_run);
        }

        let mut end = 0;
        for j in 1..n {
            if prev[j] > prev[end] {
                end = j;
            }
        }
        if let Some(indices) = indices {
            let mut j = end;
            for i in (0..atom.len()).rev() {
                indices.push(j);
                j = trace[i * n + j] as usize;
            }
        }
        Some(u32::try_from(prev[end]).unwrap_or(0))
    }
}

fn fold(c: char) -> char {
    if c.is_ascii() {
        c.to_ascii_lowercase()
    } else {
        match c.to_lowercase().next().unwrap_or(c) {
            'ς' => 'σ',
            lower => lower,
        }
    }
}

fn is_separator(c: char) -> bool {
    c.is_whitespace() || matches!(c, '_' | '-' | '.' | '/' | ':')
}

fn char_bonus(prev: Option<char>, c: char, next: Option<char>) -> i32 {
    let Some(prev) = prev else {
        return BONUS_BOUNDARY;
    };
    if is_separator(c) {
        0
    } else if is_separator(prev) {
        BONUS_BOUNDARY
    } else if (prev.is_lowercase() && c.is_uppercase())
        || (prev.is_uppercase() && c.is_uppercase() && next.is_some_and(char::is_lowercase))
        || (prev.is_alphabetic() && c.is_numeric())
        || (prev.is_numeric() && c.is_alphabetic())
    {
        BONUS_CAMEL
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{string::String, vec::Vec};

    fn indices(pattern: &str, haystack: &str) -> Vec<usize> {
        FuzzyPattern::new(pattern)
            .match_str(haystack)
            .unwrap()
            .indices
    }

    fn score(pattern: &str, haystack: &str) -> u32 {
        FuzzyPattern::new(pattern).score(haystack).unwrap()
    }

    #[test]
    fn empty_pattern_matches_everything() {
        for pattern in ["", "   ", "\t\n"] {
            let pattern = FuzzyPattern::new(pattern);
            assert!(pattern.is_empty());
            assert_eq!(pattern.score("anything"), Some(0));
            assert_eq!(pattern.score(""), Some(0));
            assert_eq!(pattern.match_str("abc"), Some(FuzzyMatch::default()));
        }
    }

    #[test]
    fn ignores_case() {
        assert_eq!(score("POINT", "pointlight"), score("point", "PointLight"));
        assert_eq!(indices("GRÖ", "größe"), [0, 1, 2]);
    }

    #[test]
    fn final_sigma_matches_sigma() {
        assert_eq!(indices("ΟΣ", "ος"), [0, 1]);
        assert_eq!(indices("ς", "σ"), [0]);
        assert_eq!(indices("σ", "ς"), [0]);
    }

    #[test]
    fn folding_keeps_length() {
        assert_eq!(indices("ß", "ẞ"), [0]);
        assert_eq!(FuzzyPattern::new("ss").score("Straße"), None);
        assert_eq!(FuzzyPattern::new("ß").score("STRASSE"), None);
        assert_eq!(FuzzyPattern::new("fi").score("ﬁ"), None);
    }

    #[test]
    fn all_atoms_must_match() {
        let pattern = FuzzyPattern::new("point  light");
        assert!(pattern.score("PointLight").is_some());
        assert!(pattern.score("Point").is_none());
        assert_eq!(
            pattern.match_str("PointLight").unwrap().indices,
            (0..10).collect::<Vec<_>>()
        );
        assert_eq!(indices("ab ab", "ab"), [0, 1]);
    }

    #[test]
    fn non_match() {
        let pattern = FuzzyPattern::new("xyz");
        assert_eq!(pattern.score("Transform"), None);
        assert_eq!(pattern.match_str(""), None);
        assert_eq!(FuzzyPattern::new("ba").score("ab"), None);
    }

    #[test]
    fn prefers_word_starts() {
        assert!(score("pl", "PointLight") > score("pl", "Spline"));
        assert!(score("tr", "Transform") > score("tr", "Matrix"));
        assert_eq!(indices("plt", "PointLight"), [0, 5, 9]);
        assert_eq!(indices("gt", "global_transform"), [0, 7]);
        assert_eq!(indices("v3", "vec_v3"), [4, 5]);
        assert!(score("3d", "Camera3d") > score("3d", "Camera3xd"));
    }

    #[test]
    fn uppercase_run_ends_at_next_word() {
        assert_eq!(score("cm", "ClusterFarZMode"), score("cm", "ClusterFarZeeMode"));
        let items = ["ClusterFarZeeMode", "ClusterFarZMode"];
        let ranked = FuzzyPattern::new("cm").rank(&items, |item| *item);
        assert_eq!(ranked[0].index, 1);
        assert_eq!(indices("zm", "ClusterFarZMode"), [10, 11]);
        assert!(score("p", "XMLParser") > score("p", "Xp"));
        assert_eq!(score("m", "XM"), score("m", "Xm"));
    }

    #[test]
    fn prefers_consecutive() {
        assert!(score("abc", "xabcx") > score("abc", "xaxbxcx"));
        assert_eq!(indices("light", "xlxightlight"), [7, 8, 9, 10, 11]);
    }

    #[test]
    fn exact_substring_beats_scattered() {
        assert!(score("light", "Highlight") > score("light", "LongItemGetHeightTotal"));
        assert!(score("form", "Transform") > score("form", "FooOrRandomMap"));
    }

    #[test]
    fn shorter_haystack_breaks_ties() {
        let items = ["Transform2", "Transform", "Transforms"];
        assert_eq!(score("trans", items[0]), score("trans", items[1]));
        let ranked = FuzzyPattern::new("trans").rank(&items, |item| *item);
        let order: Vec<_> = ranked.iter().map(|r| r.index).collect();
        assert_eq!(order, [1, 0, 2]);
    }

    #[test]
    fn indices_are_char_indices() {
        assert_eq!(indices("ße", "Größe"), [3, 4]);
        assert_eq!(indices("öe", "Größe"), [2, 4]);
        assert_eq!(indices("b", "日本b"), [2]);
        assert_eq!(indices("本", "日本b"), [1]);
    }

    #[test]
    fn segments_reconstruct_haystack() {
        for (pattern, haystack) in [
            ("ße", "Größe"),
            ("gö", "Größe"),
            ("本b", "日本語bc"),
            ("pl", "PointLight"),
            ("", "empty"),
        ] {
            let matched = FuzzyPattern::new(pattern).match_str(haystack).unwrap();
            let parts: Vec<_> = segments(haystack, &matched.indices).collect();
            let joined: String = parts.iter().map(|(s, _)| *s).collect();
            assert_eq!(joined, haystack);
            assert!(parts.iter().all(|(s, _)| !s.is_empty()));
            assert!(parts.windows(2).all(|w| w[0].1 != w[1].1));
            let matched_chars: String = parts.iter().filter(|(_, m)| *m).map(|(s, _)| *s).collect();
            assert_eq!(matched_chars.chars().count(), matched.indices.len());
        }
        let parts: Vec<_> = segments("Größe", &[3, 4]).collect();
        assert_eq!(parts, [("Grö", false), ("ße", true)]);
        assert_eq!(segments("", &[0]).count(), 0);
    }

    #[test]
    fn haystack_is_capped() {
        let mut haystack: String = "a".repeat(MAX_HAYSTACK_CHARS - 1);
        haystack.push('z');
        assert!(FuzzyPattern::new("z").score(&haystack).is_some());
        haystack.insert(0, 'a');
        assert!(FuzzyPattern::new("z").score(&haystack).is_none());
    }

    #[test]
    fn rank_is_stable() {
        let items = ["abc", "xyz", "abc", "abx", "abc"];
        let ranked = FuzzyPattern::new("ab").rank(&items, |item| *item);
        let order: Vec<_> = ranked.iter().map(|r| r.index).collect();
        assert_eq!(order, [0, 2, 3, 4]);
        let all = FuzzyPattern::new("").rank(&items, |item| *item);
        let order: Vec<_> = all.iter().map(|r| r.index).collect();
        assert_eq!(order, [0, 1, 2, 3, 4]);
    }

    #[test]
    fn empty_pattern_keeps_order() {
        let items = ["Transform", "Name", "PointLight"];
        for pattern in ["", "  "] {
            let ranked = FuzzyPattern::new(pattern).rank(&items, |item| *item);
            let order: Vec<_> = ranked.iter().map(|r| r.index).collect();
            assert_eq!(order, [0, 1, 2]);
        }
    }

    #[test]
    fn deterministic() {
        let pattern = FuzzyPattern::new("ab");
        let first = pattern.match_str("abab_ab");
        for _ in 0..4 {
            assert_eq!(pattern.match_str("abab_ab"), first);
        }
        assert_eq!(first.as_ref().map(|m| m.score), pattern.score("abab_ab"));
    }
}
