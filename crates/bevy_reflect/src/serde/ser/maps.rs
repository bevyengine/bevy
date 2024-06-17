use crate::serde::TypedReflectSerializer;
use crate::{Map, TypeRegistry};
use serde::ser::SerializeMap;
use serde::Serialize;

/// A serializer for [`Map`] values.
pub(super) struct MapSerializer<'a> {
    map: &'a dyn Map,
    registry: &'a TypeRegistry,
}

impl<'a> MapSerializer<'a> {
    pub fn new(map: &'a dyn Map, registry: &'a TypeRegistry) -> Self {
        Self { map, registry }
    }
}

impl<'a> Serialize for MapSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(Some(self.map.len()))?;
        for (key, value) in self.map.iter() {
            state.serialize_entry(
                &TypedReflectSerializer::new_internal(key, self.registry),
                &TypedReflectSerializer::new_internal(value, self.registry),
            )?;
        }
        state.end()
    }
}
