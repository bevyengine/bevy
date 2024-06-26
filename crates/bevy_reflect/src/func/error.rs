use crate::func::args::ArgError;
use thiserror::Error;

/// An error that occurs when calling a [`DynamicFunction`].
///
/// [`DynamicFunction`]: crate::func::DynamicFunction
#[derive(Debug, Error, PartialEq)]
pub enum FunctionError {
    /// An error occurred while converting an argument.
    #[error(transparent)]
    Arg(#[from] ArgError),
    /// The number of arguments provided does not match the expected number.
    #[error("expected {expected} arguments but received {received}")]
    ArgCount { expected: usize, received: usize },
}
