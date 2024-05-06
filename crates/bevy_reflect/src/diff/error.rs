use crate::{ApplyError, ReflectKind, VariantType};
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

/// Error enum used when applying a diff to a [`Reflect`](crate::Reflect) object.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DiffApplyError {
    /// Attempted to apply a diff of a different [kind].
    ///
    /// [kinds]: ReflectKind
    #[error("expected {expected}, but found {received}")]
    KindMismatch {
        expected: ReflectKind,
        received: ReflectKind,
    },
    #[error("expected a {expected} variant, but found a {received} variant")]
    VariantMismatch {
        expected: VariantType,
        received: VariantType,
    },
    #[error("expected a required field")]
    MissingField,
    #[error("expected type information to be present")]
    MissingTypeInfo,
    #[error("expected a diff")]
    MissingDiff,
    #[error("received a mismatched type")]
    TypeMismatch,
    #[error(transparent)]
    ReflectApplyError(#[from] ApplyError),
    #[error("failed to apply diff: {0}")]
    Other(String),
}
