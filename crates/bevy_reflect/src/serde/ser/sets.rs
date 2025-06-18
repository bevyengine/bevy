use crate::{serde::TypedReflectSerializer, Set, TypeRegistry};
use serde::{ser::SerializeSeq, Serialize};

use super::ReflectSerializerProcessor;

/// A serializer for [`Set`] values.
pub(super) struct SetSerializer<'a, P> {
    pub set: &'a dyn Set,
    pub registry: &'a TypeRegistry,
    pub processor: Option<&'a P>,
}

impl<P: ReflectSerializerProcessor> Serialize for SetSerializer<'_, P> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_seq(Some(self.set.len()))?;
        for value in self.set.iter() {
            state.serialize_element(&TypedReflectSerializer::new_internal(
                value,
                self.registry,
                self.processor,
            ))?;
        }
        state.end()
    }
}
