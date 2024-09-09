use crate::serde::ser::error_utils::make_custom_error;
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
        let type_info = self.map.get_represented_type_info().ok_or_else(|| {
            make_custom_error(format_args!(
                "cannot get type info for` {}`",
                self.map.reflect_type_path()
            ))
        })?;

        let map_info = type_info.as_map().map_err(make_custom_error)?;
        let key_info = map_info.key_info();
        let value_info = map_info.value_info();

        let mut state = serializer.serialize_map(Some(self.map.len()))?;
        for (key, value) in self.map.iter() {
            state.serialize_entry(
                &TypedReflectSerializer::new_internal(key, key_info, self.registry),
                &TypedReflectSerializer::new_internal(value, value_info, self.registry),
            )?;
        }
        state.end()
    }
}
