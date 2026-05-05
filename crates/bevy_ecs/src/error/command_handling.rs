use crate::{
    error::BevyError, never::Never, system::entity_command::EntityCommandError,
    world::error::EntityMutableFetchError,
};
use core::fmt::{Debug, Display};

/// A trait implemented for types that can be used as the output of a [`Command`].
///
/// [`Command`]: crate::system::Command
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid `Command` output type",
    label = "invalid `Command` output type",
    note = "the output type of a `Command` should be `()`, `Never`, or a `Result` where the error type can be converted into `BevyError`"
)]
pub trait CommandOutput: Sized {
    /// Converts the output into an optional [`BevyError`].
    fn to_err(self) -> Option<BevyError>;
}

impl<T, E> CommandOutput for Result<T, E>
where
    E: Into<BevyError>,
{
    #[inline]
    fn to_err(self) -> Option<BevyError> {
        self.err().map(Into::into)
    }
}

impl CommandOutput for Never {
    #[inline]
    fn to_err(self) -> Option<BevyError> {
        None
    }
}

impl CommandOutput for () {
    #[inline]
    fn to_err(self) -> Option<BevyError> {
        None
    }
}

/// A trait implemented for types that can be used as the output of an [`EntityCommand`].
///
/// [`EntityCommand`]: crate::system::EntityCommand
pub trait EntityCommandOutput {
    /// The type returned when the command is successfully applied.
    type Out;

    /// The error type returned when the command fails to apply. The type must
    /// be convertible into a [`BevyError`] and constructible from an
    /// [`EntityMutableFetchError`].
    type Error: Into<BevyError> + From<EntityMutableFetchError>;

    /// Converts the output into a `Result` containing either the successful output or an error.
    fn into_result(self) -> Result<Self::Out, Self::Error>;
}

impl EntityCommandOutput for () {
    type Out = ();
    type Error = EntityMutableFetchError;

    #[inline]
    fn into_result(self) -> Result<Self::Out, Self::Error> {
        Ok(())
    }
}

impl<T, E> EntityCommandOutput for Result<T, E>
where
    E: Debug + Display + Send + Sync + 'static,
{
    type Out = T;
    type Error = EntityCommandError<E>;

    #[inline]
    fn into_result(self) -> Result<Self::Out, Self::Error> {
        self.map_err(EntityCommandError::CommandFailed)
    }
}
