use crate::FieldId;
use alloc::{borrow::Cow, format};
use thiserror::Error;

/// An error that occurs when cloning a type via [`PartialReflect::reflect_clone`].
///
/// [`PartialReflect::reflect_clone`]: crate::PartialReflect::reflect_clone
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ReflectCloneError {
    /// The type does not have a custom implementation for [`PartialReflect::reflect_clone`].
    ///
    /// [`PartialReflect::reflect_clone`]: crate::PartialReflect::reflect_clone
    #[error("`PartialReflect::reflect_clone` not implemented for `{type_path}`")]
    NotImplemented { type_path: Cow<'static, str> },
    /// The type cannot be cloned via [`PartialReflect::reflect_clone`].
    ///
    /// This type should be returned when a type is intentionally opting out of reflection cloning.
    ///
    /// [`PartialReflect::reflect_clone`]: crate::PartialReflect::reflect_clone
    #[error("`{type_path}` cannot be made cloneable for `PartialReflect::reflect_clone`")]
    NotCloneable { type_path: Cow<'static, str> },
    /// The field cannot be cloned via [`PartialReflect::reflect_clone`].
    ///
    /// When [deriving `Reflect`], this usually means that a field marked with `#[reflect(ignore)]`
    /// is missing a `#[reflect(clone)]` attribute.
    ///
    /// This may be intentional if the field is not meant/able to be cloned.
    ///
    /// [`PartialReflect::reflect_clone`]: crate::PartialReflect::reflect_clone
    /// [deriving `Reflect`]: derive@crate::Reflect
    #[error(
        "field `{}` cannot be made cloneable for `PartialReflect::reflect_clone` (are you missing a `#[reflect(clone)]` attribute?)",
        full_path(.field, .variant.as_deref(), .container_type_path)
    )]
    FieldNotCloneable {
        field: FieldId,
        variant: Option<Cow<'static, str>>,
        container_type_path: Cow<'static, str>,
    },
    /// Could not downcast to the expected type.
    ///
    /// Realistically this should only occur when a type has incorrectly implemented [`Reflect`].
    ///
    /// [`Reflect`]: crate::Reflect
    #[error("expected downcast to `{expected}`, but received `{received}`")]
    FailedDowncast {
        expected: Cow<'static, str>,
        received: Cow<'static, str>,
    },
}

fn full_path(
    field: &FieldId,
    variant: Option<&str>,
    container_type_path: &str,
) -> alloc::string::String {
    match variant {
        Some(variant) => format!("{}::{}::{}", container_type_path, variant, field),
        None => format!("{}::{}", container_type_path, field),
    }
}
