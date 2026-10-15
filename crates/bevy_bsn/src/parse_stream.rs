use crate::{
    lex::{LexError, SpannedToken, Token},
    span::{OwnedSpan, Span},
    types::{LBrace, LBracket, LParen, RBrace, RBracket, RParen},
};
use alloc::collections::VecDeque;
use core::fmt::Display;
use thiserror::Error;

/// An error that occurs while parsing BSN.
#[derive(Error)]
pub enum ParseError {
    /// A lex error.
    #[error(transparent)]
    LexError(LexError),
    /// Occurs when an unexpected token is encountered.
    #[error("{}", .0.span.as_span().message(&format!("Unexpected Token {:?}", .0.token)))]
    UnexpectedToken(OwnedSpannedToken),
    /// Occurs when the end of input was reached unexpectedly
    #[error("Unexpected end of input.")]
    EndOfInput,
    /// An error that occurred for a given spanned token.
    #[error("{}", .token.span.as_span().message(.message))]
    SpannedMessage {
        /// The message. This should provide helpful user-facing context for the error.
        message: String,
        /// The spanned token that produced the error.
        token: OwnedSpannedToken,
    },
    /// A span-less error occurred. Prefer [`ParseError::SpannedMessage`] when possible.
    #[error("{0}")]
    Message(String),
}

/// A [`Token`] coupled with the [`Span`] that produced it.
#[derive(Debug, Clone)]
pub struct OwnedSpannedToken {
    /// A [`Token`].
    pub token: Token,
    /// The [`Span`] that produced the [`Token`].
    pub span: OwnedSpan,
}

impl<'a> From<SpannedToken<'a>> for OwnedSpannedToken {
    fn from(value: SpannedToken<'a>) -> Self {
        Self {
            token: value.token,
            span: value.span.into(),
        }
    }
}

impl std::fmt::Debug for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}
impl From<LexError> for ParseError {
    fn from(value: LexError) -> Self {
        Self::LexError(value)
    }
}

/// A peek-able stream of tokens to be parsed.
pub struct ParseStream<'a> {
    span: Span<'a>,
    peek_buffer: VecDeque<Result<SpannedToken<'a>, LexError>>,
}

impl<'a> From<Span<'a>> for ParseStream<'a> {
    fn from(value: Span<'a>) -> Self {
        Self {
            span: value,
            peek_buffer: VecDeque::new(),
        }
    }
}

impl<'a> ParseStream<'a> {
    /// Try to parse the given type `P`.
    pub fn parse<P: Parse>(&mut self) -> Result<P, ParseError> {
        P::parse(self)
    }

    /// Keeps the given type `P`. Returns true if it is present. Otherwise returns false.
    pub fn peek<P: Peek>(&mut self) -> bool {
        P::peek(self)
    }

    /// Returns true if the parse stream is empty (no more tokens).
    pub fn is_empty(&mut self) -> bool {
        self.peek_token().is_none()
    }

    /// Returns true if the parse stream is empty or [`Token::RBracket`], [`Token::RParen`], or [`Token::RBrace`]
    /// are present.
    pub fn is_empty_or_closing_delimiter(&mut self) -> bool {
        if let Some(token) = self.peek_token() {
            matches!(token.token, Token::RBracket | Token::RParen | Token::RBrace)
        } else {
            true
        }
    }

    /// Peeks the next token. Returns [`Some`] if the token is present. Otherwise returns [`None`].
    pub fn peek_token(&mut self) -> Option<&SpannedToken<'a>> {
        if !self.peek_buffer.is_empty() {
            self.peek_buffer.front().unwrap().as_ref().ok()
        } else {
            match self.span.next() {
                Some(result) => {
                    self.peek_buffer.push_back(result);
                    self.peek_buffer.back().unwrap().as_ref().ok()
                }
                None => None,
            }
        }
    }

    /// Retrieves the next [`SpannedToken`] in the parse stream, advancing the position in the stream forward.
    pub fn next_token(&mut self) -> Result<Option<SpannedToken<'a>>, LexError> {
        if let Some(value) = self.peek_buffer.pop_front() {
            return value.map(Some);
        }

        self.span.next().transpose()
    }

    /// Returns a [`ParseError`] with the given `message`. If there is a next token in the stream,
    /// its [`Span`] will be used.
    pub fn error(&mut self, message: impl Into<String>) -> ParseError {
        match self.peek_token() {
            Some(token) => ParseError::SpannedMessage {
                message: message.into(),
                token: token.clone().into(),
            },
            None => ParseError::Message(message.into()),
        }
    }

    /// Parses a type `P` contained within brackets.
    pub fn bracketed<P: Parse>(&mut self) -> Result<P, ParseError> {
        self.parse::<LBracket>()?;
        let p = self.parse::<P>()?;
        self.parse::<RBracket>()?;
        Ok(p)
    }

    /// Parses a type `P` contained within parentheses.
    pub fn parenthesized<P: Parse>(&mut self) -> Result<P, ParseError> {
        self.parse::<LParen>()?;
        let p = self.parse::<P>()?;
        self.parse::<RParen>()?;
        Ok(p)
    }

    /// Parses a type `P` contained within braces.
    pub fn braced<P: Parse>(&mut self) -> Result<P, ParseError> {
        self.parse::<LBrace>()?;
        let p = self.parse::<P>()?;
        self.parse::<RBrace>()?;
        Ok(p)
    }
}

/// A parse-able type.
pub trait Parse: Sized {
    /// Parses the given type using the next tokens in the `input` [`ParseStream`].
    fn parse<'a>(input: &mut ParseStream<'a>) -> Result<Self, ParseError>;
}

/// A peek-able type.
pub trait Peek {
    /// Peeks the given type using the next tokens in the `input` [`ParseStream`].
    fn peek(input: &mut ParseStream) -> bool;
}

#[cfg(test)]
mod tests {
    use crate::{
        parse_stream::{ParseError, ParseStream},
        span::Span,
        types::At,
    };

    #[test]
    fn parse_basics() {
        let mut input = ParseStream::from(Span::from("@"));
        assert!(input.peek::<At>());
        assert!(!input.is_empty());
        assert!(input.peek::<At>());
        let _ = input.parse::<At>().unwrap();
        assert!(matches!(input.parse::<At>(), Err(ParseError::EndOfInput)));
        assert!(input.is_empty());
    }
}
