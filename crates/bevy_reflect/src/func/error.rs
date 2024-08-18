use crate::func::args::ArgError;
use crate::func::Return;
use alloc::borrow::Cow;
use thiserror::Error;

/// An error that occurs when calling a [`DynamicCallable`] or [`DynamicCallableMut`].
///
/// [`DynamicCallable`]: crate::func::DynamicCallable
/// [`DynamicCallableMut`]: crate::func::DynamicCallableMut
#[derive(Debug, Error, PartialEq)]
pub enum FunctionError {
    /// An error occurred while converting an argument.
    #[error(transparent)]
    ArgError(#[from] ArgError),
    /// The number of arguments provided does not match the expected number.
    #[error("expected {expected} arguments but received {received}")]
    ArgCountMismatch { expected: usize, received: usize },
}

/// The result of calling a dynamic [`DynamicCallable`] or [`DynamicCallableMut`].
///
/// Returns `Ok(value)` if the callable was called successfully,
/// where `value` is the [`Return`] value of the function.
///
/// [`DynamicCallable`]: crate::func::DynamicCallable
/// [`DynamicCallableMut`]: crate::func::DynamicCallableMut
pub type FunctionResult<'a> = Result<Return<'a>, FunctionError>;

/// An error that occurs when registering a callable into a [`FunctionRegistry`].
///
/// [`FunctionRegistry`]: crate::func::FunctionRegistry
#[derive(Debug, Error, PartialEq)]
pub enum FunctionRegistrationError {
    /// A callable with the given name has already been registered.
    ///
    /// Contains the duplicate callable name.
    #[error("a callable has already been registered with name {0:?}")]
    DuplicateName(Cow<'static, str>),
    /// The callable is missing a name by which it can be registered.
    #[error("callable name is missing")]
    MissingName,
}
