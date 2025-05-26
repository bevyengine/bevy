use crate::func::signature::ArgumentSignature;
use crate::func::{
    args::{ArgCount, ArgError},
    Return,
};
use alloc::borrow::Cow;
use bevy_platform::collections::HashSet;
use thiserror::Error;

/// An error that occurs when calling a [`DynamicFunction`] or [`DynamicFunctionMut`].
///
/// [`DynamicFunction`]: crate::func::DynamicFunction
/// [`DynamicFunctionMut`]: crate::func::DynamicFunctionMut
#[derive(Debug, Error, PartialEq)]
pub enum FunctionError {
    /// An error occurred while converting an argument.
    #[error(transparent)]
    ArgError(#[from] ArgError),
    /// The number of arguments provided does not match the expected number.
    #[error("received {received} arguments but expected one of {expected:?}")]
    ArgCountMismatch { expected: ArgCount, received: usize },
    /// No overload was found for the given set of arguments.
    #[error("no overload found for arguments with signature `{received:?}`, expected one of `{expected:?}`")]
    NoOverload {
        expected: HashSet<ArgumentSignature>,
        received: ArgumentSignature,
    },
}

/// The result of calling a [`DynamicFunction`] or [`DynamicFunctionMut`].
///
/// Returns `Ok(value)` if the function was called successfully,
/// where `value` is the [`Return`] value of the function.
///
/// [`DynamicFunction`]: crate::func::DynamicFunction
/// [`DynamicFunctionMut`]: crate::func::DynamicFunctionMut
pub type FunctionResult<'a> = Result<Return<'a>, FunctionError>;

/// An error that occurs when attempting to add a function overload.
#[derive(Debug, Error, PartialEq)]
pub enum FunctionOverloadError {
    /// A [`SignatureInfo`] was expected, but none was found.
    ///
    /// [`SignatureInfo`]: crate::func::info::SignatureInfo
    #[error("expected at least one `SignatureInfo` but found none")]
    MissingSignature,
    /// An error that occurs when attempting to add a function overload with a duplicate signature.
    #[error("could not add function overload: duplicate found for signature `{0:?}`")]
    DuplicateSignature(ArgumentSignature),
    #[error(
        "argument signature `{:?}` has too many arguments (max {})",
        0,
        ArgCount::MAX_COUNT
    )]
    TooManyArguments(ArgumentSignature),
}

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
