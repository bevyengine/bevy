use crate::{serde::TypedReflectSerializer, List, TypeRegistry};
use serde::{ser::SerializeSeq, Serialize};

use super::ReflectSerializerProcessor;

/// A serializer for [`List`] values.
pub(super) struct ListSerializer<'a, P> {
    pub list: &'a dyn List,
    pub registry: &'a TypeRegistry,
    pub processor: Option<&'a P>,
}

impl<P: ReflectSerializerProcessor> Serialize for ListSerializer<'_, P> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_seq(Some(self.list.len()))?;
        for value in self.list.iter() {
            state.serialize_element(&TypedReflectSerializer::new_internal(
                value,
                self.registry,
                self.processor,
            ))?;
        }
        state.end()
    }
}
