use crate::func::{args::ArgError, Return};
use alloc::borrow::Cow;
use derive_more::derive::{Display, Error, From};

/// An error that occurs when calling a [`DynamicFunction`] or [`DynamicFunctionMut`].
///
/// [`DynamicFunction`]: crate::func::DynamicFunction
/// [`DynamicFunctionMut`]: crate::func::DynamicFunctionMut
#[derive(Debug, Error, Display, PartialEq, From)]
pub enum FunctionError {
    /// An error occurred while converting an argument.
    ArgError(ArgError),
    /// The number of arguments provided does not match the expected number.
    #[display("expected {expected} arguments but received {received}")]
    ArgCountMismatch { expected: usize, received: usize },
}

/// The result of calling a [`DynamicFunction`] or [`DynamicFunctionMut`].
///
/// Returns `Ok(value)` if the function was called successfully,
/// where `value` is the [`Return`] value of the function.
///
/// [`DynamicFunction`]: crate::func::DynamicFunction
/// [`DynamicFunctionMut`]: crate::func::DynamicFunctionMut
pub type FunctionResult<'a> = Result<Return<'a>, FunctionError>;

/// An error that occurs when registering a function into a [`FunctionRegistry`].
///
/// [`FunctionRegistry`]: crate::func::FunctionRegistry
#[derive(Debug, Error, Display, PartialEq)]
pub enum FunctionRegistrationError {
    /// A function with the given name has already been registered.
    ///
    /// Contains the duplicate function name.
    #[display("a function has already been registered with name {_0:?}")]
    #[error(ignore)]
    DuplicateName(Cow<'static, str>),
    /// The function is missing a name by which it can be registered.
    #[display("function name is missing")]
    MissingName,
}
