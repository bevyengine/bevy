//! Module with schemas used for various BRP endpoints
use alloc::borrow::Cow;
use bevy_derive::Deref;
use bevy_ecs::{
    reflect::{ReflectComponent, ReflectResource},
    resource::Resource,
};
use bevy_platform::collections::HashMap;
use bevy_reflect::{
    prelude::ReflectDefault, FromType, Reflect, ReflectDeserialize, ReflectSerialize, TypeData,
    TypeRegistration,
};
use core::any::TypeId;

use crate::schemas::{json_schema::JsonSchemaBevyType, open_rpc::OpenRpcDocument};

pub mod json_schema;
pub mod open_rpc;
pub mod reflect_info;

/// Holds mapping of reflect [type data](TypeData) to human-readable type names,
/// later on used in Bevy Json Schema.
#[derive(Debug, Resource, Reflect, Clone)]
#[reflect(Resource)]
pub struct SchemaTypesMetadata {
    /// Type Data id mapping to human-readable type names.
    pub type_data_map: HashMap<TypeId, Cow<'static, str>>,
}

/// Reflect-compatible custom JSON Schema for this type
#[derive(Clone)]
pub struct ReflectJsonSchemaForceAsArray;

impl<T: Reflect> FromType<T> for ReflectJsonSchemaForceAsArray {
    fn from_type() -> Self {
        ReflectJsonSchemaForceAsArray
    }
}

/// Helper trait
pub(crate) trait RegisterReflectJsonSchemas {
    /// Register types and or type data that are implemented by this crate
    fn register_schema_base_types(&mut self) {
        #[cfg(feature = "bevy_math")]
        {
            self.register_type_data_internal::<bevy_math::Vec2, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::DVec2, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::I8Vec2, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::U8Vec2, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::I16Vec2, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::U16Vec2, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::IVec2, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::UVec2, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::I64Vec2, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::U64Vec2, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::BVec2, ReflectJsonSchemaForceAsArray>();

            self.register_type_data_internal::<bevy_math::Vec3, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::DVec3, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::I8Vec3, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::U8Vec3, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::I16Vec3, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::U16Vec3, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::IVec3, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::UVec3, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::I64Vec3, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::U64Vec3, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::BVec3, ReflectJsonSchemaForceAsArray>();

            self.register_type_data_internal::<bevy_math::Vec4, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::DVec4, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::I8Vec4, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::U8Vec4, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::I16Vec4, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::U16Vec4, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::IVec4, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::UVec4, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::I64Vec4, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::U64Vec4, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::BVec4, ReflectJsonSchemaForceAsArray>();

            self.register_type_data_internal::<bevy_math::Quat, ReflectJsonSchemaForceAsArray>();
            self.register_type_data_internal::<bevy_math::DQuat, ReflectJsonSchemaForceAsArray>();
        }
        self.register_type_internal::<OpenRpcDocument>();
        self.register_type_data_internal::<OpenRpcDocument, ReflectJsonSchema>();
    }
    fn register_type_internal<T>(&mut self)
    where
        T: bevy_reflect::GetTypeRegistration;

    fn register_type_data_internal<T, D>(&mut self)
    where
        T: Reflect + bevy_reflect::TypePath + bevy_reflect::GetTypeRegistration,
        D: TypeData + FromType<T>;
}

impl RegisterReflectJsonSchemas for bevy_reflect::TypeRegistry {
    fn register_type_data_internal<T, D>(&mut self)
    where
        T: Reflect + bevy_reflect::TypePath + bevy_reflect::GetTypeRegistration,
        D: TypeData + FromType<T>,
    {
        if !self.contains(TypeId::of::<T>()) {
            self.register::<T>();
        }
        self.register_type_data::<T, D>();
    }

    fn register_type_internal<T>(&mut self)
    where
        T: bevy_reflect::GetTypeRegistration,
    {
        self.register::<T>();
    }
}
impl RegisterReflectJsonSchemas for bevy_app::App {
    fn register_type_data_internal<T, D>(&mut self)
    where
        T: Reflect + bevy_reflect::TypePath + bevy_reflect::GetTypeRegistration,
        D: TypeData + FromType<T>,
    {
        self.register_type::<T>();
        self.register_type_data::<T, D>();
    }

    fn register_type_internal<T>(&mut self)
    where
        T: bevy_reflect::GetTypeRegistration,
    {
        self.register_type::<T>();
    }
}

/// Reflect-compatible custom JSON Schema for this type
#[derive(Clone, Deref)]
pub struct ReflectJsonSchema(pub JsonSchemaBevyType);

impl From<&JsonSchemaBevyType> for ReflectJsonSchema {
    fn from(schema: &JsonSchemaBevyType) -> Self {
        Self(schema.clone())
    }
}

impl From<JsonSchemaBevyType> for ReflectJsonSchema {
    fn from(schema: JsonSchemaBevyType) -> Self {
        Self(schema)
    }
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
    /// Map `TypeId` of `TypeData` to a human-readable type name
    pub fn map_type_data<T: TypeData>(&mut self, name: impl Into<Cow<'static, str>>) {
        self.type_data_map.insert(TypeId::of::<T>(), name.into());
    }

    /// Build reflect types list for a given type registration
    pub fn get_registered_reflect_types(&self, reg: &TypeRegistration) -> Vec<Cow<'static, str>> {
        self.type_data_map
            .iter()
            .filter_map(|(id, name)| reg.data_by_id(*id).and(Some(name.clone())))
            .collect()
    }

    /// Checks if slice contains a type name that matches the checked `TypeData`
    pub fn has_type_data<T: TypeData>(&self, types_string_slice: &[Cow<'static, str>]) -> bool {
        self.has_type_data_by_id(TypeId::of::<T>(), types_string_slice)
    }

    /// Checks if slice contains a type name that matches the checked `TypeData` by id.
    pub fn has_type_data_by_id(
        &self,
        id: TypeId,
        types_string_slice: &[Cow<'static, str>],
    ) -> bool {
        self.type_data_map
            .get(&id)
            .is_some_and(|data_s| types_string_slice.iter().any(|e| e.eq(data_s)))
    }
}
