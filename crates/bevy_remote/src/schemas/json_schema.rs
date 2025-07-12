//! Module with JSON Schema type for Bevy Registry Types.
//!  It tries to follow this standard: <https://json-schema.org/specification>
use alloc::borrow::Cow;
use bevy_platform::collections::HashMap;
use bevy_reflect::{
    prelude::ReflectDefault, serde::ReflectSerializer, GetTypeRegistration, Reflect, TypeInfo,
    TypeRegistration, TypeRegistry,
};
use core::any::TypeId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::schemas::{
    reflect_info::{
        OptionalInfoReader, SchemaNumber, TypeDefinitionBuilder, TypeReferenceId, TypeReferencePath,
    },
    SchemaTypesMetadata,
};

/// Helper trait for converting `TypeRegistration` to `JsonSchemaBevyType`
pub trait TypeRegistrySchemaReader {
    /// Export type JSON Schema.
    fn export_type_json_schema<T: GetTypeRegistration + 'static>(
        &self,
        extra_info: &SchemaTypesMetadata,
    ) -> Option<JsonSchemaBevyType> {
        self.export_type_json_schema_for_id(TypeId::of::<T>(), extra_info)
    }
    /// Export type JSON Schema.
    fn export_type_json_schema_for_id(
        &self,
        type_id: TypeId,
        extra_info: &SchemaTypesMetadata,
    ) -> Option<JsonSchemaBevyType>;

    /// Try to get default value for type id.
    fn try_get_default_value_for_type_id(&self, type_id: TypeId) -> Option<Value>;
}

impl TypeRegistrySchemaReader for TypeRegistry {
    fn export_type_json_schema_for_id(
        &self,
        type_id: TypeId,
        extra_info: &SchemaTypesMetadata,
    ) -> Option<JsonSchemaBevyType> {
        let mut schema = self.build_schema_for_type_id_with_definitions(type_id, extra_info)?;
        schema.schema = Some(SchemaMarker.into());
        schema.default_value = self.try_get_default_value_for_type_id(type_id);

        Some(schema)
    }

    fn try_get_default_value_for_type_id(&self, type_id: TypeId) -> Option<Value> {
        let type_reg = self.get(type_id)?;
        let default_data = type_reg.data::<ReflectDefault>()?;
        let default = default_data.default();
        let serializer = ReflectSerializer::new(&*default, self);
        let value_object = serde_json::to_value(serializer)
            .ok()
            .and_then(|v| v.as_object().cloned())?;
        if value_object.len() == 1 {
            if let Some((_, value)) = value_object.into_iter().next() {
                return Some(value);
            }
        }

        None
    }
}

/// Identifies the JSON Schema version used in the schema.
#[derive(Deserialize, Serialize, Debug, Reflect, PartialEq, Clone)]
pub struct SchemaMarker;

impl SchemaMarker {
    const DEFAULT_SCHEMA: &'static str = "https://json-schema.org/draft/2020-12/schema";
}

impl From<SchemaMarker> for &'static str {
    fn from(_: SchemaMarker) -> Self {
        SchemaMarker::DEFAULT_SCHEMA
    }
}

impl From<SchemaMarker> for Cow<'static, str> {
    fn from(_: SchemaMarker) -> Self {
        Cow::Borrowed(SchemaMarker::DEFAULT_SCHEMA)
    }
}

/// JSON Schema type for Bevy Registry Types
/// It tries to follow this standard: <https://json-schema.org/specification>
///
/// To take the full advantage from info provided by Bevy registry it provides extra fields
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default, Reflect)]
#[serde(rename_all = "camelCase")]
pub struct JsonSchemaBevyType {
    /// Identifies the JSON Schema version used in the schema.
    #[serde(rename = "$schema")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub schema: Option<Cow<'static, str>>,
    /// JSON Schema specific field.
    /// This keyword is used to reference a statically identified schema.
    ///
    /// Serialization format matches RFC 3986, which means that the reference must be a valid URI.
    /// During serialization, all the reserved characters are encoded as percent-encoded sequences.
    #[serde(rename = "$ref")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub ref_type: Option<TypeReferencePath>,
    /// JSON Schema specific field.
    ///
    /// The title keyword is a placeholder for a concise human-readable string
    /// summary of what a schema or any of its subschemas are about.
    ///
    /// Bevy uses this field to provide a field name for the schema.
    /// It can contain dots to indicate nested fields.
    #[serde(skip_serializing_if = "str::is_empty", default)]
    pub title: Cow<'static, str>,
    /// Bevy specific field, short path of the type.
    #[serde(skip_serializing_if = "str::is_empty", default)]
    pub short_path: Cow<'static, str>,
    /// Bevy specific field, full path of the type.
    #[serde(skip_serializing_if = "str::is_empty", default)]
    pub type_path: Cow<'static, str>,
    /// Bevy specific field, path of the module that type is part of.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub module_path: Option<Cow<'static, str>>,
    /// Bevy specific field, name of the crate that type is part of.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub crate_name: Option<Cow<'static, str>>,
    /// Bevy specific field, names of the types that type reflects. Mapping of the names to the data types is provided by [`SchemaTypesMetadata`].
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub reflect_type_data: Vec<Cow<'static, str>>,
    /// Bevy specific field, [`bevy_reflect::TypeInfo`] type mapping.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub kind: Option<SchemaKind>,
    /// JSON Schema specific field.
    /// This keyword is used to reference a constant value.
    #[serde(rename = "const")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[reflect(ignore)]
    pub const_value: Option<Value>,
    /// Bevy specific field, provided when [`SchemaKind`] `kind` field is equal to [`SchemaKind::Map`].
    ///
    /// It contains type info of value of the Map.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[reflect(ignore)]
    pub value_type: Option<Box<JsonSchemaBevyType>>,
    /// Bevy specific field, provided when [`SchemaKind`] `kind` field is equal to [`SchemaKind::Map`].
    ///
    /// It contains type info of key of the Map.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[reflect(ignore)]
    pub key_type: Option<Box<JsonSchemaBevyType>>,
    /// Bevy specific field.
    ///
    /// It is provided when type is serialized as array, but the type is not an array.
    /// It is done to provide additional information about the fields in the schema.
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    #[reflect(ignore)]
    pub rust_fields_info: HashMap<Cow<'static, str>, Box<JsonSchemaBevyType>>,
    /// The type keyword is fundamental to JSON Schema. It specifies the data type for a schema.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[serde(rename = "type")]
    pub schema_type: Option<SchemaTypeVariant>,
    /// The behavior of this keyword depends on the presence and annotation results of "properties"
    /// and "patternProperties" within the same schema object.
    /// Validation with "additionalProperties" applies only to the child
    /// values of instance names that do not appear in the annotation results of either "properties" or "patternProperties".
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub additional_properties: Option<JsonSchemaVariant>,
    /// This keyword restricts object instances to only define properties whose names match the given schema.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[reflect(ignore)]
    pub property_names: Option<Box<JsonSchemaBevyType>>,
    /// The pattern keyword restricts string instances to match the given regular expression.
    /// For now used mostly when limiting property names for Map types.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub pattern: Option<Cow<'static, str>>,
    /// Validation succeeds if, for each name that appears in both the instance and as a name
    /// within this keyword's value, the child instance for that name successfully validates
    /// against the corresponding schema.
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub properties: HashMap<Cow<'static, str>, JsonSchemaVariant>,
    /// An object instance is valid against this keyword if every item in the array is the name of a property in the instance.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub required: Vec<Cow<'static, str>>,
    /// An instance validates successfully against this keyword if it validates successfully against exactly one schema defined by this keyword's value.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    #[reflect(ignore)]
    pub one_of: Vec<Box<JsonSchemaBevyType>>,
    /// Validation succeeds if each element of the instance validates against the schema at the same position, if any. This keyword does not constrain the length of the array. If the array is longer than this keyword's value, this keyword validates only the prefix of matching length.
    ///
    /// This keyword produces an annotation value which is the largest index to which this keyword
    /// applied a subschema. The value MAY be a boolean true if a subschema was applied to every
    /// index of the instance, such as is produced by the "items" keyword.
    /// This annotation affects the behavior of "items" and "unevaluatedItems".
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub prefix_items: Vec<JsonSchemaVariant>,
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
    pub items: Option<JsonSchemaVariant>,
    /// The value of this keyword MUST be a non-negative integer.
    /// An array instance is valid against "minItems" if its size is greater than,
    /// or equal to, the value of this keyword.
    /// Omitting this keyword has the same behavior as a value of 0.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub min_items: Option<u64>,
    /// The value of this keyword MUST be a non-negative integer.
    /// An array instance is valid against "maxItems" if its size is less than,
    /// or equal to, the value of this keyword.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_items: Option<u64>,
    /// The value of "minimum" MUST be a number,
    /// representing an inclusive lower limit for a numeric instance.
    /// If the instance is a number, then this keyword validates only
    /// if the instance is greater than or exactly equal to "minimum".
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub minimum: Option<SchemaNumber>,
    /// The value of "maximum" MUST be a number,
    /// representing an inclusive upper limit for a numeric instance.
    /// If the instance is a number, then this keyword validates only
    /// if the instance is less than or exactly equal to "maximum".
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub maximum: Option<SchemaNumber>,
    /// The value of "exclusiveMinimum" MUST be a number,
    /// representing an exclusive lower limit for a numeric instance.
    /// If the instance is a number, then this keyword validates only
    /// if the instance is greater than "exclusiveMinimum".
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub exclusive_minimum: Option<SchemaNumber>,
    /// The value of "exclusiveMaximum" MUST be a number,
    /// representing an exclusive upper limit for a numeric instance.
    /// If the instance is a number, then this keyword validates only
    /// if the instance is less than "exclusiveMaximum".
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub exclusive_maximum: Option<SchemaNumber>,
    /// Type description
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<Cow<'static, str>>,
    /// This keyword's value MUST be a valid JSON Schema.
    /// An instance is valid against this keyword if it fails to validate successfully against the schema defined by this keyword.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[reflect(ignore)]
    pub not: Option<Box<JsonSchemaBevyType>>,
    /// Default value for the schema.
    #[serde(skip_serializing_if = "Option::is_none", default, rename = "default")]
    #[reflect(ignore)]
    pub default_value: Option<Value>,
    /// Definitions for the schema.
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    #[reflect(ignore)]
    #[serde(rename = "$defs")]
    pub definitions: HashMap<TypeReferenceId, Box<JsonSchemaBevyType>>,
}

/// Represents different types of JSON Schema values that can be used in schema definitions.
///
/// This enum supports the various ways a JSON Schema property can be defined,
/// including boolean values, constant values, and complex schema objects.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Reflect)]
#[serde(untagged)]
pub enum JsonSchemaVariant {
    /// A simple boolean value used in schema definitions.
    ///
    /// This is commonly used for properties like `additionalProperties` where
    /// a boolean true/false indicates whether additional properties are allowed.
    BoolValue(bool),
    /// A full JSON Schema definition.
    ///
    /// This variant contains a complete schema object that defines the structure,
    /// validation rules, and metadata for a particular type or property.
    Schema(#[reflect(ignore)] Box<JsonSchemaBevyType>),
}

impl From<JsonSchemaBevyType> for JsonSchemaVariant {
    fn from(value: JsonSchemaBevyType) -> Self {
        JsonSchemaVariant::Schema(Box::new(value))
    }
}

/// Kind of json schema, maps [`bevy_reflect::TypeInfo`] type
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default, Reflect, Copy)]
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
    /// Single value, eg. primitive types or enum variant
    Value,
    /// Opaque type
    Opaque,
    /// Optional type
    Optional,
}

impl SchemaKind {
    /// Creates a [`SchemaKind`] from a [`TypeRegistration`].
    pub fn from_type_reg(type_reg: &TypeRegistration) -> Self {
        if let Some(info) =
            super::reflect_info::BASE_TYPES_INFO.get(&type_reg.type_info().type_id())
        {
            return info.schema_kind;
        }
        if type_reg.try_get_optional().is_some() {
            return SchemaKind::Optional;
        }
        match type_reg.type_info() {
            TypeInfo::Struct(_) => SchemaKind::Struct,
            TypeInfo::TupleStruct(_) => SchemaKind::TupleStruct,
            TypeInfo::Tuple(_) => SchemaKind::Tuple,
            TypeInfo::List(_) => SchemaKind::List,
            TypeInfo::Array(_) => SchemaKind::Array,
            TypeInfo::Map(_) => SchemaKind::Map,
            TypeInfo::Set(_) => SchemaKind::Set,
            TypeInfo::Opaque(o) => {
                let schema_type: SchemaType = o.ty().id().into();
                match schema_type {
                    SchemaType::Object => SchemaKind::Struct,
                    SchemaType::Array => SchemaKind::Array,
                    _ => SchemaKind::Value,
                }
            }
            TypeInfo::Enum(_) => SchemaKind::Enum,
        }
    }
}
/// Represents the possible type variants for a JSON Schema.
///
/// In JSON Schema, the `type` keyword can either specify a single type
/// or an array of types to allow multiple valid types for a property.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Reflect, Eq, PartialOrd, Ord)]
#[serde(untagged)]
pub enum SchemaTypeVariant {
    /// A single schema type (e.g., "string", "number", "object").
    /// This is the most common case where a property has exactly one valid type.
    Single(SchemaType),
    /// Multiple schema types allowed for the same property.
    /// This variant is used when a property can accept multiple types,
    /// such as allowing both "string" and "number" for the same field.
    /// In Rust case it most often means it is a Option type.
    Multiple(Vec<SchemaType>),
}

impl SchemaTypeVariant {
    /// Adds a new type to the variant.
    pub fn with(self, new: SchemaType) -> Self {
        match self {
            Self::Single(t) => match t.eq(&new) {
                true => Self::Single(t),
                false => Self::Multiple(vec![t, new]),
            },
            Self::Multiple(mut types) => match types.contains(&new) {
                true => Self::Multiple(types),
                false => {
                    types.push(new);
                    Self::Multiple(types)
                }
            },
        }
    }
}

/// Type of json schema
/// More [here](https://json-schema.org/draft-07/draft-handrews-json-schema-01#rfc.section.4.2.1)
#[derive(
    Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Reflect, Default, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "lowercase")]
pub enum SchemaType {
    /// A string of Unicode code points, from the JSON "string" production.
    String,

    /// An arbitrary-precision, base-10 decimal number value, from the JSON "number" production.
    Number,

    /// Represents both a signed and unsigned integer.
    Integer,

    /// An unordered set of properties mapping a string to an instance, from the JSON "object" production.
    Object,

    /// An ordered list of instances, from the JSON "array" production.
    Array,

    /// A "true" or "false" value, from the JSON "true" or "false" productions.
    Boolean,

    /// A JSON "null" production.
    #[default]
    Null,
}

impl From<TypeId> for SchemaType {
    fn from(value: TypeId) -> Self {
        if let Some(info) = super::reflect_info::BASE_TYPES_INFO.get(&value) {
            info.schema_type
        } else {
            Self::Object
        }
    }
}

impl From<SchemaType> for SchemaTypeVariant {
    fn from(value: SchemaType) -> Self {
        SchemaTypeVariant::Single(value)
    }
}
impl From<SchemaType> for Option<SchemaTypeVariant> {
    fn from(value: SchemaType) -> Self {
        Some(SchemaTypeVariant::Single(value))
    }
}

impl SchemaType {
    /// Returns the primitive type corresponding to the given type ID, if it exists.
    pub fn try_get_primitive_type_from_type_id(type_id: TypeId) -> Option<Self> {
        let schema_type: SchemaType = type_id.into();
        if schema_type.eq(&Self::Object) || schema_type.eq(&Self::Array) {
            None
        } else {
            Some(schema_type)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::schemas::open_rpc::OpenRpcDocument;
    use crate::schemas::reflect_info::ReferenceLocation;
    use crate::schemas::CustomInternalSchemaData;
    use crate::schemas::ExternalSchemaSource;

    use super::*;
    use bevy_ecs::prelude::ReflectComponent;
    use bevy_ecs::prelude::ReflectResource;

    use bevy_ecs::{component::Component, reflect::AppTypeRegistry, resource::Resource};
    use bevy_reflect::prelude::ReflectDefault;
    use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
    use serde_json::json;

    fn export_type<T: GetTypeRegistration + 'static>() -> JsonSchemaBevyType {
        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<T>();
        }
        let type_registry = atr.read();
        let Some(schema) =
            type_registry.export_type_json_schema::<T>(&SchemaTypesMetadata::default())
        else {
            panic!("Failed to export JSON schema for Foo");
        };
        schema
    }

    #[test]
    fn reflect_export_struct() {
        #[derive(Reflect, Resource, Default, Deserialize, Serialize)]
        #[reflect(Resource, Default, Serialize, Deserialize)]
        struct Foo {
            a: f32,
            #[reflect(@10..=15i16)]
            b: Option<i16>,
        }
        let schema = export_type::<Foo>();

        assert!(
            !schema
                .reflect_type_data
                .contains(&Cow::Borrowed("Component")),
            "Should not be a component"
        );
        assert!(
            schema
                .reflect_type_data
                .contains(&Cow::Borrowed("Resource")),
            "Should be a resource"
        );

        let _ = schema.properties.get("a").expect("Missing `a` field");
        let _ = schema.properties.get("b").expect("Missing `b` field");
        assert!(
            schema.required.contains(&Cow::Borrowed("a")),
            "Field a should be required"
        );
        assert!(
            schema.required.contains(&Cow::Borrowed("b")),
            "Field b should be required"
        );
    }

    #[test]
    fn reflect_export_enum() {
        #[derive(Reflect, Component, Default, Deserialize, Serialize)]
        #[reflect(Component, Default, Serialize, Deserialize)]
        enum EnumComponent {
            ValueOne(Option<i32>, i32),
            ValueTwo {
                #[reflect(@111..5555i32)]
                test: i32,
            },
            #[default]
            /// default option
            NoValue,
        }
        let schema = export_type::<EnumComponent>();
        assert!(
            schema
                .reflect_type_data
                .contains(&Cow::Borrowed("Component")),
            "Should be a component"
        );
        assert!(
            !schema
                .reflect_type_data
                .contains(&Cow::Borrowed("Resource")),
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
        let schema = export_type::<EnumComponent>();
        assert!(
            !schema
                .reflect_type_data
                .contains(&Cow::Borrowed("Component")),
            "Should not be a component"
        );
        assert!(
            !schema
                .reflect_type_data
                .contains(&Cow::Borrowed("Resource")),
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
            ValueOne(i32, #[reflect(@..155i16)] i16),
            ValueTwo {
                test: i32,
            },
            #[default]
            NoValue,
        }

        #[derive(Clone)]
        pub struct ReflectCustomData;

        impl<T: Reflect> bevy_reflect::FromType<T> for ReflectCustomData {
            fn from_type() -> Self {
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
        let schema = type_registry
            .export_type_json_schema::<EnumComponent>(&metadata)
            .expect("Failed to export schema");

        assert!(
            !metadata.has_type_data::<ReflectComponent>(&schema.reflect_type_data),
            "Should not be a component"
        );
        assert!(
            !metadata.has_type_data::<ReflectResource>(&schema.reflect_type_data),
            "Should not be a resource"
        );
        assert!(
            metadata.has_type_data::<ReflectDefault>(&schema.reflect_type_data),
            "Should have default"
        );
        assert!(
            metadata.has_type_data::<ReflectCustomData>(&schema.reflect_type_data),
            "Should have CustomData"
        );
        assert!(schema.properties.is_empty(), "Should not have any field");
        assert!(schema.one_of.len() == 3, "Should have 3 possible schemas");
    }

    #[test]
    fn reflect_struct_with_custom_schema() {
        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<OpenRpcDocument>();
            register.register_type_data::<OpenRpcDocument, CustomInternalSchemaData>();
        }
        let type_registry = atr.read();
        let schema = type_registry
            .export_type_json_schema::<OpenRpcDocument>(&SchemaTypesMetadata::default())
            .expect("Failed to export schema");
        assert_eq!(
            schema.ref_type,
            Some(TypeReferencePath::new_ref(
                ReferenceLocation::Url,
                "raw.githubusercontent.com/open-rpc/meta-schema/master/schema.json",
            ))
        );
        assert_eq!(
            schema.description,
            Some(
                "Represents an `OpenRPC` document as defined by the `OpenRPC` specification."
                    .into()
            )
        );
        assert!(schema.properties.is_empty());
    }

    #[test]
    fn reflect_export_with_custom_schema() {
        /// Custom type for testing purposes.
        #[derive(Reflect, Component)]
        struct SomeType;

        impl ExternalSchemaSource for SomeType {
            fn get_external_schema_source() -> TypeReferencePath {
                TypeReferencePath::new_ref(
                    ReferenceLocation::Url,
                    "raw.githubusercontent.com/open-rpc/meta-schema/master/schema.json",
                )
            }
        }

        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<SomeType>();
            register.register_type_data::<SomeType, CustomInternalSchemaData>();
        }
        let type_registry = atr.read();
        let schema = type_registry
            .export_type_json_schema::<SomeType>(&SchemaTypesMetadata::default())
            .expect("Failed to export schema");
        assert!(
            !schema
                .reflect_type_data
                .contains(&Cow::Borrowed("Component")),
            "Should not be a component"
        );
        assert!(
            schema.ref_type.is_some_and(|t| !t.to_string().is_empty()),
            "Should have a reference type"
        );
        assert!(
            schema.description.is_some_and(|t| !t.is_empty()),
            "Should have a description"
        );
    }
    #[test]
    fn reflect_export_tuple_struct() {
        #[derive(Reflect, Component, Default, Deserialize, Serialize)]
        #[reflect(Component, Default, Serialize, Deserialize)]
        struct TupleStructType(usize, i32);

        let schema = export_type::<TupleStructType>();
        assert!(
            schema
                .reflect_type_data
                .contains(&Cow::Borrowed("Component")),
            "Should be a component"
        );
        assert!(
            !schema
                .reflect_type_data
                .contains(&Cow::Borrowed("Resource")),
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
            /// Test doc
            a: f32,
            b: u8,
        }

        let schema = export_type::<Foo>();
        let schema_as_value = serde_json::to_value(&schema).expect("Failed to serialize schema");
        let mut value = json!({
          "shortPath": "Foo",
          "$schema": "https://json-schema.org/draft/2020-12/schema",
          "typePath": "bevy_remote::schemas::json_schema::tests::Foo",
          "modulePath": "bevy_remote::schemas::json_schema::tests",
          "crateName": "bevy_remote",
          "reflectTypeData": [
            "Resource",
            "Default",
          ],
          "default": {
            "a": 0.0,
            "b": 0
          },
          "kind": "Struct",
          "type": "object",
          "additionalProperties": false,
          "properties": {
            "a": {
              "title": "a",
              "type": "number",
              "kind": "Value",
              "typePath": "f32"
            },
            "b": {
              "title": "b",
              "minimum": 0,
              "maximum": 255,
              "type": "integer",
              "kind": "Value",
              "typePath": "u8"
            }
          },
          "required": [
            "a",
            "b"
          ]
        });
        if cfg!(feature = "documentation") {
            value["properties"]["a"]["description"] = json!("Test doc");
        }
        assert_normalized_values(schema_as_value, value);
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
