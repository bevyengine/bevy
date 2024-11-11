use crate::{serde::TypedReflectSerializer, Array, TypeRegistry};
use serde::{ser::SerializeTuple, Serialize};

/// A serializer for [`Array`] values.
pub(super) struct ArraySerializer<'a> {
    array: &'a dyn Array,
    registry: &'a TypeRegistry,
}

impl<'a> ArraySerializer<'a> {
    pub fn new(array: &'a dyn Array, registry: &'a TypeRegistry) -> Self {
        Self { array, registry }
    }
}

impl<'a> Serialize for ArraySerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_tuple(self.array.len())?;
        for value in self.array.iter() {
            state.serialize_element(&TypedReflectSerializer::new_internal(value, self.registry))?;
        }
        state.end()
    }
}
