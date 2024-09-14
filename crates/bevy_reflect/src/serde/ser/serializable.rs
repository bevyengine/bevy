use crate::serde::ser::error_utils::make_custom_error;
use crate::{PartialReflect, ReflectSerialize, TypeRegistry};
use serde::ser::Error;
use std::ops::Deref;

/// A type-erased serializable value.
pub enum Serializable<'a> {
    Owned(Box<dyn erased_serde::Serialize + 'a>),
    Borrowed(&'a dyn erased_serde::Serialize),
}

impl<'a> Serializable<'a> {
    /// Attempts to create a [`Serializable`] from a [`PartialReflect`] value.
    ///
    /// Returns an error if any of the following conditions are met:
    /// - The underlying type of `value` does not implement [`Reflect`].
    /// - The underlying type of `value` does not represent any type (via [`PartialReflect::get_represented_type_info`]).
    /// - The represented type of `value` is not registered in the `type_registry`.
    /// - The represented type of `value` did not register the [`ReflectSerialize`] type data.
    ///
    /// [`Reflect`]: crate::Reflect
    pub fn try_from_reflect_value<E: Error>(
        value: &'a dyn PartialReflect,
        type_registry: &TypeRegistry,
    ) -> Result<Serializable<'a>, E> {
        let value = value.try_as_reflect().ok_or_else(|| {
            make_custom_error(format_args!(
                "type `{}` does not implement `Reflect`",
                value.reflect_type_path()
            ))
        })?;

        let info = value.reflect_type_info();

        let registration = type_registry.get(info.type_id()).ok_or_else(|| {
            make_custom_error(format_args!(
                "type `{}` is not registered in the type registry",
                info.type_path(),
            ))
        })?;

        let reflect_serialize = registration.data::<ReflectSerialize>().ok_or_else(|| {
            make_custom_error(format_args!(
                "type `{}` did not register the `ReflectSerialize` type data. For certain types, this may need to be registered manually using `register_type_data`",
                info.type_path(),
            ))
        })?;

        Ok(reflect_serialize.get_serializable(value))
    }
}

impl<'a> Deref for Serializable<'a> {
    type Target = dyn erased_serde::Serialize + 'a;

    fn deref(&self) -> &Self::Target {
        match self {
            Serializable::Borrowed(serialize) => serialize,
            Serializable::Owned(serialize) => serialize,
        }
    }
}
