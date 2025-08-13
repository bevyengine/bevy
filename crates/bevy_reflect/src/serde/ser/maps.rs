use crate::{serde::TypedReflectSerializer, Map, TypeRegistry};
use serde::{ser::SerializeMap, Serialize};

use super::ReflectSerializerProcessor;

/// A serializer for [`Map`] values.
pub(super) struct MapSerializer<'a, P> {
    pub map: &'a dyn Map,
    pub registry: &'a TypeRegistry,
    pub processor: Option<&'a P>,
}

impl<P: ReflectSerializerProcessor> Serialize for MapSerializer<'_, P> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(Some(self.map.len()))?;
        for (key, value) in self.map.iter() {
            state.serialize_entry(
                &TypedReflectSerializer::new_internal(key, self.registry, self.processor),
                &TypedReflectSerializer::new_internal(value, self.registry, self.processor),
            )?;
        }
        state.end()
    }
}
