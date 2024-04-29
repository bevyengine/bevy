use crate::func::args::ArgError;
use thiserror::Error;

/// An error that occurs when calling a dynamic [`Function`].
///
/// [`Function`]: crate::func::Function
#[derive(Debug, Error, PartialEq)]
pub enum FuncError {
    /// An error occurred while converting an argument.
    #[error(transparent)]
    Arg(#[from] ArgError),
    /// The number of arguments provided does not match the expected number.
    #[error("expected {expected} arguments but received {received}")]
    ArgCount { expected: usize, received: usize },
}
