use crate::{array::ArrayLike, serde::TypedReflectSerializer, TypeRegistry};
use serde::{ser::SerializeTuple, Serialize};

use super::ReflectSerializerProcessor;

/// A serializer for [`Array`] values.
pub(super) struct ArraySerializer<'a, A: ArrayLike + ?Sized, P> {
    pub array: &'a A,
    pub registry: &'a TypeRegistry,
    pub processor: Option<&'a P>,
}

impl<A: ArrayLike + ?Sized, P: ReflectSerializerProcessor> Serialize for ArraySerializer<'_, A, P> {
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
