use crate::{error::BevyError, never::Never};

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
