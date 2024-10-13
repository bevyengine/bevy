use alloc::borrow::Cow;

use derive_more::derive::{Display, Error};

use crate::func::args::Ownership;

/// An error that occurs when converting an [argument].
///
/// [argument]: crate::func::args::Arg
#[derive(Debug, Error, Display, PartialEq)]
pub enum ArgError {
    /// The argument is not the expected type.
    #[display("expected `{expected}` but received `{received}` (@ argument index {index})")]
    UnexpectedType {
        index: usize,
        expected: Cow<'static, str>,
        received: Cow<'static, str>,
    },
    /// The argument has the wrong ownership.
    #[display(
        "expected {expected} value but received {received} value (@ argument index {index})"
    )]
    InvalidOwnership {
        index: usize,
        expected: Ownership,
        received: Ownership,
    },
    /// Occurs when attempting to access an argument from an empty [`ArgList`].
    ///
    /// [`ArgList`]: crate::func::args::ArgList
    #[display("expected an argument but received none")]
    EmptyArgList,
}
