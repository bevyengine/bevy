use crate::{serde::de::error_utils::make_custom_error, TypeRegistration, TypeRegistry};
use core::{fmt, fmt::Formatter};
use serde::de::{DeserializeSeed, Error, Visitor};

/// A deserializer for type registrations.
///
/// This will return a [`&TypeRegistration`] corresponding to the given type.
/// This deserializer expects a string containing the _full_ [type path] of the
/// type to find the `TypeRegistration` of.
///
/// [`&TypeRegistration`]: TypeRegistration
/// [type path]: crate::TypePath::type_path
pub struct TypeRegistrationDeserializer<'a> {
    registry: &'a TypeRegistry,
}

impl<'a> TypeRegistrationDeserializer<'a> {
    /// Creates a new [`TypeRegistrationDeserializer`].
    pub fn new(registry: &'a TypeRegistry) -> Self {
        Self { registry }
    }
}

impl<'a, 'de> DeserializeSeed<'de> for TypeRegistrationDeserializer<'a> {
    type Value = &'a TypeRegistration;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct TypeRegistrationVisitor<'a>(&'a TypeRegistry);

        impl<'de, 'a> Visitor<'de> for TypeRegistrationVisitor<'a> {
            type Value = &'a TypeRegistration;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("string containing `type` entry for the reflected value")
            }

            fn visit_str<E>(self, type_path: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                self.0.get_with_type_path(type_path).ok_or_else(|| {
                    make_custom_error(format_args!("no registration found for `{type_path}`"))
                })
            }
        }

        deserializer.deserialize_str(TypeRegistrationVisitor(self.registry))
    }
}
