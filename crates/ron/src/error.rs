use serde::{de, ser};
use std::{error::Error as StdError, fmt, io, str::Utf8Error, string::FromUtf8Error};

pub use crate::parse::Position;

/// This type represents all possible errors that can occur when
/// serializing or deserializing RON data.
#[derive(Clone, Debug, PartialEq)]
pub struct Error {
    pub code: ErrorCode,
    pub position: Position,
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq)]
pub enum ErrorCode {
    Io(String),
    Message(String),
    Base64Error(base64::DecodeError),
    Eof,
    ExpectedArray,
    ExpectedArrayEnd,
    ExpectedAttribute,
    ExpectedAttributeEnd,
    ExpectedBoolean,
    ExpectedComma,
    // ExpectedEnum,
    ExpectedChar,
    ExpectedFloat,
    ExpectedInteger,
    ExpectedOption,
    ExpectedOptionEnd,
    ExpectedMap,
    ExpectedMapColon,
    ExpectedMapEnd,
    ExpectedStruct,
    ExpectedStructEnd,
    ExpectedUnit,
    // ExpectedStructName,
    ExpectedString,
    ExpectedStringEnd,
    ExpectedIdentifier,

    InvalidEscape(&'static str),

    IntegerOutOfBounds,

    NoSuchExtension(String),

    UnclosedBlockComment,
    UnderscoreAtBeginning,
    UnexpectedByte(char),

    Utf8Error(Utf8Error),
    TrailingCharacters,

    #[doc(hidden)]
    __Nonexhaustive,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if (self.position == Position { line: 0, col: 0 }) {
            write!(f, "{}", self.code)
        } else {
            write!(f, "{}: {}", self.position, self.code)
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ErrorCode::Io(ref s) => f.write_str(s),
            ErrorCode::Message(ref s) => f.write_str(s),
            ErrorCode::Base64Error(ref e) => fmt::Display::fmt(e, f),
            ErrorCode::Eof => f.write_str("Unexpected end of file"),
            ErrorCode::ExpectedArray => f.write_str("Expected array"),
            ErrorCode::ExpectedArrayEnd => f.write_str("Expected end of array"),
            ErrorCode::ExpectedAttribute => f.write_str("Expected an enable attribute"),
            ErrorCode::ExpectedAttributeEnd => {
                f.write_str("Expected closing `)` and `]` after the attribute")
            }
            ErrorCode::ExpectedBoolean => f.write_str("Expected boolean"),
            ErrorCode::ExpectedComma => f.write_str("Expected comma"),
            // ErrorCode::ExpectedEnum => f.write_str("Expected enum"),
            ErrorCode::ExpectedChar => f.write_str("Expected char"),
            ErrorCode::ExpectedFloat => f.write_str("Expected float"),
            ErrorCode::ExpectedInteger => f.write_str("Expected integer"),
            ErrorCode::ExpectedOption => f.write_str("Expected option"),
            ErrorCode::ExpectedOptionEnd => f.write_str("Expected end of option"),
            ErrorCode::ExpectedMap => f.write_str("Expected map"),
            ErrorCode::ExpectedMapColon => f.write_str("Expected colon"),
            ErrorCode::ExpectedMapEnd => f.write_str("Expected end of map"),
            ErrorCode::ExpectedStruct => f.write_str("Expected struct"),
            ErrorCode::ExpectedStructEnd => f.write_str("Expected end of struct"),
            ErrorCode::ExpectedUnit => f.write_str("Expected unit"),
            // ErrorCode::ExpectedStructName => f.write_str("Expected struct name"),
            ErrorCode::ExpectedString => f.write_str("Expected string"),
            ErrorCode::ExpectedStringEnd => f.write_str("Expected string end"),
            ErrorCode::ExpectedIdentifier => f.write_str("Expected identifier"),
            ErrorCode::InvalidEscape(_) => f.write_str("Invalid escape sequence"),
            ErrorCode::IntegerOutOfBounds => f.write_str("Integer is out of bounds"),
            ErrorCode::NoSuchExtension(_) => f.write_str("No such RON extension"),
            ErrorCode::Utf8Error(ref e) => fmt::Display::fmt(e, f),
            ErrorCode::UnclosedBlockComment => f.write_str("Unclosed block comment"),
            ErrorCode::UnderscoreAtBeginning => f.write_str("Found underscore at the beginning"),
            ErrorCode::UnexpectedByte(_) => f.write_str("Unexpected byte"),
            ErrorCode::TrailingCharacters => f.write_str("Non-whitespace trailing characters"),
            _ => f.write_str("Unknown ErrorCode"),
        }
    }
}

impl de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error {
            code: ErrorCode::Message(msg.to_string()),
            position: Position { line: 0, col: 0 },
        }
    }
}

impl ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error {
            code: ErrorCode::Message(msg.to_string()),
            position: Position { line: 0, col: 0 },
        }
    }
}

impl StdError for Error {}

impl From<Utf8Error> for ErrorCode {
    fn from(e: Utf8Error) -> Self {
        ErrorCode::Utf8Error(e)
    }
}

impl From<FromUtf8Error> for ErrorCode {
    fn from(e: FromUtf8Error) -> Self {
        ErrorCode::Utf8Error(e.utf8_error())
    }
}

impl From<Utf8Error> for Error {
    fn from(e: Utf8Error) -> Self {
        Error {
            code: ErrorCode::Utf8Error(e),
            position: Position { line: 0, col: 0 },
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error {
            code: ErrorCode::Io(e.to_string()),
            position: Position { line: 0, col: 0 },
        }
    }
}
