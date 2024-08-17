use crate::func::args::ArgError;
use crate::func::Return;
use alloc::borrow::Cow;
use thiserror::Error;

/// An error that occurs when calling a [`DynamicClosure`] or [`DynamicClosureMut`].
///
/// [`DynamicClosure`]: crate::func::DynamicClosure
/// [`DynamicClosureMut`]: crate::func::DynamicClosureMut
#[derive(Debug, Error, PartialEq)]
pub enum FunctionError {
    /// An error occurred while converting an argument.
    #[error(transparent)]
    ArgError(#[from] ArgError),
    /// The number of arguments provided does not match the expected number.
    #[error("expected {expected} arguments but received {received}")]
    ArgCountMismatch { expected: usize, received: usize },
}

/// The result of calling a dynamic [`DynamicClosure`] or [`DynamicClosureMut`].
///
/// Returns `Ok(value)` if the function was called successfully,
/// where `value` is the [`Return`] value of the function.
///
/// [`DynamicClosure`]: crate::func::DynamicClosure
/// [`DynamicClosureMut`]: crate::func::DynamicClosureMut
pub type FunctionResult<'a> = Result<Return<'a>, FunctionError>;

/// An error that occurs when registering a function into a [`FunctionRegistry`].
///
/// [`FunctionRegistry`]: crate::func::FunctionRegistry
#[derive(Debug, Error, PartialEq)]
pub enum FunctionRegistrationError {
    /// A function with the given name has already been registered.
    ///
    /// Contains the duplicate function name.
    #[error("a function has already been registered with name {0:?}")]
    DuplicateName(Cow<'static, str>),
    /// The function is missing a name by which it can be registered.
    #[error("function name is missing")]
    MissingName,
}
