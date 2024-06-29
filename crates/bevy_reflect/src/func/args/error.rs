use alloc::borrow::Cow;

use thiserror::Error;

use crate::func::args::{ArgId, Ownership};

/// An error that occurs when converting an [argument].
///
/// [argument]: crate::func::Arg
#[derive(Debug, Error, PartialEq)]
pub enum ArgError {
    /// The argument is not the expected type.
    #[error("expected `{expected}` but received `{received}` (@ {id:?})")]
    UnexpectedType {
        id: ArgId,
        expected: Cow<'static, str>,
        received: Cow<'static, str>,
    },
    /// The argument has the wrong ownership.
    #[error("expected {expected} value but received {received} value (@ {id:?})")]
    InvalidOwnership {
        id: ArgId,
        expected: Ownership,
        received: Ownership,
    },
}
