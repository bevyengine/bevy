//! Module with schemas used for various BRP endpoints
use alloc::borrow::Cow;
use bevy_ecs::{
    reflect::{ReflectComponent, ReflectResource},
    resource::Resource,
};
use bevy_platform::collections::HashMap;
use bevy_reflect::{
    prelude::ReflectDefault, FromType, GetTypeRegistration, Reflect, ReflectDeserialize,
    ReflectSerialize, TypeData, TypePath, TypeRegistration,
};
use core::any::TypeId;

use crate::schemas::{
    open_rpc::OpenRpcDocument,
    reflect_info::{FieldsInformation, InternalSchemaType, TypeReferencePath},
};

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

/// Custom internal schema data.
#[derive(Debug, Resource, Reflect, Clone)]
#[reflect(Resource)]
pub struct CustomInternalSchemaData(pub InternalSchemaType);

/// Trait for external schema sources.
pub trait ExternalSchemaSource {
    /// Get the external schema source.
    fn get_external_schema_source() -> TypeReferencePath;
}

impl<T: Reflect + ExternalSchemaSource> FromType<T> for CustomInternalSchemaData {
    fn from_type() -> Self {
        Self(InternalSchemaType::ExternalSource(
            T::get_external_schema_source(),
        ))
    }
}

/// Helper trait
pub(crate) trait RegisterReflectJsonSchemas {
    /// Register types and or type data that are implemented by this crate
    fn register_schema_base_types(&mut self) {
        #[cfg(feature = "bevy_math")]
        {
            self.registry_force_schema_to_be_array::<bevy_math::Vec2>();
            self.registry_force_schema_to_be_array::<bevy_math::DVec2>();
            self.registry_force_schema_to_be_array::<bevy_math::I8Vec2>();
            self.registry_force_schema_to_be_array::<bevy_math::U8Vec2>();
            self.registry_force_schema_to_be_array::<bevy_math::I16Vec2>();
            self.registry_force_schema_to_be_array::<bevy_math::U16Vec2>();
            self.registry_force_schema_to_be_array::<bevy_math::IVec2>();
            self.registry_force_schema_to_be_array::<bevy_math::UVec2>();
            self.registry_force_schema_to_be_array::<bevy_math::I64Vec2>();
            self.registry_force_schema_to_be_array::<bevy_math::U64Vec2>();
            self.registry_force_schema_to_be_array::<bevy_math::BVec2>();

            self.registry_force_schema_to_be_array::<bevy_math::Vec3A>();
            self.registry_force_schema_to_be_array::<bevy_math::Vec3>();
            self.registry_force_schema_to_be_array::<bevy_math::DVec3>();
            self.registry_force_schema_to_be_array::<bevy_math::I8Vec3>();
            self.registry_force_schema_to_be_array::<bevy_math::U8Vec3>();
            self.registry_force_schema_to_be_array::<bevy_math::I16Vec3>();
            self.registry_force_schema_to_be_array::<bevy_math::U16Vec3>();
            self.registry_force_schema_to_be_array::<bevy_math::IVec3>();
            self.registry_force_schema_to_be_array::<bevy_math::UVec3>();
            self.registry_force_schema_to_be_array::<bevy_math::I64Vec3>();
            self.registry_force_schema_to_be_array::<bevy_math::U64Vec3>();
            self.registry_force_schema_to_be_array::<bevy_math::BVec3>();

            self.registry_force_schema_to_be_array::<bevy_math::Vec4>();
            self.registry_force_schema_to_be_array::<bevy_math::DVec4>();
            self.registry_force_schema_to_be_array::<bevy_math::I8Vec4>();
            self.registry_force_schema_to_be_array::<bevy_math::U8Vec4>();
            self.registry_force_schema_to_be_array::<bevy_math::I16Vec4>();
            self.registry_force_schema_to_be_array::<bevy_math::U16Vec4>();
            self.registry_force_schema_to_be_array::<bevy_math::IVec4>();
            self.registry_force_schema_to_be_array::<bevy_math::UVec4>();
            self.registry_force_schema_to_be_array::<bevy_math::I64Vec4>();
            self.registry_force_schema_to_be_array::<bevy_math::U64Vec4>();
            self.registry_force_schema_to_be_array::<bevy_math::BVec4>();

            self.registry_force_schema_to_be_array::<bevy_math::Quat>();
            self.registry_force_schema_to_be_array::<bevy_math::DQuat>();

            self.registry_force_schema_to_be_array::<bevy_math::Mat2>();
            self.registry_force_schema_to_be_array::<bevy_math::DMat2>();
            self.registry_force_schema_to_be_array::<bevy_math::DMat3>();
            self.registry_force_schema_to_be_array::<bevy_math::Mat3A>();
            self.registry_force_schema_to_be_array::<bevy_math::Mat3>();
            self.registry_force_schema_to_be_array::<bevy_math::DMat4>();
            self.registry_force_schema_to_be_array::<bevy_math::Mat4>();
            self.registry_force_schema_to_be_array::<bevy_math::Affine2>();
            self.registry_force_schema_to_be_array::<bevy_math::DAffine2>();
            self.registry_force_schema_to_be_array::<bevy_math::DAffine3>();
            self.registry_force_schema_to_be_array::<bevy_math::Affine3A>();
        }
        self.register_type_internal::<OpenRpcDocument>();
        self.register_type_data_internal::<OpenRpcDocument, CustomInternalSchemaData>();
    }
    /// Registers a type by value.
    fn register_data_type_by_value<T, D>(&mut self, data: D)
    where
        T: Reflect + TypePath + GetTypeRegistration,
        D: TypeData;
    fn register_type_internal<T>(&mut self)
    where
        T: GetTypeRegistration;

    fn register_type_data_internal<T, D>(&mut self)
    where
        T: Reflect + TypePath + GetTypeRegistration,
        D: TypeData + FromType<T>;
    /// Registers a [`CustomInternalSchemaData`] data type for a type that will force to treat the type as an array during building the [`json_schema::JsonSchemaBevyType`] for given type.
    /// It is useful when you want to force the type to be treated as an array in the schema, for example when type has custom serialization.
    fn registry_force_schema_to_be_array<T>(&mut self)
    where
        T: Reflect + TypePath + GetTypeRegistration,
    {
        let bevy_reflect::TypeInfo::Struct(struct_info) = T::get_type_registration().type_info()
        else {
            return;
        };
        let data =
            CustomInternalSchemaData(InternalSchemaType::FieldsHolder(FieldsInformation::new(
                struct_info.iter(),
                reflect_info::FieldType::ForceUnnamed(struct_info.ty().id()),
            )));
        self.register_data_type_by_value::<T, CustomInternalSchemaData>(data);
    }
}

impl RegisterReflectJsonSchemas for bevy_reflect::TypeRegistry {
    fn register_type_data_internal<T, D>(&mut self)
    where
        T: Reflect + TypePath + GetTypeRegistration,
        D: TypeData + FromType<T>,
    {
        if !self.contains(TypeId::of::<T>()) {
            self.register::<T>();
        }
        self.register_type_data::<T, D>();
    }

    fn register_type_internal<T>(&mut self)
    where
        T: GetTypeRegistration,
    {
        self.register::<T>();
    }

    fn register_data_type_by_value<T, D>(&mut self, data: D)
    where
        T: Reflect + TypePath + GetTypeRegistration,
        D: TypeData,
    {
        if let Some(type_reg) = self.get_mut(TypeId::of::<T>()) {
            type_reg.insert(data);
        }
    }
}
impl RegisterReflectJsonSchemas for bevy_app::App {
    fn register_type_data_internal<T, D>(&mut self)
    where
        T: Reflect + TypePath + GetTypeRegistration,
        D: TypeData + FromType<T>,
    {
        self.register_type::<T>();
        self.register_type_data::<T, D>();
    }

    fn register_type_internal<T>(&mut self)
    where
        T: GetTypeRegistration,
    {
        self.register_type::<T>();
    }

    fn register_data_type_by_value<T, D>(&mut self, data: D)
    where
        T: Reflect + TypePath + GetTypeRegistration,
        D: TypeData,
    {
        let sub_app = self.main_mut();
        let world = sub_app.world_mut();
        let registry = world.resource_mut::<bevy_ecs::reflect::AppTypeRegistry>();
        let mut r = registry.write();
        r.register_data_type_by_value::<T, D>(data);
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
    pub fn get_registered_reflect_types(&self, reg: &TypeRegistration) -> Vec<&Cow<'static, str>> {
        self.type_data_map
            .iter()
            .filter_map(|(id, name)| reg.data_by_id(*id).and(Some(name)))
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
