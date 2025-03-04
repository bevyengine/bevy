use crate::{serde::TypedReflectSerializer, Array, TypeRegistry};
use serde::{ser::SerializeTuple, Serialize};

use super::ReflectSerializerProcessor;

/// A serializer for [`Array`] values.
pub(super) struct ArraySerializer<'a, P> {
    pub array: &'a dyn Array,
    pub registry: &'a TypeRegistry,
    pub processor: Option<&'a P>,
}

impl<P: ReflectSerializerProcessor> Serialize for ArraySerializer<'_, P> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_tuple(self.array.len())?;
        for value in self.array.iter() {
            state.serialize_element(&TypedReflectSerializer::new_internal(
                value,
                self.registry,
                self.processor,
            ))?;
        }
        state.end()
    }
}
