use crate::{FromType, Reflect, TypeRegistry};
use alloc::boxed::Box;
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
/// For the deserialization equivalent of this trait, see [`DeserializeWithRegistry`].
///
/// # Rationale
///
/// Without this trait and its associated [type data], such a serializer would have to
/// write out all of the serialization logic itself, possibly including
/// unnecessary code duplication and trivial implementations.
///
/// This is because a normal [`Serialize`] implementation has no knowledge of the
/// [`TypeRegistry`] and therefore cannot create a reflection-based serializer for
/// nested items.
///
/// # Implementors
///
/// In order for this to work with the reflection serializers like [`TypedReflectSerializer`]
/// and [`ReflectSerializer`], implementors should be sure to register the
/// [`ReflectSerializeWithRegistry`] type data.
/// This can be done [via the registry] or by adding `#[reflect(SerializeWithRegistry)]` to
/// the type definition.
///
/// [`DeserializeWithRegistry`]: crate::serde::DeserializeWithRegistry
/// [type data]: ReflectSerializeWithRegistry
/// [`TypedReflectSerializer`]: crate::serde::TypedReflectSerializer
/// [`ReflectSerializer`]: crate::serde::ReflectSerializer
/// [via the registry]: TypeRegistry::register_type_data
pub trait SerializeWithRegistry {
    /// Serialize this value using the given [Serializer] and [`TypeRegistry`].
    ///
    /// [`Serializer`]: ::serde::Serializer
    fn serialize<S>(&self, serializer: S, registry: &TypeRegistry) -> Result<S::Ok, S::Error>
    where
        S: Serializer;
}

/// Type data used to serialize a [`Reflect`] type with a custom [`SerializeWithRegistry`] implementation.
#[derive(Clone)]
pub struct ReflectSerializeWithRegistry {
    serialize: for<'a> fn(
        value: &'a dyn Reflect,
        registry: &'a TypeRegistry,
    ) -> Box<dyn erased_serde::Serialize + 'a>,
}

impl ReflectSerializeWithRegistry {
    /// Serialize a [`Reflect`] type with this type data's custom [`SerializeWithRegistry`] implementation.
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

impl<T: Reflect + SerializeWithRegistry> FromType<T> for ReflectSerializeWithRegistry {
    fn from_type() -> Self {
        Self {
            serialize: |value: &dyn Reflect, registry| {
                let value = value.downcast_ref::<T>().unwrap_or_else(|| {
                    panic!(
                        "Expected value to be of type {} but received {}",
                        core::any::type_name::<T>(),
                        value.reflect_type_path()
                    )
                });
                Box::new(SerializableWithRegistry { value, registry })
            },
        }
    }
}

struct SerializableWithRegistry<'a, T: SerializeWithRegistry> {
    value: &'a T,
    registry: &'a TypeRegistry,
}

impl<'a, T: SerializeWithRegistry> Serialize for SerializableWithRegistry<'a, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.value.serialize(serializer, self.registry)
    }
}
