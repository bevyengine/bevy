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
    ArgCountMismatch {
        /// Expected argument count. [`ArgCount`] for overloaded functions will contain multiple possible counts.
        expected: ArgCount,
        /// Number of arguments received.
        received: usize,
    },
    /// No overload was found for the given set of arguments.
    #[error("no overload found for arguments with signature `{received:?}`, expected one of `{expected:?}`")]
    NoOverload {
        /// The set of available argument signatures.
        expected: HashSet<ArgumentSignature>,
        /// The received argument signature.
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
    /// An attempt was made to add an overload with more than [`ArgCount::MAX_COUNT`] arguments.
    ///
    /// [`ArgCount::MAX_COUNT`]: crate::func::args::ArgCount
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
