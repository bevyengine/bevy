use crate::{serde::TypedReflectSerializer, tuple::TupleLike, TypeRegistry};
use serde::{ser::SerializeTuple, Serialize};

use super::ReflectSerializerProcessor;

/// A serializer for [`Tuple`] values.
pub(super) struct TupleSerializer<'a, T: TupleLike + ?Sized, P> {
    pub tuple: &'a T,
    pub registry: &'a TypeRegistry,
    pub processor: Option<&'a P>,
}

impl<T: TupleLike + ?Sized, P: ReflectSerializerProcessor> Serialize for TupleSerializer<'_, T, P> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_tuple(self.tuple.field_len())?;

        for value in self.tuple.iter_fields() {
            state.serialize_element(&TypedReflectSerializer::new_internal(
                value,
                self.registry,
                self.processor,
            ))?;
        }
        state.end()
    }
}
