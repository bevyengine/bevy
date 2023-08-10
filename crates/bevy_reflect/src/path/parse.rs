use std::{fmt, num::ParseIntError};

use thiserror::Error;

use super::{Access, ReflectPathError};

/// An error that occurs when parsing reflect path strings.
#[derive(Debug, PartialEq, Eq, Error)]
#[error(transparent)]
pub struct ParseError<'a>(Error<'a>);

/// A parse error for a path string.
#[derive(Debug, PartialEq, Eq, Error)]
enum Error<'a> {
    #[error("expected an identifier, but reached end of path string")]
    NoIdent,

    #[error("expected an identifier, got '{0}' instead")]
    ExpectedIdent(Token<'a>),

    #[error("failed to parse index as integer")]
    InvalidIndex(#[from] ParseIntError),

    #[error("a '[' wasn't closed, reached end of path string before finding a ']'")]
    Unclosed,

    #[error("a '[' wasn't closed properly, got '{0}' instead")]
    BadClose(Token<'a>),

    #[error("a ']' was found before an opening '['")]
    CloseBeforeOpen,
}

pub(super) struct PathParser<'a> {
    path: &'a str,
    offset: usize,
}
impl<'a> PathParser<'a> {
    pub(super) fn new(path: &'a str) -> Self {
        PathParser { path, offset: 0 }
    }

    fn next_token(&mut self) -> Option<Token<'a>> {
        let input = &self.path[self.offset..];

        // Return with `None` if empty.
        let first_char = input.chars().next()?;

        if let Some(token) = Token::symbol_from_char(first_char) {
            self.offset += 1; // NOTE: we assume all symbols are ASCII
            return Some(token);
        }
        // We are parsing either `0123` or `field`.
        // If we do not find a subsequent token, we are at the end of the parse string.
        let ident = input.split_once(Token::SYMBOLS).map_or(input, |t| t.0);

        self.offset += ident.len();
        Some(Token::Ident(Ident(ident)))
    }

    fn next_ident(&mut self) -> Result<Ident<'a>, Error<'a>> {
        match self.next_token() {
            Some(Token::Ident(ident)) => Ok(ident),
            Some(other) => Err(Error::ExpectedIdent(other)),
            None => Err(Error::NoIdent),
        }
    }

    fn access_following(&mut self, token: Token<'a>) -> Result<Access<'a>, Error<'a>> {
        match token {
            Token::Dot => Ok(self.next_ident()?.field()),
            Token::Pound => self.next_ident()?.field_index(),
            Token::Ident(ident) => Ok(ident.field()),
            Token::CloseBracket => Err(Error::CloseBeforeOpen),
            Token::OpenBracket => {
                let index_ident = self.next_ident()?.list_index()?;
                match self.next_token() {
                    Some(Token::CloseBracket) => Ok(index_ident),
                    Some(other) => Err(Error::BadClose(other)),
                    None => Err(Error::Unclosed),
                }
            }
        }
    }
}
impl<'a> Iterator for PathParser<'a> {
    type Item = (Result<Access<'a>, ReflectPathError<'a>>, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.next_token()?;
        let offset = self.offset;
        let err = |error| ReflectPathError::ParseError {
            offset,
            path: self.path,
            error: ParseError(error),
        };
        Some((self.access_following(token).map_err(err), offset))
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Ident<'a>(&'a str);

impl<'a> Ident<'a> {
    fn field(self) -> Access<'a> {
        let field = |_| Access::Field(self.0.into());
        self.0.parse().map(Access::TupleIndex).unwrap_or_else(field)
    }
    fn field_index(self) -> Result<Access<'a>, Error<'a>> {
        Ok(Access::FieldIndex(self.0.parse()?))
    }
    fn list_index(self) -> Result<Access<'a>, Error<'a>> {
        Ok(Access::ListIndex(self.0.parse()?))
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Token<'a> {
    Dot,
    Pound,
    OpenBracket,
    CloseBracket,
    Ident(Ident<'a>),
}
impl fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Dot => f.write_str("."),
            Token::Pound => f.write_str("#"),
            Token::OpenBracket => f.write_str("["),
            Token::CloseBracket => f.write_str("]"),
            Token::Ident(ident) => f.write_str(ident.0),
        }
    }
}
impl<'a> Token<'a> {
    const SYMBOLS: &[char] = &['.', '#', '[', ']'];
    fn symbol_from_char(char: char) -> Option<Self> {
        match char {
            '.' => Some(Self::Dot),
            '#' => Some(Self::Pound),
            '[' => Some(Self::OpenBracket),
            ']' => Some(Self::CloseBracket),
            _ => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::path::ParsedPath;

    #[test]
    fn parse_invalid() {
        assert_eq!(
            ParsedPath::parse_static("x.."),
            Err(ReflectPathError::ParseError {
                error: ParseError(Error::ExpectedIdent(Token::Dot)),
                offset: 2,
                path: "x..",
            }),
        );
        assert!(matches!(
            ParsedPath::parse_static("y[badindex]"),
            Err(ReflectPathError::ParseError {
                error: ParseError(Error::InvalidIndex(_)),
                offset: 2,
                path: "y[badindex]",
            }),
        ));
    }
}
