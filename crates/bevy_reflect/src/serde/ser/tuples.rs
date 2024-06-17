use crate::serde::TypedReflectSerializer;
use crate::{Tuple, TypeRegistry};
use serde::ser::SerializeTuple;
use serde::Serialize;

/// A serializer for [`Tuple`] values.
pub(super) struct TupleSerializer<'a> {
    tuple: &'a dyn Tuple,
    registry: &'a TypeRegistry,
}

impl<'a> TupleSerializer<'a> {
    pub fn new(tuple: &'a dyn Tuple, registry: &'a TypeRegistry) -> Self {
        Self { tuple, registry }
    }
}

impl<'a> Serialize for TupleSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_tuple(self.tuple.field_len())?;

        for value in self.tuple.iter_fields() {
            state.serialize_element(&TypedReflectSerializer::new_internal(value, self.registry))?;
        }
        state.end()
    }
}
