//! Module with JSON Schema type for Bevy Registry Types.
//!  It tries to follow this standard: <https://json-schema.org/specification>
use alloc::{borrow::Cow, collections::BTreeMap};
use bevy_annotations::{DisplayName, ReadOnly, Step, Tooltip, Unit, ValueRange};
use bevy_ecs::component::{Components, StorageType};
use bevy_ecs::{component::ComponentInfo, relationship::RelationshipAccessor};
use bevy_log::warn;
use bevy_reflect::{
    attributes::CustomAttributes,
    enums::VariantInfo,
    serde::{SerializationData, TypedReflectSerializer},
    std_traits::ReflectDefault,
    CreateTypeData, GetTypeRegistration, NamedField, OpaqueInfo, ReflectSerialize, TypeInfo,
    TypePath, TypeRegistration, TypeRegistry,
};
use core::any::TypeId;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::schemas::SchemaTypesMetadata;

/// Helper trait for converting `TypeRegistration` to `JsonSchemaBevyType`
pub trait TypeRegistrySchemaReader {
    /// Export type JSON Schema.
    fn export_type_json_schema<T: GetTypeRegistration + 'static>(
        &self,
        extra_info: &SchemaTypesMetadata,
        components: &Components,
    ) -> Option<JsonSchemaBevyType> {
        self.export_type_json_schema_for_id(extra_info, TypeId::of::<T>(), components)
    }
    /// Export type JSON Schema.
    fn export_type_json_schema_for_id(
        &self,
        extra_info: &SchemaTypesMetadata,
        type_id: TypeId,
        components: &Components,
    ) -> Option<JsonSchemaBevyType>;
}

impl TypeRegistrySchemaReader for TypeRegistry {
    fn export_type_json_schema_for_id(
        &self,
        extra_info: &SchemaTypesMetadata,
        type_id: TypeId,
        components: &Components,
    ) -> Option<JsonSchemaBevyType> {
        let type_reg = self.get(type_id)?;
        Some((type_reg, extra_info, components, self).into())
    }
}

/// Exports schema info for a given type
pub fn export_type(
    reg: &TypeRegistration,
    metadata: &SchemaTypesMetadata,
    components: &Components,
    registry: &TypeRegistry,
) -> (Cow<'static, str>, JsonSchemaBevyType) {
    (
        reg.type_info().type_path().into(),
        (reg, metadata, components, registry).into(),
    )
}

/// Describes the JSON a type serializes to over BRP, for types whose `Serialize`
/// implementation does not follow their reflected structure.
///
/// `registry.schema` uses [`ReflectBrpJsonSchema`] to describe such types by their serialized
/// form, keeping the reflected structure under `reflectSchema`.
///
/// ```
/// # use bevy_reflect::{Reflect, ReflectSerialize};
/// # use bevy_remote::schemas::json_schema::{BrpJsonSchema, ReflectBrpJsonSchema};
/// # use serde::Serialize;
/// # use serde_json::{json, Map, Value};
/// #[derive(Reflect, Clone, Serialize)]
/// #[reflect(opaque, Clone, Serialize, BrpJsonSchema)]
/// #[serde(transparent)]
/// struct Tag(String);
///
/// impl BrpJsonSchema for Tag {
///     fn json_schema() -> Map<String, Value> {
///         let mut schema = Map::new();
///         schema.insert("type".into(), json!("string"));
///         schema
///     }
/// }
/// ```
pub trait BrpJsonSchema {
    /// The JSON Schema keywords describing the serialized value.
    ///
    /// Only standard keywords are expected; use [`json_schema_ref`] to refer to other types.
    fn json_schema() -> Map<String, Value>;
}

/// Type data for [`BrpJsonSchema`], registered with `#[reflect(BrpJsonSchema)]`.
#[derive(Clone)]
pub struct ReflectBrpJsonSchema {
    json_schema: fn() -> Map<String, Value>,
}

impl ReflectBrpJsonSchema {
    /// The JSON Schema keywords describing the serialized value.
    pub fn json_schema(&self) -> Map<String, Value> {
        (self.json_schema)()
    }
}

impl<T: BrpJsonSchema> CreateTypeData<T> for ReflectBrpJsonSchema {
    fn create_type_data(_input: ()) -> Self {
        Self {
            json_schema: T::json_schema,
        }
    }
}

/// A `$ref` to the schema of `T`.
pub fn json_schema_ref<T: TypePath + ?Sized>() -> Value {
    json!({ "$ref": type_ref(T::type_path()) })
}

/// Unwraps a `json!` object literal.
fn object(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        _ => Map::new(),
    }
}

const COMPONENTS: [&str; 4] = ["x", "y", "z", "w"];
const AXES: [&str; 4] = ["x_axis", "y_axis", "z_axis", "w_axis"];

/// Labels of the components of a vector with `n` components.
fn vector_labels(prefix: &str, n: usize) -> Vec<String> {
    COMPONENTS[..n]
        .iter()
        .map(|component| format!("{prefix}{component}"))
        .collect()
}

/// Labels of the column-major components of an `n` by `n` matrix.
fn matrix_labels(prefix: &str, n: usize) -> Vec<String> {
    AXES[..n]
        .iter()
        .flat_map(|axis| vector_labels(&format!("{prefix}{axis}."), n))
        .collect()
}

/// Labels of an affine transform: its `n` by `n` matrix followed by its translation.
fn affine_labels(n: usize) -> Vec<String> {
    let mut labels = matrix_labels(&format!("matrix{n}."), n);
    labels.extend(vector_labels("translation.", n));
    labels
}

/// A fixed-length array of `T`, with one titled item per label.
fn fixed_array<T: TypePath>(labels: Vec<String>) -> Map<String, Value> {
    let len = labels.len();
    let prefix_items: Vec<Value> = labels
        .into_iter()
        .map(|label| {
            let mut item = json_schema_ref::<T>();
            item["title"] = label.into();
            item
        })
        .collect();
    object(json!({
        "type": "array",
        "prefixItems": prefix_items,
        "items": false,
        "minItems": len,
        "maxItems": len,
    }))
}

macro_rules! impl_brp_json_schemas {
    ($($(#[$meta:meta])* $ty:ty => $schema:expr;)*) => {
        $(
            $(#[$meta])*
            impl BrpJsonSchema for $ty {
                fn json_schema() -> Map<String, Value> {
                    $schema
                }
            }
        )*

        /// Registers [`ReflectBrpJsonSchema`] for the types that BRP serializes differently
        /// from their reflected structure, such as `glam` vectors and matrices, [`Entity`]
        /// and [`Name`].
        ///
        /// [`RemotePlugin`](crate::RemotePlugin) calls this on the app's type registry.
        ///
        /// [`Entity`]: bevy_ecs::entity::Entity
        /// [`Name`]: bevy_ecs::name::Name
        pub fn register_brp_json_schemas(registry: &mut TypeRegistry) {
            $(
                $(#[$meta])*
                {
                    registry.register::<$ty>();
                    registry.register_type_data::<$ty, ReflectBrpJsonSchema>();
                }
            )*
        }
    };
}

mod builtin {
    use super::*;
    use alloc::borrow::Cow;
    use bevy_ecs::{entity::Entity, name::Name};
    use bevy_math::{
        Affine2, Affine3, Affine3A, BVec2, BVec3, BVec3A, BVec4, BVec4A, DAffine2, DAffine3, DMat2,
        DMat3, DMat4, DQuat, DVec2, DVec3, DVec4, I16Vec2, I16Vec3, I16Vec4, I64Vec2, I64Vec3,
        I64Vec4, I8Vec2, I8Vec3, I8Vec4, IVec2, IVec3, IVec4, Mat2, Mat3, Mat3A, Mat4, Quat,
        U16Vec2, U16Vec3, U16Vec4, U64Vec2, U64Vec3, U64Vec4, U8Vec2, U8Vec3, U8Vec4, UVec2, UVec3,
        UVec4, Vec2, Vec3, Vec3A, Vec4,
    };
    use core::{
        net::SocketAddr,
        num::{
            NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU128,
            NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
        },
        time::Duration,
    };
    use std::path::PathBuf;

    fn string() -> Map<String, Value> {
        object(json!({ "type": "string" }))
    }

    fn non_zero_unsigned() -> Map<String, Value> {
        object(json!({ "type": "integer", "minimum": 1 }))
    }

    fn non_zero_signed() -> Map<String, Value> {
        object(json!({ "type": "integer", "not": { "const": 0 } }))
    }

    #[cfg(feature = "bevy_asset")]
    fn uuid() -> Map<String, Value> {
        object(json!({ "type": "string", "format": "uuid" }))
    }

    impl_brp_json_schemas! {
        Vec2 => fixed_array::<f32>(vector_labels("", 2));
        Vec3 => fixed_array::<f32>(vector_labels("", 3));
        Vec3A => fixed_array::<f32>(vector_labels("", 3));
        Vec4 => fixed_array::<f32>(vector_labels("", 4));
        DVec2 => fixed_array::<f64>(vector_labels("", 2));
        DVec3 => fixed_array::<f64>(vector_labels("", 3));
        DVec4 => fixed_array::<f64>(vector_labels("", 4));
        IVec2 => fixed_array::<i32>(vector_labels("", 2));
        IVec3 => fixed_array::<i32>(vector_labels("", 3));
        IVec4 => fixed_array::<i32>(vector_labels("", 4));
        I8Vec2 => fixed_array::<i8>(vector_labels("", 2));
        I8Vec3 => fixed_array::<i8>(vector_labels("", 3));
        I8Vec4 => fixed_array::<i8>(vector_labels("", 4));
        I16Vec2 => fixed_array::<i16>(vector_labels("", 2));
        I16Vec3 => fixed_array::<i16>(vector_labels("", 3));
        I16Vec4 => fixed_array::<i16>(vector_labels("", 4));
        I64Vec2 => fixed_array::<i64>(vector_labels("", 2));
        I64Vec3 => fixed_array::<i64>(vector_labels("", 3));
        I64Vec4 => fixed_array::<i64>(vector_labels("", 4));
        UVec2 => fixed_array::<u32>(vector_labels("", 2));
        UVec3 => fixed_array::<u32>(vector_labels("", 3));
        UVec4 => fixed_array::<u32>(vector_labels("", 4));
        U8Vec2 => fixed_array::<u8>(vector_labels("", 2));
        U8Vec3 => fixed_array::<u8>(vector_labels("", 3));
        U8Vec4 => fixed_array::<u8>(vector_labels("", 4));
        U16Vec2 => fixed_array::<u16>(vector_labels("", 2));
        U16Vec3 => fixed_array::<u16>(vector_labels("", 3));
        U16Vec4 => fixed_array::<u16>(vector_labels("", 4));
        U64Vec2 => fixed_array::<u64>(vector_labels("", 2));
        U64Vec3 => fixed_array::<u64>(vector_labels("", 3));
        U64Vec4 => fixed_array::<u64>(vector_labels("", 4));
        BVec2 => fixed_array::<bool>(vector_labels("", 2));
        BVec3 => fixed_array::<bool>(vector_labels("", 3));
        BVec3A => fixed_array::<bool>(vector_labels("", 3));
        BVec4 => fixed_array::<bool>(vector_labels("", 4));
        BVec4A => fixed_array::<bool>(vector_labels("", 4));
        Quat => fixed_array::<f32>(vector_labels("", 4));
        DQuat => fixed_array::<f64>(vector_labels("", 4));
        Mat2 => fixed_array::<f32>(matrix_labels("", 2));
        Mat3 => fixed_array::<f32>(matrix_labels("", 3));
        Mat3A => fixed_array::<f32>(matrix_labels("", 3));
        Mat4 => fixed_array::<f32>(matrix_labels("", 4));
        DMat2 => fixed_array::<f64>(matrix_labels("", 2));
        DMat3 => fixed_array::<f64>(matrix_labels("", 3));
        DMat4 => fixed_array::<f64>(matrix_labels("", 4));
        Affine2 => fixed_array::<f32>(affine_labels(2));
        Affine3 => fixed_array::<f32>(affine_labels(3));
        Affine3A => fixed_array::<f32>(affine_labels(3));
        DAffine2 => fixed_array::<f64>(affine_labels(2));
        DAffine3 => fixed_array::<f64>(affine_labels(3));
        NonZeroU8 => non_zero_unsigned();
        NonZeroU16 => non_zero_unsigned();
        NonZeroU32 => non_zero_unsigned();
        NonZeroU64 => non_zero_unsigned();
        NonZeroU128 => non_zero_unsigned();
        NonZeroUsize => non_zero_unsigned();
        NonZeroI8 => non_zero_signed();
        NonZeroI16 => non_zero_signed();
        NonZeroI32 => non_zero_signed();
        NonZeroI64 => non_zero_signed();
        NonZeroI128 => non_zero_signed();
        NonZeroIsize => non_zero_signed();
        Duration => object(json!({
            "type": "object",
            "properties": {
                "secs": json_schema_ref::<u64>(),
                "nanos": json_schema_ref::<u32>(),
            },
            "required": ["secs", "nanos"],
            "additionalProperties": false,
        }));
        Entity => object(json!({
            "type": "string",
            "pattern": "^([0-9]+v[0-9]+|PLACEHOLDER)$",
        }));
        Name => string();
        Cow<'static, str> => string();
        PathBuf => string();
        SocketAddr => string();
        #[cfg(feature = "bevy_asset")]
        bevy_asset::AssetPath<'static> => string();
        #[cfg(feature = "bevy_asset")]
        bevy_asset::uuid::Uuid => uuid();
        #[cfg(feature = "bevy_asset")]
        bevy_asset::uuid::NonNilUuid => uuid();
    }
}

pub use builtin::register_brp_json_schemas;

/// Serializes the [`ReflectDefault`] value of a type, if it has one and it can be serialized.
fn default_value(reg: &TypeRegistration, registry: &TypeRegistry) -> Option<Value> {
    let value = reg.data::<ReflectDefault>()?.default();
    serde_json::to_value(TypedReflectSerializer::new(
        value.as_partial_reflect(),
        registry,
    ))
    .ok()
}

/// Adds the `annotations` key and the matching standard keywords to `schema`.
fn with_annotations(mut schema: Value, annotations: Option<SchemaAnnotations>) -> Value {
    if let (Some(annotations), Some(object)) = (annotations, schema.as_object_mut()) {
        if let Some([minimum, maximum]) = annotations.range {
            object.insert("minimum".into(), json!(minimum));
            object.insert("maximum".into(), json!(maximum));
        }
        if annotations.read_only {
            object.insert("readOnly".into(), true.into());
        }
        if let Some(title) = &annotations.display_name {
            object.insert("title".into(), json!(title));
        }
        if let Some(description) = &annotations.tooltip {
            object.insert("description".into(), json!(description));
        }
        object.insert("annotations".into(), json!(annotations));
    }
    schema
}

/// Schema of an enum variant that serializes as `{"<name>": value}`.
fn variant_wrapper(name: &str, value: Value) -> Value {
    json!({
        "type": "object",
        "properties": { name: value },
        "required": [name],
        "additionalProperties": false,
    })
}

/// Type paths of the unsigned integer primitives.
const UNSIGNED_INTEGERS: [&str; 6] = ["u8", "u16", "u32", "u64", "u128", "usize"];

/// A `$ref` to the schema of the type with the given type path.
fn type_ref(type_path: &str) -> String {
    format!("#/{type_path}")
}

impl
    From<(
        &TypeRegistration,
        &SchemaTypesMetadata,
        &Components,
        &TypeRegistry,
    )> for JsonSchemaBevyType
{
    fn from(
        value: (
            &TypeRegistration,
            &SchemaTypesMetadata,
            &Components,
            &TypeRegistry,
        ),
    ) -> Self {
        let (reg, metadata, components, registry) = value;
        let t = reg.type_info();
        let binding = t.type_path_table();

        let short_path = binding.short_path();
        let type_path = binding.path();
        let mut typed_schema = JsonSchemaBevyType {
            reflect_types: metadata.get_registered_reflect_types(reg),
            short_path: short_path.to_owned(),
            type_path: type_path.to_owned(),
            crate_name: binding.crate_name().map(str::to_owned),
            module_path: binding.module_path().map(str::to_owned),
            default: default_value(reg, registry),
            ..Default::default()
        };
        let component_info: Option<&ComponentInfo> = components
            .get_valid_id(t.type_id())
            .and_then(|component_id| components.get_info(component_id));
        typed_schema.component_info = component_info.map(|info| {
            let mutable = info.mutable();
            let storage_type = info.storage_type().into();
            let is_send_and_sync = info.is_send_and_sync();
            let required_component_types = info
                .required_components()
                .iter_ids()
                .flat_map(|component_id| components.get_info(component_id))
                .map(|info: &ComponentInfo| info.name().to_string())
                .collect::<Vec<_>>();
            let relationship_kind = info
                .relationship_accessor()
                .map(|&relationship| relationship.into());
            ComponentMetadata {
                mutable,
                storage_type,
                is_send_and_sync,
                required_component_types,
                relationship_kind,
            }
        });
        let serialization_data = reg.data::<SerializationData>();
        let is_serialized =
            |index: usize| !serialization_data.is_some_and(|data| data.is_field_skipped(index));
        let is_newtype = matches!(t, TypeInfo::TupleStruct(info) if info.field_len() == 1)
            && serialization_data.is_none();
        typed_schema.set_annotations(match t {
            TypeInfo::Struct(info) => SchemaAnnotations::from_attributes(info.custom_attributes()),
            TypeInfo::TupleStruct(info) if is_newtype => SchemaAnnotations::merged(
                SchemaAnnotations::from_attributes(info.custom_attributes()),
                SchemaAnnotations::from_attributes(info.field_at(0).unwrap().custom_attributes()),
            ),
            TypeInfo::TupleStruct(info) => {
                SchemaAnnotations::from_attributes(info.custom_attributes())
            }
            TypeInfo::Enum(info) => SchemaAnnotations::from_attributes(info.custom_attributes()),
            _ => None,
        });
        match t {
            TypeInfo::Struct(info) => {
                typed_schema.properties = info
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| is_serialized(*index))
                    .map(|(index, field)| (field.name().to_owned(), field.property(index)))
                    .collect();
                typed_schema.required = info
                    .iter()
                    .enumerate()
                    .filter(|(index, field)| {
                        is_serialized(*index)
                            && !field.type_path().starts_with("core::option::Option")
                    })
                    .map(|(_, field)| field.name().to_owned())
                    .collect::<Vec<_>>();
                typed_schema.additional_properties = Some(false.into());
                typed_schema.schema_type = Some(SchemaType::Object);
                typed_schema.kind = SchemaKind::Struct;
            }
            TypeInfo::Enum(info) => {
                typed_schema.kind = SchemaKind::Enum;
                let is_option = binding.module_path() == Some("core::option")
                    && binding.ident() == Some("Option");

                if info
                    .iter()
                    .all(|variant| matches!(variant, VariantInfo::Unit(_)))
                {
                    typed_schema.schema_type = Some(SchemaType::String);
                }
                typed_schema.one_of = info
                    .iter()
                    .map(|variant| {
                        let name = variant.name();
                        let (schema, kind) = match variant {
                            VariantInfo::Unit(_) if is_option => (json!({ "type": "null" }), None),
                            VariantInfo::Tuple(v) if is_option => {
                                let mut schema = v.field_at(0).unwrap().ref_type();
                                schema["not"] = json!({ "type": "null" });
                                (schema, Some("Tuple"))
                            }
                            VariantInfo::Unit(_) => (json!({ "const": name }), None),
                            VariantInfo::Struct(v) => (
                                variant_wrapper(
                                    name,
                                    json!({
                                        "type": "object",
                                        "properties": v
                                            .iter()
                                            .enumerate()
                                            .map(|(index, field)| {
                                                (field.name().to_owned(), field.property(index))
                                            })
                                            .collect::<Map<_, _>>(),
                                        "additionalProperties": false,
                                        "required": v
                                            .iter()
                                            .filter(|field| {
                                                !field.type_path().starts_with("core::option::Option")
                                            })
                                            .map(NamedField::name)
                                            .collect::<Vec<_>>(),
                                    }),
                                ),
                                Some("Struct"),
                            ),
                            VariantInfo::Tuple(v) if v.field_len() == 1 => (
                                variant_wrapper(name, v.field_at(0).unwrap().ref_type()),
                                Some("Tuple"),
                            ),
                            VariantInfo::Tuple(v) => (
                                variant_wrapper(
                                    name,
                                    json!({
                                        "type": "array",
                                        "prefixItems": v
                                            .iter()
                                            .map(SchemaJsonReference::ref_type)
                                            .collect::<Vec<_>>(),
                                        "items": false,
                                        "minItems": v.field_len(),
                                        "maxItems": v.field_len(),
                                    }),
                                ),
                                Some("Tuple"),
                            ),
                        };
                        let mut schema = with_annotations(
                            schema,
                            SchemaAnnotations::from_attributes(variant.custom_attributes()),
                        );
                        if let Some(object) = schema.as_object_mut() {
                            if let Some(kind) = kind {
                                object.insert("kind".into(), kind.into());
                            }
                            object.insert("typePath".into(), format!("{type_path}::{name}").into());
                            object.insert("shortPath".into(), name.into());
                        }
                        schema
                    })
                    .collect::<Vec<_>>();
            }
            TypeInfo::TupleStruct(info) if is_newtype => {
                typed_schema.reference = Some(type_ref(info.field_at(0).unwrap().type_path()));
                typed_schema.kind = SchemaKind::TupleStruct;
            }
            TypeInfo::TupleStruct(info) => {
                typed_schema.schema_type = Some(SchemaType::Array);
                typed_schema.kind = SchemaKind::TupleStruct;
                typed_schema.prefix_items = info
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| is_serialized(*index))
                    .map(|(_, field)| field.ref_type())
                    .collect::<Vec<_>>();
                typed_schema.items = Some(false.into());
                typed_schema.min_items = Some(typed_schema.prefix_items.len());
                typed_schema.max_items = Some(typed_schema.prefix_items.len());
            }
            TypeInfo::List(info) => {
                typed_schema.schema_type = Some(SchemaType::Array);
                typed_schema.kind = SchemaKind::List;
                typed_schema.items = info.item_ty().ref_type().into();
            }
            TypeInfo::Array(info) => {
                typed_schema.schema_type = Some(SchemaType::Array);
                typed_schema.kind = SchemaKind::Array;
                typed_schema.items = info.item_ty().ref_type().into();
                typed_schema.min_items = Some(info.capacity());
                typed_schema.max_items = Some(info.capacity());
            }
            TypeInfo::Map(info) => {
                typed_schema.schema_type = Some(SchemaType::Object);
                typed_schema.kind = SchemaKind::Map;
                typed_schema.key_type = info.key_ty().ref_type().into();
                typed_schema.value_type = info.value_ty().ref_type().into();
                typed_schema.additional_properties = info.value_ty().ref_type().into();
            }
            TypeInfo::Tuple(info) => {
                typed_schema.schema_type = Some(SchemaType::Array);
                typed_schema.kind = SchemaKind::Tuple;
                typed_schema.prefix_items = info
                    .iter()
                    .map(SchemaJsonReference::ref_type)
                    .collect::<Vec<_>>();
                typed_schema.items = Some(false.into());
                typed_schema.min_items = Some(info.field_len());
                typed_schema.max_items = Some(info.field_len());
            }
            TypeInfo::Set(info) => {
                typed_schema.schema_type = Some(SchemaType::Array);
                typed_schema.unique_items = true;
                typed_schema.kind = SchemaKind::Set;
                typed_schema.items = info.value_ty().ref_type().into();
            }
            TypeInfo::Opaque(info) => {
                typed_schema.schema_type = info.map_json_type();
                typed_schema.minimum = UNSIGNED_INTEGERS.contains(&info.type_path()).then_some(0.0);
                typed_schema.kind = SchemaKind::Value;
                typed_schema.raw_json = typed_schema.schema_type.is_none()
                    && reg.data::<ReflectSerialize>().is_some()
                    && reg.data::<ReflectBrpJsonSchema>().is_none();
            }
        };
        if let Some(wire) = reg.data::<ReflectBrpJsonSchema>() {
            typed_schema.apply_wire_schema(wire.json_schema());
        }
        typed_schema
    }
}

/// JSON Schema type for Bevy Registry Types.
/// It tries to follow this standard: <https://json-schema.org/specification>
///
/// To take the full advantage from info provided by Bevy registry it provides extra fields
///
/// Non-finite floats (NaN and infinity) serialize as `null` in JSON, so they will not validate
/// against `number` schemas.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct JsonSchemaBevyType {
    /// Bevy specific field, short path of the type.
    pub short_path: String,
    /// Bevy specific field, full path of the type.
    pub type_path: String,
    /// Bevy specific field, path of the module that type is part of.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub module_path: Option<String>,
    /// Bevy specific field, name of the crate that type is part of.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub crate_name: Option<String>,
    /// Bevy specific field, names of the types that type reflects.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub reflect_types: Vec<String>,
    /// Bevy specific field, [`TypeInfo`] type mapping.
    pub kind: SchemaKind,
    /// Bevy specific field, provided when [`SchemaKind`] `kind` field is equal to [`SchemaKind::Map`].
    ///
    /// It contains type info of key of the Map.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub key_type: Option<Value>,
    /// Bevy specific field, provided when [`SchemaKind`] `kind` field is equal to [`SchemaKind::Map`].
    ///
    /// It contains type info of value of the Map.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value_type: Option<Value>,
    /// The type keyword is fundamental to JSON Schema. It specifies the data type for a schema.
    ///
    /// `None` when the schema is a [`reference`](Self::reference) or the value can take more
    /// than one JSON type, as with `Option<T>`.
    /// Also `None` for opaque types with custom serialization, such as `Entity` or `Duration`.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none", default)]
    pub schema_type: Option<SchemaType>,
    /// A reference to the schema of another type that this type serializes as,
    /// such as the inner type of a newtype tuple struct.
    #[serde(rename = "$ref", skip_serializing_if = "Option::is_none", default)]
    pub reference: Option<String>,
    /// The behavior of this keyword depends on the presence and annotation results of "properties"
    /// and "patternProperties" within the same schema object.
    /// Validation with "additionalProperties" applies only to the child
    /// values of instance names that do not appear in the annotation results of either "properties" or "patternProperties".
    ///
    /// `false` for structs, or the value type schema for maps.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub additional_properties: Option<Value>,
    /// Additional metadata if this schema represents a component.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub component_info: Option<ComponentMetadata>,
    /// Validation succeeds if, for each name that appears in both the instance and as a name
    /// within this keyword's value, the child instance for that name successfully validates
    /// against the corresponding schema.
    ///
    /// Keys are sorted by name. Each property's `fieldIndex` gives its declaration order.
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub properties: BTreeMap<String, Value>,
    /// An object instance is valid against this keyword if every item in the array is the name of a property in the instance.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub required: Vec<String>,
    /// An instance validates successfully against this keyword if it validates successfully against exactly one schema defined by this keyword's value.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub one_of: Vec<Value>,
    /// Validation succeeds if each element of the instance validates against the schema at the same position, if any. This keyword does not constrain the length of the array. If the array is longer than this keyword's value, this keyword validates only the prefix of matching length.
    ///
    /// This keyword produces an annotation value which is the largest index to which this keyword
    /// applied a subschema. The value MAY be a boolean true if a subschema was applied to every
    /// index of the instance, such as is produced by the "items" keyword.
    /// This annotation affects the behavior of "items" and "unevaluatedItems".
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub prefix_items: Vec<Value>,
    /// This keyword applies its subschema to all instance elements at indexes greater
    /// than the length of the "prefixItems" array in the same schema object,
    /// as reported by the annotation result of that "prefixItems" keyword.
    /// If no such annotation result exists, "items" applies its subschema to all
    /// instance array elements.
    ///
    /// If the "items" subschema is applied to any positions within the instance array,
    /// it produces an annotation result of boolean true, indicating that all remaining
    /// array elements have been evaluated against this keyword's subschema.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub items: Option<Value>,
    /// The minimum number of items in an array-like value: a tuple, tuple struct,
    /// tuple variant or fixed-size array.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub min_items: Option<usize>,
    /// The maximum number of items in an array-like value: a tuple, tuple struct,
    /// tuple variant or fixed-size array.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_items: Option<usize>,
    /// The default value of the type, serialized the same way as BRP component values.
    ///
    /// Only present when the type registers [`ReflectDefault`].
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "deserialize_present"
    )]
    pub default: Option<Value>,
    /// The type's [`bevy_annotations`], such as `tooltip`,
    /// `displayName` and `readOnly`.
    ///
    /// Fields and enum variants carry the same object under their own `annotations` key.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub annotations: Option<SchemaAnnotations>,
    /// Display name of the type, from [`SchemaAnnotations::display_name`].
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub title: Option<String>,
    /// Help text for the type, from [`SchemaAnnotations::tooltip`].
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    /// Inclusive lower bound, from [`SchemaAnnotations::range`].
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub minimum: Option<f64>,
    /// Inclusive upper bound, from [`SchemaAnnotations::range`].
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub maximum: Option<f64>,
    /// Whether tools should not edit the value, from [`SchemaAnnotations::read_only`].
    #[serde(skip_serializing_if = "is_false", default)]
    pub read_only: bool,
    /// Whether array items must be unique, as with sets.
    #[serde(skip_serializing_if = "is_false", default)]
    pub unique_items: bool,
    /// Bevy specific field, set on opaque types that serialize with their own `Serialize`
    /// implementation into JSON whose shape is not known.
    ///
    /// Such schemas have no `type`. Register [`ReflectBrpJsonSchema`] to describe them.
    #[serde(skip_serializing_if = "is_false", default)]
    pub raw_json: bool,
    /// Bevy specific field, set when the type registers [`ReflectBrpJsonSchema`].
    ///
    /// Holds the structural keywords (`type`, `$ref`, `properties`, `prefixItems`, ...) of the
    /// type's reflected structure, while the top-level keywords describe its serialized form.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reflect_schema: Option<Map<String, Value>>,
    /// Other JSON Schema keywords, such as `format` or `not`, from [`BrpJsonSchema`].
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Keywords that describe a type's structure, moved into
/// [`reflect_schema`](JsonSchemaBevyType::reflect_schema) when a type declares its serialized form.
pub(crate) const STRUCTURAL_KEYWORDS: [&str; 11] = [
    "type",
    "$ref",
    "properties",
    "required",
    "additionalProperties",
    "prefixItems",
    "items",
    "minItems",
    "maxItems",
    "oneOf",
    "uniqueItems",
];

impl JsonSchemaBevyType {
    /// Moves the structural keywords into [`reflect_schema`](Self::reflect_schema) and applies
    /// the declared `wire` keywords at the top level.
    fn apply_wire_schema(&mut self, wire: Map<String, Value>) {
        let Ok(Value::Object(mut schema)) = serde_json::to_value(&*self) else {
            return;
        };
        let reflected: Map<String, Value> = STRUCTURAL_KEYWORDS
            .iter()
            .filter_map(|keyword| Some(((*keyword).to_owned(), schema.remove(*keyword)?)))
            .collect();
        schema.extend(wire);
        match serde_json::from_value::<Self>(Value::Object(schema)) {
            Ok(wire_schema) => {
                *self = wire_schema;
                self.reflect_schema = (!reflected.is_empty()).then_some(reflected);
            }
            Err(error) => warn!(
                "Ignoring the BrpJsonSchema of `{}`: {error}",
                self.type_path
            ),
        }
    }

    /// Sets [`annotations`](Self::annotations) and the standard keywords derived from them.
    fn set_annotations(&mut self, annotations: Option<SchemaAnnotations>) {
        if let Some(annotations) = &annotations {
            self.title.clone_from(&annotations.display_name);
            self.description.clone_from(&annotations.tooltip);
            self.minimum = annotations.range.map(|range| range[0]);
            self.maximum = annotations.range.map(|range| range[1]);
            self.read_only = annotations.read_only;
        }
        self.annotations = annotations;
    }
}

/// Annotations read from [`bevy_annotations`] on a type, field or enum variant.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SchemaAnnotations {
    /// Inclusive `[min, max]` bounds, from [`ValueRange`] or a bare numeric range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<[f64; 2]>,
    /// Increment per step, from [`Step`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    /// Unit label, from [`Unit`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Whether tools should not edit the value, from [`ReadOnly`].
    #[serde(skip_serializing_if = "is_false")]
    pub read_only: bool,
    /// Help text, from [`Tooltip`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
    /// Label shown in place of the name, from [`DisplayName`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

impl SchemaAnnotations {
    /// Reads the annotations from `attributes`, returning `None` when no annotation is present.
    pub fn from_attributes(attributes: &CustomAttributes) -> Option<Self> {
        let annotations = Self {
            range: ValueRange::from_attributes(attributes).map(|range| [range.min(), range.max()]),
            step: attributes.get::<Step>().map(|step| step.0),
            unit: attributes.get::<Unit>().map(|unit| unit.0.to_owned()),
            read_only: attributes.contains::<ReadOnly>(),
            tooltip: attributes
                .get::<Tooltip>()
                .map(|tooltip| tooltip.0.to_owned()),
            display_name: attributes
                .get::<DisplayName>()
                .map(|name| name.0.to_owned()),
        };
        (annotations != Self::default()).then_some(annotations)
    }

    /// Merges `primary` and `fallback` per key, preferring `primary` where both are set.
    pub fn merged(primary: Option<Self>, fallback: Option<Self>) -> Option<Self> {
        let (primary, fallback) = match (primary, fallback) {
            (Some(primary), Some(fallback)) => (primary, fallback),
            (primary, fallback) => return primary.or(fallback),
        };
        Some(Self {
            range: primary.range.or(fallback.range),
            step: primary.step.or(fallback.step),
            unit: primary.unit.or(fallback.unit),
            read_only: primary.read_only || fallback.read_only,
            tooltip: primary.tooltip.or(fallback.tooltip),
            display_name: primary.display_name.or(fallback.display_name),
        })
    }
}

fn is_false(value: &bool) -> bool {
    !value
}

/// Deserializes a present field as `Some`, so a `null` default is kept rather than dropped.
fn deserialize_present<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Value>, D::Error> {
    Value::deserialize(deserializer).map(Some)
}

/// Kind of json schema, maps [`TypeInfo`] type
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub enum SchemaKind {
    /// Struct
    #[default]
    Struct,
    /// Enum type
    Enum,
    /// A key-value map
    Map,
    /// Array
    Array,
    /// List
    List,
    /// Fixed size collection of items
    Tuple,
    /// Fixed size collection of items with named fields
    TupleStruct,
    /// Set of unique values
    Set,
    /// Single value, eg. primitive types
    Value,
}

/// Type of json schema
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SchemaType {
    /// Represents a string value.
    String,

    /// Represents any number.
    Number,

    /// Represents an integer.
    Integer,

    /// Represents an object with key-value pairs.
    Object,

    /// Represents an array of values.
    Array,

    /// Represents a boolean value (true or false).
    Boolean,

    /// Represents a null value.
    #[default]
    Null,
}

/// Component-specific metadata. Related to [`ComponentInfo`].
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ComponentMetadata {
    /// Whether component is mutable or not.
    pub mutable: bool,
    /// The storage used for this component.
    pub storage_type: StorageKind,
    /// Whether component is `Send + Sync`.
    pub is_send_and_sync: bool,
    /// Type path of [required components](`bevy_ecs::component::RequiredComponent`).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub required_component_types: Vec<String>,
    /// Kind of relationship, if the component has one of the [relationship traits](`bevy_ecs::relationship::Relationship`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub relationship_kind: Option<RelationshipKind>,
}

/// The storage used for a specific component type.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub enum StorageKind {
    /// Provides fast and cache-friendly iteration, but slower addition and removal of components.
    #[default]
    Table,
    /// Provides fast addition and removal of components, but slower iteration.
    SparseSet,
}

impl From<StorageType> for StorageKind {
    fn from(value: StorageType) -> Self {
        match value {
            StorageType::Table => StorageKind::Table,
            StorageType::SparseSet => StorageKind::SparseSet,
        }
    }
}

/// Kind of relationship.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum RelationshipKind {
    /// The child kind of relationship.
    Relationship,
    /// The parent kind of relationship.
    RelationshipTarget,
}

impl From<RelationshipAccessor> for RelationshipKind {
    fn from(value: RelationshipAccessor) -> Self {
        match value {
            RelationshipAccessor::Relationship { .. } => RelationshipKind::Relationship,
            RelationshipAccessor::RelationshipTarget { .. } => RelationshipKind::RelationshipTarget,
        }
    }
}

/// Helper trait for generating json schema reference
trait SchemaJsonReference {
    /// Reference to another type in schema.
    /// The value `$ref` is a URI-reference that is resolved against the schema.
    fn ref_type(self) -> Value;
}

/// Helper trait for mapping bevy type path into json schema type
pub trait SchemaJsonType {
    /// Bevy Reflect type path
    fn get_type_path(&self) -> &'static str;

    /// JSON Schema type keyword from Bevy reflect type path into.
    ///
    /// `None` when the JSON type is not known, as with opaque types that use their own
    /// serialization.
    fn map_json_type(&self) -> Option<SchemaType> {
        match self.get_type_path() {
            "bool" => Some(SchemaType::Boolean),
            path if UNSIGNED_INTEGERS.contains(&path) => Some(SchemaType::Integer),
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => Some(SchemaType::Integer),
            "f32" | "f64" => Some(SchemaType::Number),
            "char" | "str" | "alloc::string::String" => Some(SchemaType::String),
            _ => None,
        }
    }
}

impl SchemaJsonType for OpaqueInfo {
    fn get_type_path(&self) -> &'static str {
        self.type_path()
    }
}

impl SchemaJsonReference for &bevy_reflect::Type {
    fn ref_type(self) -> Value {
        json!({ "$ref": type_ref(self.path()) })
    }
}

impl SchemaJsonReference for &bevy_reflect::UnnamedField {
    fn ref_type(self) -> Value {
        with_annotations(
            json!({ "$ref": type_ref(self.type_path()) }),
            SchemaAnnotations::from_attributes(self.custom_attributes()),
        )
    }
}

/// Helper trait for building the schema of a struct property.
trait SchemaJsonProperty {
    /// Property schema, with `index` being the field's position in its declaration.
    fn property(self, index: usize) -> Value;
}

impl SchemaJsonProperty for &NamedField {
    fn property(self, index: usize) -> Value {
        with_annotations(
            json!({
                "$ref": type_ref(self.type_path()),
                "fieldIndex": index,
            }),
            SchemaAnnotations::from_attributes(self.custom_attributes()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::prelude::ReflectComponent;
    use bevy_ecs::prelude::ReflectResource;

    use bevy_ecs::{component::Component, reflect::AppTypeRegistry, resource::Resource};
    use bevy_reflect::prelude::ReflectDefault;
    use bevy_reflect::{PartialReflect, Reflect, ReflectDeserialize, ReflectSerialize};

    #[test]
    fn reflect_export_struct() {
        #[derive(Reflect, Resource, Default, Deserialize, Serialize)]
        #[reflect(Resource, Default, Serialize, Deserialize)]
        struct Foo {
            a: f32,
            b: Option<f32>,
        }

        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<Foo>();
        }
        let type_registry = atr.read();
        let foo_registration = type_registry
            .get(TypeId::of::<Foo>())
            .expect("SHOULD BE REGISTERED")
            .clone();
        let (_, schema) = export_type(
            &foo_registration,
            &SchemaTypesMetadata::default(),
            &Components::default(),
            &type_registry,
        );

        assert!(
            schema.reflect_types.contains(&"Resource".to_owned()),
            "Should be a resource"
        );
        let _ = schema.properties.get("a").expect("Missing `a` field");
        let _ = schema.properties.get("b").expect("Missing `b` field");
        assert!(
            schema.required.contains(&"a".to_owned()),
            "Field a should be required"
        );
        assert!(
            !schema.required.contains(&"b".to_owned()),
            "Field b should not be required"
        );
    }

    #[test]
    fn reflect_export_enum() {
        #[derive(Reflect, Component, Default, Deserialize, Serialize)]
        #[reflect(Component, Default, Serialize, Deserialize)]
        enum EnumComponent {
            ValueOne(i32),
            ValueTwo {
                test: i32,
            },
            #[default]
            NoValue,
        }

        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<EnumComponent>();
        }
        let type_registry = atr.read();
        let foo_registration = type_registry
            .get(TypeId::of::<EnumComponent>())
            .expect("SHOULD BE REGISTERED")
            .clone();
        let (_, schema) = export_type(
            &foo_registration,
            &SchemaTypesMetadata::default(),
            &Components::default(),
            &type_registry,
        );
        assert!(
            schema.reflect_types.contains(&"Component".to_owned()),
            "Should be a component"
        );
        assert!(
            !schema.reflect_types.contains(&"Resource".to_owned()),
            "Should not be a resource"
        );
        assert!(schema.properties.is_empty(), "Should not have any field");
        assert!(schema.one_of.len() == 3, "Should have 3 possible schemas");
    }

    #[test]
    fn reflect_export_struct_without_reflect_types() {
        #[derive(Reflect, Component, Default, Deserialize, Serialize)]
        enum EnumComponent {
            ValueOne(i32),
            ValueTwo {
                test: i32,
            },
            #[default]
            NoValue,
        }

        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<EnumComponent>();
        }
        let type_registry = atr.read();
        let foo_registration = type_registry
            .get(TypeId::of::<EnumComponent>())
            .expect("SHOULD BE REGISTERED")
            .clone();
        let (_, schema) = export_type(
            &foo_registration,
            &SchemaTypesMetadata::default(),
            &Components::default(),
            &type_registry,
        );
        assert!(
            !schema.reflect_types.contains(&"Component".to_owned()),
            "Should not be a component"
        );
        assert!(
            !schema.reflect_types.contains(&"Resource".to_owned()),
            "Should not be a resource"
        );
        assert!(schema.properties.is_empty(), "Should not have any field");
        assert!(schema.one_of.len() == 3, "Should have 3 possible schemas");
    }

    #[test]
    fn reflect_struct_with_custom_type_data() {
        #[derive(Reflect, Default, Deserialize, Serialize)]
        #[reflect(Default)]
        enum EnumComponent {
            ValueOne(i32),
            ValueTwo {
                test: i32,
            },
            #[default]
            NoValue,
        }

        #[derive(Clone)]
        pub struct ReflectCustomData;

        impl<T: Reflect> CreateTypeData<T> for ReflectCustomData {
            fn create_type_data(_input: ()) -> Self {
                ReflectCustomData
            }
        }

        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<EnumComponent>();
            register.register_type_data::<EnumComponent, ReflectCustomData>();
        }
        let mut metadata = SchemaTypesMetadata::default();
        metadata.map_type_data::<ReflectCustomData>("CustomData");
        let type_registry = atr.read();
        let foo_registration = type_registry
            .get(TypeId::of::<EnumComponent>())
            .expect("SHOULD BE REGISTERED")
            .clone();
        let (_, schema) = export_type(
            &foo_registration,
            &metadata,
            &Components::default(),
            &type_registry,
        );
        assert!(
            !metadata.has_type_data::<ReflectComponent>(&schema.reflect_types),
            "Should not be a component"
        );
        assert!(
            !metadata.has_type_data::<ReflectResource>(&schema.reflect_types),
            "Should not be a resource"
        );
        assert!(
            metadata.has_type_data::<ReflectDefault>(&schema.reflect_types),
            "Should have default"
        );
        assert!(
            metadata.has_type_data::<ReflectCustomData>(&schema.reflect_types),
            "Should have CustomData"
        );
        assert!(schema.properties.is_empty(), "Should not have any field");
        assert!(schema.one_of.len() == 3, "Should have 3 possible schemas");
    }

    #[test]
    fn reflect_export_tuple_struct() {
        #[derive(Reflect, Component, Default, Deserialize, Serialize)]
        #[reflect(Component, Default, Serialize, Deserialize)]
        struct TupleStructType(usize, i32);

        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<TupleStructType>();
        }
        let type_registry = atr.read();
        let foo_registration = type_registry
            .get(TypeId::of::<TupleStructType>())
            .expect("SHOULD BE REGISTERED")
            .clone();
        let (_, schema) = export_type(
            &foo_registration,
            &SchemaTypesMetadata::default(),
            &Components::default(),
            &type_registry,
        );
        assert!(
            schema.reflect_types.contains(&"Component".to_owned()),
            "Should be a component"
        );
        assert!(
            !schema.reflect_types.contains(&"Resource".to_owned()),
            "Should not be a resource"
        );
        assert!(schema.properties.is_empty(), "Should not have any field");
        assert!(schema.prefix_items.len() == 2, "Should have 2 prefix items");
    }

    #[test]
    fn reflect_export_serialization_check() {
        #[derive(Reflect, Resource, Default, Deserialize, Serialize)]
        #[reflect(Resource, Default)]
        struct Foo {
            a: f32,
        }

        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<Foo>();
        }
        let type_registry = atr.read();
        let foo_registration = type_registry
            .get(TypeId::of::<Foo>())
            .expect("SHOULD BE REGISTERED")
            .clone();
        let (_, schema) = export_type(
            &foo_registration,
            &SchemaTypesMetadata::default(),
            &Components::default(),
            &type_registry,
        );
        let schema_as_value = serde_json::to_value(&schema).expect("Should serialize");
        let value = json!({
          "shortPath": "Foo",
          "typePath": "bevy_remote::schemas::json_schema::tests::Foo",
          "modulePath": "bevy_remote::schemas::json_schema::tests",
          "crateName": "bevy_remote",
          "reflectTypes": [
            "Component",
            "Resource",
            "Default",
          ],
          "kind": "Struct",
          "type": "object",
          "additionalProperties": false,
          "properties": {
            "a": {
              "$ref": "#/f32",
              "fieldIndex": 0
            },
          },
          "required": [
            "a"
          ],
          "default": {
            "a": 0.0
          }
        });
        assert_normalized_values(schema_as_value, value);
    }

    #[test]
    fn reflect_export_field_index() {
        #[derive(Reflect)]
        struct Unordered {
            c: u8,
            a: f32,
            b: Option<u8>,
        }

        #[derive(Reflect)]
        enum UnorderedEnum {
            Variant { y: u8, x: i32 },
        }

        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<Unordered>();
            register.register::<UnorderedEnum>();
        }
        let type_registry = atr.read();
        let export = |type_id| {
            export_type(
                type_registry.get(type_id).expect("SHOULD BE REGISTERED"),
                &SchemaTypesMetadata::default(),
                &Components::default(),
                &type_registry,
            )
            .1
        };

        let schema = export(TypeId::of::<Unordered>());
        assert_eq!(
            schema.properties.keys().collect::<Vec<_>>(),
            ["a", "b", "c"],
            "Properties should be sorted by name"
        );
        assert_eq!(schema.properties["c"]["fieldIndex"], 0);
        assert_eq!(schema.properties["a"]["fieldIndex"], 1);
        assert_eq!(schema.properties["b"]["fieldIndex"], 2);
        assert_eq!(
            schema.properties["a"]["$ref"], "#/f32",
            "Properties should reference their field type"
        );
        assert!(schema.properties["a"].get("typePath").is_none());

        let schema = export(TypeId::of::<UnorderedEnum>());
        let properties = &schema.one_of[0]["properties"]["Variant"]["properties"];
        assert_eq!(properties["y"]["fieldIndex"], 0);
        assert_eq!(properties["x"]["fieldIndex"], 1);
        assert_eq!(properties["x"]["$ref"], "#/i32");
    }

    fn export_and_round_trip_default<T>(type_registry: &TypeRegistry) -> T
    where
        T: Reflect + bevy_reflect::FromReflect + bevy_reflect::Typed,
    {
        let registration = type_registry
            .get(TypeId::of::<T>())
            .expect("SHOULD BE REGISTERED");
        let (_, schema) = export_type(
            registration,
            &SchemaTypesMetadata::default(),
            &Components::default(),
            type_registry,
        );
        let default = schema.default.expect("Should export a default");
        let deserializer =
            bevy_reflect::serde::TypedReflectDeserializer::new(registration, type_registry);
        let value = serde::de::DeserializeSeed::deserialize(deserializer, default)
            .expect("Default should deserialize");
        T::from_reflect(value.as_ref()).expect("Should convert from reflect")
    }

    #[test]
    fn reflect_export_struct_default_round_trips() {
        #[derive(Reflect, Debug, PartialEq)]
        #[reflect(Default)]
        struct Foo {
            a: f32,
            b: Option<i32>,
            c: String,
        }

        impl Default for Foo {
            fn default() -> Self {
                Self {
                    a: 1.5,
                    b: Some(3),
                    c: "hello".to_owned(),
                }
            }
        }

        let atr = AppTypeRegistry::default();
        atr.write().register::<Foo>();
        let type_registry = atr.read();
        let value = export_and_round_trip_default::<Foo>(&type_registry);
        assert_eq!(value, Foo::default());
    }

    #[test]
    fn reflect_export_enum_struct_variant_default_round_trips() {
        #[derive(Reflect, Debug, PartialEq)]
        #[reflect(Default)]
        enum Shape {
            Point,
            Rect { width: f32, height: f32 },
        }

        impl Default for Shape {
            fn default() -> Self {
                Self::Rect {
                    width: 2.0,
                    height: 4.0,
                }
            }
        }

        let atr = AppTypeRegistry::default();
        atr.write().register::<Shape>();
        let type_registry = atr.read();
        let value = export_and_round_trip_default::<Shape>(&type_registry);
        assert_eq!(value, Shape::default());
    }

    #[test]
    fn reflect_export_without_default_has_no_default_key() {
        #[derive(Reflect)]
        struct NoDefault {
            a: f32,
        }

        let atr = AppTypeRegistry::default();
        atr.write().register::<NoDefault>();
        let type_registry = atr.read();
        let registration = type_registry
            .get(TypeId::of::<NoDefault>())
            .expect("SHOULD BE REGISTERED");
        let (_, schema) = export_type(
            registration,
            &SchemaTypesMetadata::default(),
            &Components::default(),
            &type_registry,
        );
        assert!(schema.default.is_none());
        let schema_as_value = serde_json::to_value(&schema).expect("Should serialize");
        assert!(schema_as_value.get("default").is_none());
    }

    #[test]
    fn reflect_export_annotations() {
        use bevy_annotations::{DisplayName, ReadOnly, Step, Tooltip, Unit, ValueRange};

        #[derive(Reflect)]
        #[reflect(@Tooltip("Movement settings"), @DisplayName("Movement"), @ReadOnly)]
        struct Movement {
            #[reflect(@ValueRange(0.0..=20.0), @Step(0.5), @Unit("m/s"))]
            speed: f32,
            #[reflect(@DisplayName("Jump Height"), @Tooltip("Peak height of a jump"))]
            jump: f32,
            #[reflect(@ReadOnly)]
            grounded: bool,
            #[reflect(@0.0..=1.0)]
            friction: f32,
            plain: f32,
        }

        #[derive(Reflect)]
        struct Meters(#[reflect(@Unit("m"))] f32, u8);

        #[derive(Reflect)]
        enum Shape {
            #[reflect(@Tooltip("A circle"))]
            Circle {
                #[reflect(@ValueRange(0.0..=10.0))]
                radius: f32,
                plain: f32,
            },
            Point,
        }

        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<Movement>();
            register.register::<Meters>();
            register.register::<Shape>();
        }
        let type_registry = atr.read();
        let export = |type_id| {
            let schema = export_type(
                type_registry.get(type_id).expect("SHOULD BE REGISTERED"),
                &SchemaTypesMetadata::default(),
                &Components::default(),
                &type_registry,
            )
            .1;
            serde_json::to_value(&schema).expect("Should serialize")
        };

        let typed = export_type(
            type_registry
                .get(TypeId::of::<Movement>())
                .expect("SHOULD BE REGISTERED"),
            &SchemaTypesMetadata::default(),
            &Components::default(),
            &type_registry,
        )
        .1;
        let round_trip: JsonSchemaBevyType =
            serde_json::from_value(serde_json::to_value(&typed).expect("Should serialize"))
                .expect("Should deserialize");
        assert_eq!(round_trip, typed);

        let schema = export(TypeId::of::<Movement>());
        assert_eq!(
            schema["annotations"],
            json!({ "tooltip": "Movement settings", "displayName": "Movement", "readOnly": true })
        );
        let properties = &schema["properties"];
        assert_eq!(
            properties["speed"]["annotations"],
            json!({ "range": [0.0, 20.0], "step": 0.5, "unit": "m/s" })
        );
        assert_eq!(
            properties["jump"]["annotations"],
            json!({ "displayName": "Jump Height", "tooltip": "Peak height of a jump" })
        );
        assert_eq!(
            properties["grounded"]["annotations"],
            json!({ "readOnly": true })
        );
        assert_eq!(
            properties["friction"]["annotations"],
            json!({ "range": [0.0, 1.0] })
        );
        assert!(properties["plain"].get("annotations").is_none());

        let schema = export(TypeId::of::<Meters>());
        assert!(schema.get("annotations").is_none());
        assert_eq!(
            schema["prefixItems"][0]["annotations"],
            json!({ "unit": "m" })
        );
        assert!(schema["prefixItems"][1].get("annotations").is_none());

        let schema = export(TypeId::of::<Shape>());
        let circle = &schema["oneOf"][0];
        assert_eq!(circle["annotations"], json!({ "tooltip": "A circle" }));
        let fields = &circle["properties"]["Circle"]["properties"];
        assert_eq!(
            fields["radius"]["annotations"],
            json!({ "range": [0.0, 10.0] })
        );
        assert!(fields["plain"].get("annotations").is_none());
        assert!(schema["oneOf"][1].get("annotations").is_none());
    }

    fn export_schema_and_value<T: Reflect + GetTypeRegistration>(value: &T) -> (Value, Value) {
        let atr = AppTypeRegistry::default();
        atr.write().register::<T>();
        let type_registry = atr.read();
        let (_, schema) = export_type(
            type_registry
                .get(TypeId::of::<T>())
                .expect("SHOULD BE REGISTERED"),
            &SchemaTypesMetadata::default(),
            &Components::default(),
            &type_registry,
        );
        let schema = serde_json::to_value(&schema).expect("Should serialize");
        let value = serde_json::to_value(TypedReflectSerializer::new(value, &type_registry))
            .expect("Value should serialize");
        (schema, value)
    }

    #[test]
    fn reflect_export_newtype_struct_refs_inner_type() {
        #[derive(Reflect, Default)]
        #[reflect(Default)]
        struct Cube(#[reflect(@Unit("m"))] f32);

        let (schema, value) = export_schema_and_value(&Cube(1.5));
        assert_eq!(value, json!(1.5));
        assert_normalized_values(
            schema,
            json!({
                "shortPath": "Cube",
                "typePath": "bevy_remote::schemas::json_schema::tests::Cube",
                "modulePath": "bevy_remote::schemas::json_schema::tests",
                "crateName": "bevy_remote",
                "reflectTypes": ["Default"],
                "kind": "TupleStruct",
                "$ref": "#/f32",
                "default": 0.0,
                "annotations": { "unit": "m" },
            }),
        );
    }

    #[test]
    fn reflect_export_skipped_field_tuple_struct_stays_array() {
        #[derive(Reflect)]
        struct Wrapper(#[reflect(skip_serializing)] f32);

        let (schema, value) = export_schema_and_value(&Wrapper(1.5));
        assert_eq!(value, json!([]));
        assert_eq!(schema["type"], "array");
        assert!(schema.get("$ref").is_none());
    }

    #[test]
    fn reflect_export_skipped_tuple_struct_field_is_not_an_item() {
        #[derive(Reflect)]
        struct Wrapper(#[reflect(skip_serializing)] u8, f32);

        let (schema, value) = export_schema_and_value(&Wrapper(2, 1.5));
        assert_eq!(value, json!([1.5]));
        assert_eq!(schema["prefixItems"], json!([{ "$ref": "#/f32" }]));
    }

    #[test]
    fn reflect_export_skipped_struct_field_is_not_required() {
        #[derive(Reflect)]
        struct Settings {
            kept: f32,
            #[reflect(skip_serializing)]
            skipped: f32,
        }

        let (schema, value) = export_schema_and_value(&Settings {
            kept: 1.0,
            skipped: 2.0,
        });
        assert_eq!(value, json!({ "kept": 1.0 }));
        assert_eq!(schema["required"], json!(["kept"]));
        assert!(schema["properties"].get("skipped").is_none());
    }

    #[test]
    fn reflect_export_newtype_keeps_type_and_field_annotations() {
        #[derive(Reflect)]
        #[reflect(@Tooltip("Top speed"))]
        struct Speed(#[reflect(@0.0..=10.0, @Unit("m/s"))] f32);

        let (schema, _) = export_schema_and_value(&Speed(1.0));
        assert_eq!(
            schema["annotations"],
            json!({ "tooltip": "Top speed", "range": [0.0, 10.0], "unit": "m/s" })
        );
        assert_eq!(schema["minimum"], 0.0);
    }

    #[test]
    fn reflect_export_option_some_excludes_null() {
        use bevy_ecs::entity::Entity;

        let (schema, value) = export_schema_and_value(&None::<Entity>);
        assert_eq!(value, Value::Null);
        assert_eq!(schema["oneOf"][1]["not"], json!({ "type": "null" }));
    }

    #[test]
    fn reflect_export_enum_variants_match_serialized_values() {
        #[derive(Reflect)]
        enum Shape {
            Circle(#[reflect(@Unit("m"))] f32),
            Pair(f32, f32),
            Rect { w: f32 },
            Point,
        }

        let path = "bevy_remote::schemas::json_schema::tests::Shape";
        let (schema, circle) = export_schema_and_value(&Shape::Circle(2.0));
        let (_, pair) = export_schema_and_value(&Shape::Pair(1.0, 2.0));
        let (_, rect) = export_schema_and_value(&Shape::Rect { w: 3.0 });
        let (_, point) = export_schema_and_value(&Shape::Point);
        assert_eq!(circle, json!({ "Circle": 2.0 }));
        assert_eq!(pair, json!({ "Pair": [1.0, 2.0] }));
        assert_eq!(rect, json!({ "Rect": { "w": 3.0 } }));
        assert_eq!(point, json!("Point"));
        assert!(schema.get("type").is_none());
        assert_eq!(
            schema["oneOf"],
            json!([
                {
                    "type": "object",
                    "properties": {
                        "Circle": { "$ref": "#/f32", "annotations": { "unit": "m" } },
                    },
                    "required": ["Circle"],
                    "additionalProperties": false,
                    "kind": "Tuple",
                    "typePath": format!("{path}::Circle"),
                    "shortPath": "Circle",
                },
                {
                    "type": "object",
                    "properties": {
                        "Pair": {
                            "type": "array",
                            "prefixItems": [{ "$ref": "#/f32" }, { "$ref": "#/f32" }],
                            "items": false,
                            "minItems": 2,
                            "maxItems": 2,
                        },
                    },
                    "required": ["Pair"],
                    "additionalProperties": false,
                    "kind": "Tuple",
                    "typePath": format!("{path}::Pair"),
                    "shortPath": "Pair",
                },
                {
                    "type": "object",
                    "properties": {
                        "Rect": {
                            "type": "object",
                            "properties": { "w": { "$ref": "#/f32", "fieldIndex": 0 } },
                            "required": ["w"],
                            "additionalProperties": false,
                        },
                    },
                    "required": ["Rect"],
                    "additionalProperties": false,
                    "kind": "Struct",
                    "typePath": format!("{path}::Rect"),
                    "shortPath": "Rect",
                },
                {
                    "const": "Point",
                    "typePath": format!("{path}::Point"),
                    "shortPath": "Point",
                },
            ])
        );
    }

    #[test]
    fn reflect_export_unit_enum_is_string_constants() {
        #[derive(Reflect)]
        enum Mode {
            On,
            Off,
        }

        let (schema, value) = export_schema_and_value(&Mode::Off);
        assert_eq!(value, json!("Off"));
        assert_eq!(schema["type"], "string");
        assert_eq!(
            schema["oneOf"],
            json!([
                {
                    "const": "On",
                    "typePath": "bevy_remote::schemas::json_schema::tests::Mode::On",
                    "shortPath": "On",
                },
                {
                    "const": "Off",
                    "typePath": "bevy_remote::schemas::json_schema::tests::Mode::Off",
                    "shortPath": "Off",
                },
            ])
        );
    }

    #[test]
    fn reflect_export_standard_json_types() {
        #[derive(Reflect, Clone, Serialize)]
        #[reflect(opaque, Clone, Serialize)]
        struct Custom(u8, u8);

        let (schema, value) = export_schema_and_value(&7_u8);
        assert_eq!(value, json!(7));
        assert_eq!(schema["type"], "integer");
        assert_eq!(schema["minimum"], 0.0);
        let (schema, _) = export_schema_and_value(&-7_i32);
        assert_eq!(schema["type"], "integer");
        assert!(schema.get("minimum").is_none());
        let (schema, _) = export_schema_and_value(&1.5_f32);
        assert_eq!(schema["type"], "number");
        let (schema, _) = export_schema_and_value(&true);
        assert_eq!(schema["type"], "boolean");
        let (schema, _) = export_schema_and_value(&String::from("text"));
        assert_eq!(schema["type"], "string");

        let set = bevy_platform::collections::HashSet::from([1_u32]);
        let (schema, value) = export_schema_and_value(&set);
        assert_eq!(value, json!([1]));
        assert_eq!(schema["type"], "array");
        assert_eq!(schema["uniqueItems"], true);
        assert_eq!(schema["kind"], "Set");
        assert_eq!(schema["items"], json!({ "$ref": "#/u32" }));

        let (schema, value) = export_schema_and_value(&Custom(1, 2));
        assert_eq!(value, json!([1, 2]));
        assert_eq!(schema["kind"], "Value");
        assert!(schema.get("type").is_none());
        assert_eq!(schema["rawJson"], true);
        let (schema, _) = export_schema_and_value(&7_u8);
        assert!(schema.get("rawJson").is_none());
    }

    #[test]
    fn reflect_export_map_is_object_of_value_type() {
        let map = bevy_platform::collections::HashMap::from([(String::from("a"), 1.5_f32)]);
        let (schema, value) = export_schema_and_value(&map);
        assert_eq!(value, json!({ "a": 1.5 }));
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["kind"], "Map");
        assert_eq!(schema["additionalProperties"], json!({ "$ref": "#/f32" }));
        assert_eq!(
            schema["keyType"],
            json!({ "$ref": "#/alloc::string::String" })
        );
        assert_eq!(schema["valueType"], json!({ "$ref": "#/f32" }));
    }

    #[test]
    fn reflect_export_option_is_inner_type_or_null() {
        let (schema, none) = export_schema_and_value(&None::<f32>);
        let (_, some) = export_schema_and_value(&Some(3.0_f32));
        assert_eq!(none, Value::Null);
        assert_eq!(some, json!(3.0));
        assert_eq!(schema["kind"], "Enum");
        assert!(schema.get("type").is_none());
        assert_eq!(
            schema["oneOf"],
            json!([
                {
                    "type": "null",
                    "typePath": "core::option::Option<f32>::None",
                    "shortPath": "None",
                },
                {
                    "$ref": "#/f32",
                    "not": { "type": "null" },
                    "kind": "Tuple",
                    "typePath": "core::option::Option<f32>::Some",
                    "shortPath": "Some",
                },
            ])
        );
    }

    #[test]
    fn reflect_export_standard_keywords() {
        use bevy_annotations::{DisplayName, ReadOnly, Step, Tooltip};

        #[derive(Reflect)]
        #[reflect(@Tooltip("Movement settings"), @DisplayName("Movement"), @ReadOnly)]
        struct Movement {
            #[reflect(@0.0..=20.0, @Step(0.5), @DisplayName("Speed"), @Tooltip("Top speed"))]
            speed: f32,
            plain: f32,
        }

        #[derive(Reflect)]
        struct Pair(#[reflect(@ReadOnly, @0..=9)] u8, f32);

        let (schema, _) = export_schema_and_value(&Movement {
            speed: 1.0,
            plain: 0.0,
        });
        assert_eq!(schema["title"], "Movement");
        assert_eq!(schema["description"], "Movement settings");
        assert_eq!(schema["readOnly"], true);
        let speed = &schema["properties"]["speed"];
        assert_eq!(speed["minimum"], 0.0);
        assert_eq!(speed["maximum"], 20.0);
        assert_eq!(speed["title"], "Speed");
        assert_eq!(speed["description"], "Top speed");
        assert!(speed.get("multipleOf").is_none());
        assert_eq!(speed["annotations"]["step"], 0.5);
        let plain = schema["properties"]["plain"].as_object().unwrap();
        assert_eq!(plain.keys().collect::<Vec<_>>(), ["$ref", "fieldIndex"]);

        let (schema, _) = export_schema_and_value(&Pair(1, 2.0));
        let first = &schema["prefixItems"][0];
        assert_eq!(first["readOnly"], true);
        assert_eq!(first["minimum"], 0.0);
        assert_eq!(first["maximum"], 9.0);
        assert!(schema["prefixItems"][1].get("readOnly").is_none());
    }

    #[test]
    fn reflect_export_fixed_lengths() {
        #[derive(Reflect)]
        struct Pair(f32, u8, bool);

        #[derive(Reflect)]
        struct Skipped(f32, #[reflect(skip_serializing)] u8, bool);

        #[derive(Reflect)]
        enum Event {
            Moved(f32, f32),
        }

        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<(f32, u8)>();
            register.register::<Pair>();
            register.register::<Skipped>();
            register.register::<Event>();
            register.register::<[f32; 4]>();
        }
        let type_registry = atr.read();
        let export = |type_id| {
            let schema = export_type(
                type_registry.get(type_id).expect("SHOULD BE REGISTERED"),
                &SchemaTypesMetadata::default(),
                &Components::default(),
                &type_registry,
            )
            .1;
            serde_json::to_value(&schema).expect("Should serialize")
        };
        let serialize = |value: &dyn PartialReflect| {
            serde_json::to_value(TypedReflectSerializer::new(value, &type_registry))
                .expect("Should serialize")
        };

        let schema = export(TypeId::of::<(f32, u8)>());
        assert_eq!(schema["minItems"], 2);
        assert_eq!(schema["maxItems"], 2);
        assert_eq!(schema["items"], false);
        assert_eq!(serialize(&(1.0f32, 2u8)).as_array().map(Vec::len), Some(2));

        let schema = export(TypeId::of::<Pair>());
        assert_eq!(schema["minItems"], 3);
        assert_eq!(schema["maxItems"], 3);
        assert_eq!(schema["items"], false);
        assert_eq!(
            serialize(&Pair(1.0, 2, true)).as_array().map(Vec::len),
            Some(3)
        );

        let schema = export(TypeId::of::<Skipped>());
        assert_eq!(schema["minItems"], 2);
        assert_eq!(schema["maxItems"], 2);
        assert_eq!(
            serialize(&Skipped(1.0, 2, true)).as_array().map(Vec::len),
            Some(2)
        );

        let schema = export(TypeId::of::<Event>());
        let moved = &schema["oneOf"][0]["properties"]["Moved"];
        assert_eq!(moved["minItems"], 2);
        assert_eq!(moved["maxItems"], 2);
        assert_eq!(moved["items"], false);
        assert_eq!(
            serialize(&Event::Moved(1.0, 2.0))["Moved"]
                .as_array()
                .map(Vec::len),
            Some(2)
        );

        let schema = export(TypeId::of::<[f32; 4]>());
        assert_eq!(schema["minItems"], 4);
        assert_eq!(schema["maxItems"], 4);
        assert_eq!(serialize(&[0.0f32; 4]).as_array().map(Vec::len), Some(4));
    }

    /// Checks `value` against the subset of JSON Schema that `registry.schema` emits.
    fn validate(schema: &Value, value: &Value, schemas: &Map<String, Value>) -> Result<(), String> {
        if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
            let path = reference.trim_start_matches("#/");
            let target = schemas.get(path).ok_or(format!("unknown {reference}"))?;
            validate(target, value, schemas)?;
        }
        if let Some(ty) = schema.get("type").and_then(Value::as_str) {
            let matches = match ty {
                "string" => value.is_string(),
                "number" => value.is_number(),
                "integer" => value.is_i64() || value.is_u64(),
                "boolean" => value.is_boolean(),
                "array" => value.is_array(),
                "object" => value.is_object(),
                "null" => value.is_null(),
                _ => false,
            };
            if !matches {
                return Err(format!("{value} is not of type {ty}"));
            }
        }
        if let (Some(minimum), Some(number)) = (
            schema.get("minimum").and_then(Value::as_f64),
            value.as_f64(),
        ) && number < minimum
        {
            return Err(format!("{value} is below {minimum}"));
        }
        if let Some(constant) = schema.get("const")
            && value != constant
        {
            return Err(format!("{value} is not {constant}"));
        }
        if let Some(not) = schema.get("not")
            && validate(not, value, schemas).is_ok()
        {
            return Err(format!("{value} matches {not}"));
        }
        if let Value::Array(items) = value {
            let len = items.len() as u64;
            if schema["minItems"].as_u64().is_some_and(|min| len < min)
                || schema["maxItems"].as_u64().is_some_and(|max| len > max)
            {
                return Err(format!("{value} has the wrong length"));
            }
            let prefix = schema["prefixItems"].as_array();
            for (index, item) in items.iter().enumerate() {
                match (
                    prefix.and_then(|prefix| prefix.get(index)),
                    schema.get("items"),
                ) {
                    (None, Some(Value::Bool(false))) => {
                        return Err(format!("{value} has too many items"));
                    }
                    (Some(item_schema), _) | (None, Some(item_schema)) => {
                        validate(item_schema, item, schemas)?;
                    }
                    (None, None) => {}
                }
            }
        }
        if let Value::Object(fields) = value {
            for name in schema["required"].as_array().into_iter().flatten() {
                if !fields.contains_key(name.as_str().unwrap_or_default()) {
                    return Err(format!("{value} is missing {name}"));
                }
            }
            for (name, field) in fields {
                match schema["properties"].get(name) {
                    Some(property) => validate(property, field, schemas)?,
                    None if schema.get("additionalProperties") == Some(&Value::Bool(false)) => {
                        return Err(format!("{value} has unexpected {name}"));
                    }
                    None => {}
                }
            }
        }
        Ok(())
    }

    fn wire_registry() -> AppTypeRegistry {
        let atr = AppTypeRegistry::default();
        {
            let mut registry = atr.write();
            register_brp_json_schemas(&mut registry);
            registry.register::<u32>();
            registry.register::<u64>();
        }
        atr
    }

    fn export_all(registry: &TypeRegistry) -> Map<String, Value> {
        registry
            .iter()
            .map(|reg| {
                let (path, schema) = export_type(
                    reg,
                    &SchemaTypesMetadata::default(),
                    &Components::default(),
                    registry,
                );
                (
                    path.into_owned(),
                    serde_json::to_value(schema).expect("Should serialize"),
                )
            })
            .collect()
    }

    #[test]
    fn brp_json_schema_vec3_describes_array() {
        let atr = wire_registry();
        let registry = atr.read();
        let schemas = export_all(&registry);
        let schema = &schemas["glam::Vec3"];
        assert_eq!(schema["type"], "array");
        assert_eq!(schema["kind"], "Struct");
        assert_eq!(
            schema["prefixItems"],
            json!([
                { "$ref": "#/f32", "title": "x" },
                { "$ref": "#/f32", "title": "y" },
                { "$ref": "#/f32", "title": "z" },
            ])
        );
        assert_eq!(schema["items"], false);
        assert_eq!(schema["minItems"], 3);
        assert_eq!(schema["maxItems"], 3);
        assert_eq!(schema["default"], json!([0.0, 0.0, 0.0]));
        assert!(schema.get("properties").is_none());
        let reflected = &schema["reflectSchema"];
        assert_eq!(reflected["type"], "object");
        assert_eq!(reflected["required"], json!(["x", "y", "z"]));
        assert_eq!(reflected["properties"]["y"]["fieldIndex"], 1);
        assert!(schema.get("rawJson").is_none());
        assert_eq!(
            schemas["bevy_ecs::name::Name"]["reflectSchema"]["$ref"],
            "#/bevy_ecs::name::HashedStr"
        );
        assert_eq!(schemas["bevy_ecs::name::Name"]["type"], "string");
        assert!(schemas["core::time::Duration"]
            .get("reflectSchema")
            .is_none());
    }

    #[test]
    fn brp_json_schema_builtins_match_serialized_values() {
        use alloc::borrow::Cow;
        use bevy_ecs::{entity::Entity, name::Name};
        use bevy_math::{Affine2, Affine3A, BVec3A, DMat3, Mat4, Quat, Vec3};
        use bevy_reflect::{serde::TypedReflectDeserializer, GetPath, PartialReflect};
        use core::num::{
            NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU128,
            NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
        };
        use serde::de::DeserializeSeed;

        let atr = wire_registry();
        let registry = atr.read();
        let schemas = export_all(&registry);
        let serialize = |value: &dyn PartialReflect| {
            serde_json::to_value(TypedReflectSerializer::new(value, &registry))
                .expect("Should serialize")
        };
        let check = |value: &dyn PartialReflect| {
            let path = value.reflect_type_path();
            let serialized = serialize(value);
            validate(&schemas[path], &serialized, &schemas)
                .unwrap_or_else(|error| panic!("{path}: {error}"));
            assert!(validate(&schemas[path], &json!({ "unexpected": [] }), &schemas).is_err());
        };

        let mut values: Vec<Box<dyn PartialReflect>> = vec![
            Box::new(Vec3::new(1.0, 2.0, 3.0)),
            Box::new(Quat::from_xyzw(0.0, 0.5, 0.5, 0.5)),
            Box::new(Mat4::from_cols_array(&core::array::from_fn(|i| i as f32))),
            Box::new(DMat3::from_cols_array(&core::array::from_fn(|i| i as f64))),
            Box::new(Affine2::from_cols_array(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0])),
            Box::new(Affine3A::from_cols_array(&core::array::from_fn(|i| {
                i as f32
            }))),
            Box::new(BVec3A::new(true, false, true)),
            Box::new(core::time::Duration::new(5, 7)),
            Box::new(Entity::from_raw_u32(3).unwrap()),
            Box::new(Entity::PLACEHOLDER),
            Box::new(Name::new("cube")),
            Box::new(Cow::<'static, str>::Borrowed("text")),
            Box::new(std::path::PathBuf::from("assets/cube.glb")),
            Box::new(core::net::SocketAddr::from(([127, 0, 0, 1], 15702))),
            Box::new(NonZeroU8::new(3).unwrap()),
            Box::new(NonZeroU16::new(3).unwrap()),
            Box::new(NonZeroU32::new(3).unwrap()),
            Box::new(NonZeroU64::new(u64::MAX).unwrap()),
            Box::new(NonZeroU128::new(3).unwrap()),
            Box::new(NonZeroUsize::new(3).unwrap()),
            Box::new(NonZeroI8::new(-4).unwrap()),
            Box::new(NonZeroI16::new(-4).unwrap()),
            Box::new(NonZeroI32::new(-4).unwrap()),
            Box::new(NonZeroI64::new(-4).unwrap()),
            Box::new(NonZeroI128::new(-4).unwrap()),
            Box::new(NonZeroIsize::new(-4).unwrap()),
        ];
        #[cfg(feature = "bevy_asset")]
        {
            use bevy_asset::uuid::{NonNilUuid, Uuid};
            values.push(Box::new(bevy_asset::AssetPath::from("textures/cube.png")));
            values.push(Box::new(Uuid::from_u128(7)));
            values.push(Box::new(NonNilUuid::new(Uuid::from_u128(7)).unwrap()));
        }
        assert_eq!(serialize(values[0].as_ref()), json!([1.0, 2.0, 3.0]));
        assert_eq!(serialize(values[1].as_ref()), json!([0.0, 0.5, 0.5, 0.5]));
        assert_eq!(
            serialize(values[7].as_ref()),
            json!({ "secs": 5, "nanos": 7 })
        );
        assert_eq!(serialize(values[8].as_ref()), json!("3v0"));
        assert_eq!(serialize(values[10].as_ref()), json!("cube"));
        let mut checked = Vec::new();
        for value in &values {
            check(value.as_ref());
            checked.push(value.reflect_type_path().to_owned());
        }
        assert!(validate(&schemas["core::num::NonZeroI16"], &json!(0), &schemas).is_err());

        for reg in registry.iter() {
            if reg.data::<ReflectBrpJsonSchema>().is_none() {
                continue;
            }
            let path = reg.type_info().type_path();
            let schema = &schemas[path];
            let Some(prefix_items) = schema["prefixItems"].as_array() else {
                assert!(checked.iter().any(|checked| checked == path), "{path}");
                continue;
            };
            let sample: Vec<Value> = prefix_items
                .iter()
                .enumerate()
                .map(|(index, item)| match item["$ref"].as_str() {
                    Some("#/bool") => json!(index % 2 == 0),
                    Some("#/f32" | "#/f64") => json!(index as f64 + 0.5),
                    _ => json!(index + 1),
                })
                .collect();
            let value = TypedReflectDeserializer::new(reg, &registry)
                .deserialize(Value::Array(sample.clone()))
                .unwrap_or_else(|error| panic!("{path}: {error}"));
            check(value.as_ref());
            let Some(reflected) = value.try_as_reflect() else {
                continue;
            };
            for (item, expected) in prefix_items.iter().zip(&sample) {
                let label = item["title"].as_str().expect("Items should be labeled");
                if let Ok(field) = reflected.reflect_path(label) {
                    assert_eq!(&serialize(field), expected, "{path}.{label}");
                } else {
                    assert_eq!(schema["kind"], "Value", "{path}.{label}");
                }
            }
        }
    }

    #[test]
    fn brp_json_schema_on_user_type() {
        #[derive(Reflect)]
        #[reflect(Serialize, BrpJsonSchema)]
        struct Span {
            min: f32,
            max: f32,
        }

        impl Serialize for Span {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                (self.min, self.max).serialize(serializer)
            }
        }

        impl BrpJsonSchema for Span {
            fn json_schema() -> Map<String, Value> {
                let mut schema = fixed_array::<f32>(vec!["min".into(), "max".into()]);
                schema.insert("description".into(), "A closed range".into());
                schema
            }
        }

        let (schema, value) = export_schema_and_value(&Span { min: 1.0, max: 2.0 });
        assert_eq!(value, json!([1.0, 2.0]));
        assert_eq!(schema["type"], "array");
        assert_eq!(schema["description"], "A closed range");
        assert_eq!(schema["prefixItems"][1]["title"], "max");
        assert_eq!(schema["reflectSchema"]["type"], "object");
        assert_eq!(schema["reflectSchema"]["required"], json!(["min", "max"]));
    }

    #[test]
    fn brp_json_schema_extra_keywords_round_trip() {
        let atr = wire_registry();
        let registry = atr.read();
        let schemas = export_all(&registry);
        let non_zero = &schemas["core::num::NonZeroI32"];
        assert_eq!(non_zero["not"], json!({ "const": 0 }));
        let parsed: JsonSchemaBevyType =
            serde_json::from_value(non_zero.clone()).expect("Should deserialize");
        assert_eq!(parsed.extra["not"], json!({ "const": 0 }));
        assert_eq!(&serde_json::to_value(&parsed).unwrap(), non_zero);
    }

    /// This function exist to avoid false failures due to ordering differences between `serde_json` values.
    fn assert_normalized_values(mut one: Value, mut two: Value) {
        normalize_json(&mut one);
        normalize_json(&mut two);
        assert_eq!(one, two);

        /// Recursively sorts arrays in a `serde_json::Value`
        fn normalize_json(value: &mut Value) {
            match value {
                Value::Array(arr) => {
                    for v in arr.iter_mut() {
                        normalize_json(v);
                    }
                    arr.sort_by_key(ToString::to_string); // Sort by stringified version
                }
                Value::Object(map) => {
                    for (_k, v) in map.iter_mut() {
                        normalize_json(v);
                    }
                }
                _ => {}
            }
        }
    }
}
