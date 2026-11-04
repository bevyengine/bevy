use std::fmt::Display;

use memchr::Memchr;
use nom::{Compare, Input, Offset};

use crate::lex::{LexError, Token};

/// A reference to a range of characters within a source string.
///
/// See also: [`OwnedSpan`]
#[derive(Clone, Debug)]
pub struct Span<'a> {
    /// The fragment of the input that this span refers to.
    pub fragment: &'a str,
    /// The position of the fragment relative to the input of the parser. Starts at 0.
    pub offset: usize,
    /// The line number of the fragment relative to the input of the parser. Starts at 1.
    pub line: u32,
}

/// An owned reference to a range of characters within a source string.
///
/// See also: [`Span`]
#[derive(Clone, Debug)]
pub struct OwnedSpan {
    /// The fragment of the input that this span refers to.
    pub fragment: String,
    /// The position of the fragment relative to the input of the parser. Starts at 0.
    pub offset: usize,
    /// The line number of the fragment relative to the input of the parser. Starts at 1.
    pub line: u32,
}

impl OwnedSpan {
    /// Borrows the owned span as an equivalent [`Span`]
    pub fn as_span(&self) -> Span<'_> {
        Span {
            fragment: &self.fragment,
            offset: self.offset,
            line: self.line,
        }
    }
}

impl<'a> From<Span<'a>> for OwnedSpan {
    fn from(value: Span<'a>) -> Self {
        Self {
            fragment: value.fragment.to_owned(),
            line: value.line,
            offset: value.offset,
        }
    }
}

impl<'a> Display for Span<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fragment.fmt(f)
    }
}

impl<'a> From<&'a str> for Span<'a> {
    fn from(value: &'a str) -> Self {
        Self {
            fragment: value,
            offset: 0,
            line: 0,
        }
    }
}

impl<'a> Span<'a> {
    // TODO: Improve the formatting of this
    /// Formats a message string that references this current span.
    pub fn message(&self, message: &str) -> String {
        format!("{message}. Line {}: {}", self.line, self.fragment)
    }

    pub(crate) fn slice_by(&self, next_fragment: &'a str) -> Self {
        let consumed_len = self.fragment.offset(next_fragment);
        if consumed_len == 0 {
            return Self {
                line: self.line,
                offset: self.offset,
                fragment: next_fragment,
            };
        }

        let consumed = self.fragment.take(consumed_len);

        let next_offset = self.offset + consumed_len;

        // PERF: we might want to skip this in release builds
        let consumed_as_bytes = consumed.as_bytes();
        let iter = Memchr::new(b'\n', consumed_as_bytes);
        let number_of_lines = iter.count() as u32;
        let next_line = self.line + number_of_lines;

        Self {
            line: next_line,
            offset: next_offset,
            fragment: next_fragment,
        }
    }

    /// Lex all tokens in this [`Span`]
    pub fn lex_all(self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        for token in self {
            tokens.push(token?.token);
        }
        Ok(tokens)
    }
}

impl<'a> core::ops::Deref for Span<'a> {
    type Target = &'a str;
    fn deref(&self) -> &Self::Target {
        &self.fragment
    }
}

impl<'a> Compare<&'a str> for Span<'a> {
    fn compare(&self, t: &'a str) -> nom::CompareResult {
        self.fragment.compare(t)
    }

    fn compare_no_case(&self, t: &'a str) -> nom::CompareResult {
        self.fragment.compare_no_case(t)
    }
}

impl<'a> Offset for Span<'a> {
    fn offset(&self, second: &Self) -> usize {
        second.offset - self.offset
    }
}

impl<'a> Input for Span<'a> {
    type Item = char;
    type Iter = <&'a str as Input>::Iter;
    type IterIndices = <&'a str as Input>::IterIndices;

    #[inline]
    fn input_len(&self) -> usize {
        self.fragment.input_len()
    }

    #[inline]
    fn take(&self, index: usize) -> Self {
        self.slice_by(self.fragment.take(index))
    }

    #[inline]
    fn take_from(&self, index: usize) -> Self {
        self.slice_by(self.fragment.take_from(index))
    }

    #[inline]
    fn take_split(&self, index: usize) -> (Self, Self) {
        (self.take_from(index), self.take(index))
    }

    #[inline]
    fn position<P>(&self, predicate: P) -> Option<usize>
    where
        P: Fn(Self::Item) -> bool,
    {
        self.fragment.position(predicate)
    }

    #[inline]
    fn iter_elements(&self) -> Self::Iter {
        self.fragment.iter_elements()
    }

    #[inline]
    fn iter_indices(&self) -> Self::IterIndices {
        self.fragment.iter_indices()
    }

    #[inline]
    fn slice_index(&self, count: usize) -> Result<usize, nom::Needed> {
        self.fragment.slice_index(count)
    }
}
