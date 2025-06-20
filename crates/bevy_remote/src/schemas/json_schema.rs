//! Module with JSON Schema type for Bevy Registry Types.
//!  It tries to follow this standard: <https://json-schema.org/specification>
use bevy_platform::collections::HashMap;
use bevy_reflect::{
    prelude::ReflectDefault, serde::ReflectSerializer, GetTypeRegistration, Reflect,
    TypeRegistration, TypeRegistry,
};
use core::any::TypeId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::schemas::{
    reflect_info::{SchemaInfoReflect, SchemaNumber},
    ReflectJsonSchema, SchemaTypesMetadata,
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
        let type_reg = self.get(type_id)?;
        let mut schema: JsonSchemaBevyType = (type_reg, extra_info).into();
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

impl From<(&TypeRegistration, &SchemaTypesMetadata)> for JsonSchemaBevyType {
    fn from(value: (&TypeRegistration, &SchemaTypesMetadata)) -> Self {
        let (reg, metadata) = value;
        if let Some(s) = reg.data::<ReflectJsonSchema>() {
            return s.0.clone();
        }
        let type_info = reg.type_info();
        let base_schema = type_info.build_schema();

        let JsonSchemaVariant::Schema(mut typed_schema) = base_schema else {
            return JsonSchemaBevyType::default();
        };
        typed_schema.reflect_types = metadata.get_registered_reflect_types(reg);
        *typed_schema
    }
}

/// JSON Schema type for Bevy Registry Types
/// It tries to follow this standard: <https://json-schema.org/specification>
///
/// To take the full advantage from info provided by Bevy registry it provides extra fields
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default, Reflect)]
#[serde(rename_all = "camelCase")]
pub struct JsonSchemaBevyType {
    /// JSON Schema specific field.
    /// This keyword is used to reference a statically identified schema.
    #[serde(rename = "$ref")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub ref_type: Option<String>,
    /// Bevy specific field, short path of the type.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub short_path: String,
    /// Bevy specific field, full path of the type.
    #[serde(skip_serializing_if = "String::is_empty", default)]
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
    pub key_type: Option<JsonSchemaVariant>,
    /// Bevy specific field, provided when [`SchemaKind`] `kind` field is equal to [`SchemaKind::Map`].
    ///
    /// It contains type info of value of the Map.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value_type: Option<JsonSchemaVariant>,
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
    /// Validation succeeds if, for each name that appears in both the instance and as a name
    /// within this keyword's value, the child instance for that name successfully validates
    /// against the corresponding schema.
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub properties: HashMap<String, JsonSchemaVariant>,
    /// An object instance is valid against this keyword if every item in the array is the name of a property in the instance.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub required: Vec<String>,
    /// An instance validates successfully against this keyword if it validates successfully against exactly one schema defined by this keyword's value.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub one_of: Vec<JsonSchemaVariant>,
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
    pub description: Option<String>,
    /// Default value for the schema.
    #[serde(skip_serializing_if = "Option::is_none", default, rename = "default")]
    #[reflect(ignore)]
    pub default_value: Option<Value>,
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
    /// A constant value with an optional description.
    ///
    /// This variant represents a JSON Schema `const` keyword, which specifies
    /// that a value must be exactly equal to the given constant value.
    Const {
        /// The constant value that must be matched exactly.
        #[reflect(ignore)]
        #[serde(rename = "const")]
        value: Value,
        /// Optional human-readable description of the constant value.
        #[serde(skip_serializing_if = "Option::is_none", default)]
        description: Option<String>,
    },
    /// A full JSON Schema definition.
    ///
    /// This variant contains a complete schema object that defines the structure,
    /// validation rules, and metadata for a particular type or property.
    Schema(#[reflect(ignore)] Box<JsonSchemaBevyType>),
}

impl JsonSchemaVariant {
    /// Creates a new constant value variant from any serializable type.
    ///
    /// This is a convenience constructor that serializes the provided value
    /// to JSON and wraps it in the `Const` variant with an optional description.
    ///
    /// # Arguments
    ///
    /// * `serializable` - Any value that implements `Serialize`
    /// * `description` - Optional description for the constant value
    ///
    /// # Returns
    ///
    /// A new `JsonSchemaVariant::Const` with the serialized value
    pub fn const_value(serializable: impl Serialize, description: Option<String>) -> Self {
        Self::Const {
            value: serde_json::to_value(serializable).unwrap_or_default(),
            description,
        }
    }
}

/// Kind of json schema, maps [`TypeInfo`] type
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default, Reflect)]
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
    /// Opaque type
    Opaque,
    /// Optional type
    Optional,
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

/// Type of json schema
/// More [here](https://json-schema.org/draft-07/draft-handrews-json-schema-01#rfc.section.4.2.1)
#[derive(
    Debug, Serialize, Deserialize, Clone, PartialEq, Reflect, Default, Eq, PartialOrd, Ord,
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
        if value.eq(&TypeId::of::<bool>()) {
            Self::Boolean
        } else if value.eq(&TypeId::of::<f32>()) || value.eq(&TypeId::of::<f64>()) {
            Self::Number
        } else if value.eq(&TypeId::of::<u8>())
            || value.eq(&TypeId::of::<u16>())
            || value.eq(&TypeId::of::<u32>())
            || value.eq(&TypeId::of::<u64>())
            || value.eq(&TypeId::of::<u128>())
            || value.eq(&TypeId::of::<usize>())
        {
            Self::Integer
        } else if value.eq(&TypeId::of::<i8>())
            || value.eq(&TypeId::of::<i16>())
            || value.eq(&TypeId::of::<i32>())
            || value.eq(&TypeId::of::<i64>())
            || value.eq(&TypeId::of::<i128>())
            || value.eq(&TypeId::of::<isize>())
        {
            Self::Integer
        } else if value.eq(&TypeId::of::<str>())
            || value.eq(&TypeId::of::<char>())
            || value.eq(&TypeId::of::<String>())
        {
            Self::String
        } else {
            Self::Object
        }
    }
}

impl SchemaType {
    /// Returns the primitive type corresponding to the given type ID, if it exists.
    pub fn try_get_primitive_type_from_type_id(type_id: TypeId) -> Option<Self> {
        let schema_type: SchemaType = type_id.into();
        if schema_type.eq(&Self::Object) {
            None
        } else {
            Some(schema_type)
        }
    }
}

#[cfg(test)]
mod tests {
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

        eprintln!("{}", serde_json::to_string_pretty(&schema).expect("msg"));
        assert!(
            !schema.reflect_types.contains(&"Component".to_owned()),
            "Should not be a component"
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
    }

    #[test]
    fn reflect_export_enum() {
        #[derive(Reflect, Component, Default, Deserialize, Serialize)]
        #[reflect(Component, Default, Serialize, Deserialize)]
        enum EnumComponent {
            ValueOne(i32),
            ValueTwo {
                #[reflect(@111..5555i32)]
                test: i32,
            },
            #[default]
            NoValue,
        }
        let schema = export_type::<EnumComponent>();
        eprintln!("{}", serde_json::to_string_pretty(&schema).expect("msg"));
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
        let schema = export_type::<EnumComponent>();
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
            .expect("Failed to export");
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
    fn reflect_export_with_custom_schema() {
        #[derive(Reflect, Component)]
        struct SomeType;

        impl bevy_reflect::FromType<SomeType> for ReflectJsonSchema {
            fn from_type() -> Self {
                JsonSchemaBevyType {
                    ref_type: Some(
                        "https://raw.githubusercontent.com/open-rpc/meta-schema/master/schema.json"
                            .into(),
                    ),
                    description: Some("Custom type for testing purposes.".to_string()),
                    ..Default::default()
                }
                .into()
            }
        }

        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<SomeType>();
            register.register_type_data::<SomeType, ReflectJsonSchema>();
        }
        let type_registry = atr.read();
        let schema = type_registry
            .export_type_json_schema::<SomeType>(&SchemaTypesMetadata::default())
            .expect("Failed to export");
        assert!(
            !schema.reflect_types.contains(&"Component".to_owned()),
            "Should not be a component"
        );
        assert!(
            schema.ref_type.is_some_and(|t| !t.is_empty()),
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
            /// Test doc
            a: u16,
        }

        let schema = export_type::<Foo>();
        let schema_as_value = serde_json::to_value(&schema).expect("Should serialize");
        eprintln!("{:#?}", &schema_as_value);
        let value = json!({
          "shortPath": "Foo",
          "typePath": "bevy_remote::schemas::json_schema::tests::Foo",
          "modulePath": "bevy_remote::schemas::json_schema::tests",
          "crateName": "bevy_remote",
          "reflectTypes": [
            "Resource",
            "Default",
          ],
          "default": {
            "a": 0
          },
          "kind": "Struct",
          "type": "object",
          "additionalProperties": false,
          "properties": {
            "a": {
              "kind": "Value",
              "maximum": 65535,
              "minimum": 0,
              "type": "integer",
              "description": "Test doc",
              "shortPath": "u16",
              "typePath": "u16",

            },
          },
          "required": [
            "a"
          ]
        });
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
