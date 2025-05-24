use alloc::borrow::Cow;

use thiserror::Error;

use crate::func::args::Ownership;

/// An error that occurs when converting an [argument].
///
/// [argument]: crate::func::args::Arg
#[derive(Debug, Error, PartialEq)]
pub enum ArgError {
    /// The argument is not the expected type.
    #[error("expected `{expected}` but received `{received}` (@ argument index {index})")]
    UnexpectedType {
        index: usize,
        expected: Cow<'static, str>,
        received: Cow<'static, str>,
    },
    /// The argument has the wrong ownership.
    #[error("expected {expected} value but received {received} value (@ argument index {index})")]
    InvalidOwnership {
        index: usize,
        expected: Ownership,
        received: Ownership,
    },
    /// Occurs when attempting to access an argument from an empty [`ArgList`].
    ///
    /// [`ArgList`]: crate::func::args::ArgList
    #[error("expected an argument but received none")]
    EmptyArgList,
}

/// The given argument count is out of bounds.
#[derive(Debug, Error, PartialEq)]
#[error("argument count out of bounds: {0}")]
pub struct ArgCountOutOfBoundsError(pub usize);
