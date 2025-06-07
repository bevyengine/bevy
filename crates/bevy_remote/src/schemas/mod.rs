//! Module with schemas used for various BRP endpoints

use std::any::TypeId;
use bevy_asset::{ReflectAsset, ReflectHandle};
use bevy_ecs::{reflect::{ReflectComponent, ReflectResource}, resource::Resource};
use bevy_platform::collections::HashMap;
use bevy_reflect::{prelude::ReflectDefault,  Reflect, ReflectDeserialize, ReflectSerialize, TypeData, TypeRegistration};

pub mod json_schema;
pub mod open_rpc;


/// Holds mapping of reflect data types to strings, 
/// later on used in Bevy Json Schema. 
#[derive(Debug, Resource, Reflect)]
#[reflect(Resource)]
pub struct SchemaTypesMetadata {
    /// data types id mapping to strings.
    pub data_types: HashMap<TypeId, String>,
}

impl Default for SchemaTypesMetadata {
    fn default() -> Self {
        let mut data_types = Self {
            data_types: Default::default(),
        };
        data_types.register_type::<ReflectComponent>("Component");
        data_types.register_type::<ReflectResource>("Resource");
        data_types.register_type::<ReflectDefault>("Default");
        data_types.register_type::<ReflectAsset>("Asset");
        data_types.register_type::<ReflectHandle>("AssetHandle");
        data_types.register_type::<ReflectSerialize>("Serialize");
        data_types.register_type::<ReflectDeserialize>("Deserialize");
        data_types
    }
}

impl SchemaTypesMetadata {
    /// Map TypeId of TypeData to string
    pub fn register_type<T: TypeData>(&mut self, name: impl Into<String>) {
        self.data_types.insert(TypeId::of::<T>(), name.into());
    }

    /// build reflect types list for a given type registration
    pub fn get_registered_reflect_types(&self, reg: &TypeRegistration) -> Vec<String> {
        self.data_types
            .iter()
            .flat_map(|(id, name)| reg.data_by_id(*id).and(Some(name.clone())))
            .collect()
    }
}