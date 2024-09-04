use crate::func::signature::ArgumentSignature;
use crate::func::{args::ArgError, Return};
use alloc::borrow::Cow;
use bevy_utils::HashSet;
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
    /// No overload was found for the given set of arguments.
    #[display("no overload found for arguments with signature `{received:?}`, expected one of `{expected:?}`")]
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

/// A [`FunctionInfo`] was expected but none was found.
///
/// [`FunctionInfo`]: crate::func::FunctionInfo
#[derive(Debug, Error, Display, PartialEq)]
#[display("expected a `FunctionInfo` but found none")]
pub struct MissingFunctionInfoError;

/// An error that occurs when attempting to add a function overload with a duplicate signature.
#[derive(Debug, Display, Error, PartialEq)]
#[display("could not add function overload: duplicate found for signature `{signature:?}`")]
pub struct FunctionOverloadError {
    pub signature: ArgumentSignature,
}

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
