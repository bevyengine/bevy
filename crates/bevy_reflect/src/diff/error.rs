use crate::ReflectKind;
use thiserror::Error;

/// Error enum used when diffing two [`Reflect`](crate::Reflect) objects.
#[derive(Debug, PartialEq, Eq, Error)]
pub enum DiffError {
    /// Attempted to diff two values of differing [kinds].
    ///
    /// [kinds]: ReflectKind
    #[error("expected {expected}, but found {received}")]
    KindMismatch {
        expected: ReflectKind,
        received: ReflectKind,
    },
    #[error("expected a required field")]
    MissingField,
    #[error("expected type information to be present")]
    MissingInfo,
    #[error("the given values cannot be compared")]
    Incomparable,
}
