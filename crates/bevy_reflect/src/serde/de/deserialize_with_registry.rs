use crate::serde::de::error_utils::make_custom_error;
use crate::{FromType, PartialReflect, TypeRegistry};
use alloc::boxed::Box;
use serde::Deserializer;

/// Trait used to provide finer control when deserializing a reflected type with one of
/// the reflection deserializers.
///
/// This trait is the reflection equivalent of `serde`'s [`Deserialize`] trait.
/// The main difference is that this trait provides access to the [`TypeRegistry`],
/// which means that we can use the registry and all its stored type information
/// to deserialize our type.
///
/// This can be useful when writing a custom reflection deserializer where we may
/// want to handle parts of the deserialization process, but temporarily pass control
/// to the standard reflection deserializer for other parts.
///
/// For the serialization equivalent of this trait, see [`SerializeWithRegistry`].
///
/// # Rationale
///
/// Without this trait and its associated [type data], such a deserializer would have to
/// write out all of the deserialization logic itself, possibly including
/// unnecessary code duplication and trivial implementations.
///
/// This is because a normal [`Deserialize`] implementation has no knowledge of the
/// [`TypeRegistry`] and therefore cannot create a reflection-based deserializer for
/// nested items.
///
/// # Implementors
///
/// In order for this to work with the reflection deserializers like [`TypedReflectDeserializer`]
/// and [`ReflectDeserializer`], implementors should be sure to register the
/// [`ReflectDeserializeWithRegistry`] type data.
/// This can be done [via the registry] or by adding `#[reflect(DeserializeWithRegistry)]` to
/// the type definition.
///
/// [`Deserialize`]: ::serde::Deserialize
/// [`SerializeWithRegistry`]: crate::serde::SerializeWithRegistry
/// [type data]: ReflectDeserializeWithRegistry
/// [`TypedReflectDeserializer`]: crate::serde::TypedReflectDeserializer
/// [`ReflectDeserializer`]: crate::serde::ReflectDeserializer
/// [via the registry]: TypeRegistry::register_type_data
pub trait DeserializeWithRegistry<'de>: Sized {
    /// Deserialize this value using the given [Deserializer] and [`TypeRegistry`].
    ///
    /// [`Deserializer`]: ::serde::Deserializer
    fn deserialize<D>(deserializer: D, registry: &TypeRegistry) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>;
}

/// Type data used to deserialize a [`PartialReflect`] type with a custom [`DeserializeWithRegistry`] implementation.
#[derive(Clone)]
pub struct ReflectDeserializeWithRegistry {
    deserialize: fn(
        deserializer: &mut dyn erased_serde::Deserializer,
        registry: &TypeRegistry,
    ) -> Result<Box<dyn PartialReflect>, erased_serde::Error>,
}

impl ReflectDeserializeWithRegistry {
    /// Deserialize a [`PartialReflect`] type with this type data's custom [`DeserializeWithRegistry`] implementation.
    pub fn deserialize<'de, D>(
        &self,
        deserializer: D,
        registry: &TypeRegistry,
    ) -> Result<Box<dyn PartialReflect>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut erased = <dyn erased_serde::Deserializer>::erase(deserializer);
        (self.deserialize)(&mut erased, registry).map_err(make_custom_error)
    }
}

impl<T: PartialReflect + for<'de> DeserializeWithRegistry<'de>> FromType<T>
    for ReflectDeserializeWithRegistry
{
    fn from_type() -> Self {
        Self {
            deserialize: |deserializer, registry| {
                Ok(Box::new(T::deserialize(deserializer, registry)?))
            },
        }
    }
}
