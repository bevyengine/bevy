//! Module with schemas used for various BRP endpoints
use bevy_ecs::{
    reflect::{ReflectComponent, ReflectResource},
    resource::Resource,
};
use bevy_platform::collections::HashMap;
use bevy_reflect::{
    prelude::ReflectDefault, Reflect, ReflectDeserialize, ReflectSerialize, TypeData,
    TypeRegistration,
};
use core::any::TypeId;

pub mod json_schema;
pub mod open_rpc;

/// Holds mapping of reflect [type data](TypeData) to strings,
/// later on used in Bevy Json Schema.
#[derive(Debug, Resource, Reflect)]
#[reflect(Resource)]
pub struct SchemaTypesMetadata {
    /// Type Data id mapping to strings.
    pub type_data_map: HashMap<TypeId, String>,
}

impl Default for SchemaTypesMetadata {
    fn default() -> Self {
        let mut data_types = Self {
            type_data_map: Default::default(),
        };
        data_types.map_type_data::<ReflectComponent>("Component");
        data_types.map_type_data::<ReflectResource>("Resource");
        data_types.map_type_data::<ReflectDefault>("Default");
        #[cfg(feature = "bevy_asset")]
        data_types.map_type_data::<bevy_asset::ReflectAsset>("Asset");
        #[cfg(feature = "bevy_asset")]
        data_types.map_type_data::<bevy_asset::ReflectHandle>("AssetHandle");
        data_types.map_type_data::<ReflectSerialize>("Serialize");
        data_types.map_type_data::<ReflectDeserialize>("Deserialize");
        data_types
    }
}

impl SchemaTypesMetadata {
    /// Map `TypeId` of `TypeData` to string
    pub fn map_type_data<T: TypeData>(&mut self, name: impl Into<String>) {
        self.type_data_map.insert(TypeId::of::<T>(), name.into());
    }

    /// Build reflect types list for a given type registration
    pub fn get_registered_reflect_types(&self, reg: &TypeRegistration) -> Vec<String> {
        self.type_data_map
            .iter()
            .filter_map(|(id, name)| reg.data_by_id(*id).and(Some(name.clone())))
            .collect()
    }

    /// Checks if slice contains string value that matches checked `TypeData`
    pub fn has_type_data<T: TypeData>(&self, types_string_slice: &[String]) -> bool {
        self.has_type_data_by_id(TypeId::of::<T>(), types_string_slice)
    }

    /// Checks if slice contains string value that matches checked `TypeData` by id.
    pub fn has_type_data_by_id(&self, id: TypeId, types_string_slice: &[String]) -> bool {
        self.type_data_map
            .get(&id)
            .is_some_and(|data_s| types_string_slice.iter().any(|e| e.eq(data_s)))
    }
}
