use crate::ReflectKind;
use thiserror::Error;

/// A [`TypeInfo`]-specific error.
///
/// [`TypeInfo`]: crate::info::TypeInfo
#[derive(Debug, Error)]
pub enum TypeInfoError {
    /// Caused when a type was expected to be of a certain [kind], but was not.
    ///
    /// [kind]: ReflectKind
    #[error("kind mismatch: expected {expected:?}, received {received:?}")]
    KindMismatch {
        /// Expected kind.
        expected: ReflectKind,
        /// Received kind.
        received: ReflectKind,
    },
}
