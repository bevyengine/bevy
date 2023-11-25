use crate::{FromType, Reflect, TypeRegistry};
use serde::{Serialize, Serializer};

/// Trait used to provide finer control when serializing a reflected type with one of
/// the reflection serializers.
///
/// This trait is the reflection equivalent of `serde`'s [`Serialize`] trait.
/// The main difference is that this trait provides access to the [`TypeRegistry`],
/// which means that we can use the registry and all its stored type information
/// to serialize our type.
///
/// This can be useful when writing a custom reflection serializer where we may
/// want to handle parts of the serialization process, but temporarily pass control
/// to the standard reflection serializer for other parts.
///
/// Without this trait and its associated [type data], such a serializer would have to
/// write out all of the serialization logic itself, possibly including
/// unnecessary code duplication and trivial implementations.
///
/// For the deserializtion equivalent of this trait, see [`DeserializeReflect`].
///
/// # Implementors
///
/// In order for this to work with the reflection serializers like [`TypedReflectSerializer`]
/// and [`ReflectSerializer`], implementors should be sure to register the
/// [`ReflectSerializeReflect`] type data.
/// This can be done [via the registry] or by adding `#[reflect(SerializeReflect)]` to
/// the type definition.
///
/// Note that this trait has a blanket implementation for all types that implement
/// [`Reflect`] and [`Serialize`].
///
/// [type data]: ReflectSerializeReflect
/// [`TypedReflectSerializer`]: crate::serde::TypedReflectSerializer
/// [`ReflectSerializer`]: crate::serde::ReflectSerializer
/// [via the registry]: TypeRegistry::register_type_data
pub trait SerializeReflect {
    fn serialize<S>(&self, serializer: S, registry: &TypeRegistry) -> Result<S::Ok, S::Error>
    where
        S: Serializer;
}

impl<T: Reflect + Serialize> SerializeReflect for T {
    fn serialize<S>(&self, serializer: S, _registry: &TypeRegistry) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        <T as Serialize>::serialize(self, serializer)
    }
}

/// Type data used to serialize a [`Reflect`] type with a custom [`SerializeReflect`] implementation.
#[derive(Clone)]
pub struct ReflectSerializeReflect {
    serialize: for<'a> fn(
        value: &'a dyn Reflect,
        registry: &'a TypeRegistry,
    ) -> Box<dyn erased_serde::Serialize + 'a>,
}

impl ReflectSerializeReflect {
    /// Serialize a [`Reflect`] type with this type data's custom [`SerializeReflect`] implementation.
    pub fn serialize<S>(
        &self,
        value: &dyn Reflect,
        serializer: S,
        registry: &TypeRegistry,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ((self.serialize)(value, registry)).serialize(serializer)
    }
}

impl<T: Reflect + SerializeReflect> FromType<T> for ReflectSerializeReflect {
    fn from_type() -> Self {
        Self {
            serialize: |value, registry| {
                let value = value.downcast_ref::<T>().unwrap();
                Box::new(SerializableWithRegistry { value, registry })
            },
        }
    }
}

struct SerializableWithRegistry<'a, T: SerializeReflect> {
    value: &'a T,
    registry: &'a TypeRegistry,
}

impl<'a, T: SerializeReflect> Serialize for SerializableWithRegistry<'a, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.value.serialize(serializer, self.registry)
    }
}
