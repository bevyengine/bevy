use crate::FieldId;
use alloc::borrow::Cow;
use thiserror::Error;

/// An error accessing a field via reflection.
#[derive(Error, Debug)]
pub enum ReflectFieldError {
    /// The field is readonly but was attempted to be accessed mutably.
    #[error("field `{field}` in `{container_type_path}` is readonly")]
    Readonly {
        field: FieldId,
        container_type_path: Cow<'static, str>,
    },
    /// The field does not exist.
    ///
    /// This can either mean the field does not exist on the type
    /// or it has been ignored with the `#[reflect(ignore)]` attribute.
    #[error("field `{field}` in `{container_type_path}` does not exist")]
    DoesNotExist {
        field: FieldId,
        container_type_path: Cow<'static, str>,
    },
}
