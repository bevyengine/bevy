use crate::{
    serde::{de::registration_utils::try_get_registration, TypedReflectDeserializer},
    DynamicMap, Map, MapInfo, TypeRegistry,
};
use core::{fmt, fmt::Formatter};
use serde::de::{MapAccess, Visitor};

/// A [`Visitor`] for deserializing [`Map`] values.
///
/// [`Map`]: crate::Map
pub(super) struct MapVisitor<'a> {
    map_info: &'static MapInfo,
    registry: &'a TypeRegistry,
}

impl<'a> MapVisitor<'a> {
    pub fn new(map_info: &'static MapInfo, registry: &'a TypeRegistry) -> Self {
        Self { map_info, registry }
    }
}

impl<'a, 'de> Visitor<'de> for MapVisitor<'a> {
    type Value = DynamicMap;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected map value")
    }

    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        let mut dynamic_map = DynamicMap::default();
        let key_registration = try_get_registration(self.map_info.key_ty(), self.registry)?;
        let value_registration = try_get_registration(self.map_info.value_ty(), self.registry)?;
        while let Some(key) = map.next_key_seed(TypedReflectDeserializer::new_internal(
            key_registration,
            self.registry,
        ))? {
            let value = map.next_value_seed(TypedReflectDeserializer::new_internal(
                value_registration,
                self.registry,
            ))?;
            dynamic_map.insert_boxed(key, value);
        }

        Ok(dynamic_map)
    }
}
