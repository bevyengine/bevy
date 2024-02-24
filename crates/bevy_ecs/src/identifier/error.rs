//! Error types for [`super::Identifier`] conversions. An ID can be converted
//! to various kinds, but these can fail if they are not valid forms of those
//! kinds. The error type in this module encapsulates the various failure modes.
use std::fmt;

/// An  Error type for [`super::Identifier`], mostly for providing error
/// handling for conversions of an ID to a type abstracting over the ID bits.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[non_exhaustive]
pub enum IdentifierError {
    /// A given ID has an invalid value for initialising to a [`crate::identifier::Identifier`].
    InvalidIdentifier,
    /// A given ID has an invalid configuration of bits for converting to an [`crate::entity::Entity`].
    InvalidEntityId(u64),
}

impl fmt::Display for IdentifierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier => write!(
                f,
                "The given id contains a zero value high component, which is invalid"
            ),
            Self::InvalidEntityId(_) => write!(f, "The given id is not a valid entity."),
        }
    }
}

impl std::error::Error for IdentifierError {}
