use crate::serde::Serializable;
use crate::{PartialReflect, TypeRegistry};
use serde::Serialize;

/// A serializer for [`PartialReflect`] values.
pub(crate) struct ReflectValueSerializer<'a> {
    registry: &'a TypeRegistry,
    value: &'a dyn PartialReflect,
}

impl<'a> ReflectValueSerializer<'a> {
    pub fn new(value: &'a dyn PartialReflect, registry: &'a TypeRegistry) -> Self {
        Self { value, registry }
    }
}

impl<'a> Serialize for ReflectValueSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Serializable::try_from_reflect_value::<S::Error>(self.value, self.registry)?
            .serialize(serializer)
    }
}
