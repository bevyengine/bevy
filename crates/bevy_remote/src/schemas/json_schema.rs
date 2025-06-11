//! Module with JSON Schema type for Bevy Registry Types.
//!  It tries to follow this standard: <https://json-schema.org/specification>
use alloc::borrow::Cow;
use bevy_platform::collections::HashMap;
use bevy_reflect::{
    GetTypeRegistration, NamedField, OpaqueInfo, Reflect, TypeInfo, TypeRegistration, TypeRegistry,
};
use core::any::TypeId;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::schemas::{
    reflect_info::{SchemaInfoReflect, SchemaNumber},
    SchemaTypesMetadata,
};

/// Helper trait for converting `TypeRegistration` to `JsonSchemaBevyType`
pub trait TypeRegistrySchemaReader {
    /// Export type JSON Schema.
    fn export_type_json_schema<T: GetTypeRegistration + 'static>(
        &self,
        extra_info: &SchemaTypesMetadata,
    ) -> Option<JsonSchemaBevyType> {
        self.export_type_json_schema_for_id(extra_info, TypeId::of::<T>())
    }
    /// Export type JSON Schema.
    fn export_type_json_schema_for_id(
        &self,
        extra_info: &SchemaTypesMetadata,
        type_id: TypeId,
    ) -> Option<JsonSchemaBevyType>;
}

impl TypeRegistrySchemaReader for TypeRegistry {
    fn export_type_json_schema_for_id(
        &self,
        extra_info: &SchemaTypesMetadata,
        type_id: TypeId,
    ) -> Option<JsonSchemaBevyType> {
        let type_reg = self.get(type_id)?;
        Some((type_reg, extra_info).into())
    }
}

/// Exports schema info for a given type
pub fn export_type(
    reg: &TypeRegistration,
    metadata: &SchemaTypesMetadata,
) -> (Cow<'static, str>, JsonSchemaBevyType) {
    (reg.type_info().type_path().into(), (reg, metadata).into())
}

impl From<(&TypeRegistration, &SchemaTypesMetadata)> for JsonSchemaBevyType {
    fn from(value: (&TypeRegistration, &SchemaTypesMetadata)) -> Self {
        let (reg, metadata) = value;
        let type_info = reg.type_info();
        let base_schema = type_info.build_schema();

        let JsonSchemaVariant::Schema(mut typed_schema) = base_schema else {
            return JsonSchemaBevyType::default();
        };
        typed_schema.reflect_types = metadata.get_registered_reflect_types(reg);
        match type_info {
            TypeInfo::Struct(info) => {
                // typed_schema.properties = info
                //     .iter()
                //     .map(|field| (field.name().to_owned(), field.build_schema()))
                //     .collect::<HashMap<_, _>>();
                // typed_schema.required = info
                //     .iter()
                //     .filter(|field| !field.type_path().starts_with("core::option::Option"))
                //     .map(|f| f.name().to_owned())
                //     .collect::<Vec<_>>();
                // typed_schema.additional_properties = Some(false);
                // typed_schema.schema_type = Some(SchemaType::Object);
                // typed_schema.kind = SchemaKind::Struct;
            }
            TypeInfo::Enum(info) => {
                typed_schema.kind = SchemaKind::Enum;
                typed_schema.one_of = info
                    .iter()
                    .map(|variant| variant.build_schema())
                    .collect::<Vec<_>>();
            }
            TypeInfo::TupleStruct(info) => {
                typed_schema.schema_type = Some(SchemaTypeVariant::Single(SchemaType::Array));
                typed_schema.kind = SchemaKind::TupleStruct;
                // typed_schema.prefix_items = info
                //     .iter()
                //     .map(SchemaJsonReference::ref_type)
                //     .collect::<Vec<_>>();
                // typed_schema.items = Some(false.into());
            }
            TypeInfo::List(info) => {
                typed_schema.schema_type = Some(SchemaTypeVariant::Single(SchemaType::Array));
                typed_schema.kind = SchemaKind::List;
                // typed_schema.items = info.item_ty().ref_type().into();
            }
            TypeInfo::Array(info) => {
                typed_schema.schema_type = Some(SchemaTypeVariant::Single(SchemaType::Array));
                typed_schema.kind = SchemaKind::Array;
                // typed_schema.items = info.item_ty().ref_type().into();
            }
            TypeInfo::Map(info) => {
                typed_schema.schema_type = Some(SchemaTypeVariant::Single(SchemaType::Array));
                typed_schema.kind = SchemaKind::Map;
                typed_schema.key_type = info.key_ty().ref_type().into();
                typed_schema.value_type = info.value_ty().ref_type().into();
            }
            TypeInfo::Tuple(info) => {
                typed_schema.schema_type = Some(SchemaTypeVariant::Single(SchemaType::Array));
                typed_schema.kind = SchemaKind::Tuple;
                // typed_schema.prefix_items = info
                //     .iter()
                //     .map(SchemaJsonReference::ref_type)
                //     .collect::<Vec<_>>();
                // typed_schema.items = Some(false.into());
            }
            TypeInfo::Set(info) => {
                typed_schema.schema_type = Some(SchemaTypeVariant::Single(SchemaType::Array));
                typed_schema.kind = SchemaKind::Set;
                // typed_schema.items = info.value_ty().ref_type().into();
            }
            TypeInfo::Opaque(info) => {
                typed_schema.schema_type = Some(SchemaTypeVariant::Single(SchemaType::Array));
                typed_schema.kind = SchemaKind::Value;
            }
        };
        *typed_schema
    }
}

/// JSON Schema type for Bevy Registry Types
/// It tries to follow this standard: <https://json-schema.org/specification>
///
/// To take the full advantage from info provided by Bevy registry it provides extra fields
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
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[serde(rename = "type")]
    pub schema_type: Option<SchemaTypeVariant>,
    /// The behavior of this keyword depends on the presence and annotation results of "properties"
    /// and "patternProperties" within the same schema object.
    /// Validation with "additionalProperties" applies only to the child
    /// values of instance names that do not appear in the annotation results of either "properties" or "patternProperties".
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub additional_properties: Option<bool>,
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
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub minimum: Option<SchemaNumber>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub maximum: Option<SchemaNumber>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub exclusive_minimum: Option<SchemaNumber>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub exclusive_maximum: Option<SchemaNumber>,
    /// Type description
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Reflect)]
#[serde(untagged)]
pub enum JsonSchemaVariant {
    BoolValue(bool),
    Const {
        #[reflect(ignore)]
        #[serde(rename = "const")]
        value: Value,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        description: Option<String>,
    },
    Schema(#[reflect(ignore)] Box<JsonSchemaBevyType>),
}
impl JsonSchemaVariant {
    pub fn const_value(serializable: impl Serialize, description: Option<String>) -> Self {
        Self::Const {
            value: serde_json::to_value(serializable).unwrap_or_default(),
            description,
        }
    }
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
    /// Opaque type
    Opaque,
    /// Optional type
    Optional,
}
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Reflect, Eq, PartialOrd, Ord)]
#[serde(untagged)]
pub enum SchemaTypeVariant {
    Single(SchemaType),
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

    /// JSON Schema type keyword from Bevy reflect type path into
    fn map_json_type(&self) -> SchemaType {
        match self.get_type_path() {
            "bool" => SchemaType::Boolean,
            "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => SchemaType::Integer,
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => SchemaType::Integer,
            "f32" | "f64" => SchemaType::Number,
            "char" | "str" | "alloc::string::String" => SchemaType::String,
            _ => SchemaType::Object,
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
        let path = self.path();
        json!({"type": json!({ "$ref": format!("#/$defs/{path}") })})
    }
}

impl SchemaJsonReference for &bevy_reflect::UnnamedField {
    fn ref_type(self) -> Value {
        let path = self.type_path();
        json!({"type": json!({ "$ref": format!("#/$defs/{path}") })})
    }
}

impl SchemaJsonReference for &NamedField {
    fn ref_type(self) -> Value {
        let type_path = self.type_path();
        json!({"type": json!({ "$ref": format!("#/$defs/{type_path}") }), "typePath": self.name()})
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

    #[test]
    fn reflect_export_struct() {
        #[derive(Reflect, Resource, Default, Deserialize, Serialize)]
        #[reflect(Resource, Default, Serialize, Deserialize)]
        struct Foo {
            a: f32,
            #[reflect(@10..=15i16)]
            b: Option<i16>,
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
        // for field in foo_registration
        //     .type_info()
        //     .as_struct()
        //     .expect("msg")
        //     .iter()
        // {
        //     eprintln!("{}: {:#?}", field.name(), field.build_schema_type_info());
        // }
        let (_, schema) = export_type(&foo_registration, &SchemaTypesMetadata::default());

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
        let (_, schema) = export_type(&foo_registration, &SchemaTypesMetadata::default());
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
        let (_, schema) = export_type(&foo_registration, &SchemaTypesMetadata::default());
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
        let foo_registration = type_registry
            .get(TypeId::of::<EnumComponent>())
            .expect("SHOULD BE REGISTERED")
            .clone();
        let (_, schema) = export_type(&foo_registration, &metadata);
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
        let (_, schema) = export_type(&foo_registration, &SchemaTypesMetadata::default());
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
        let (_, schema) = export_type(&foo_registration, &SchemaTypesMetadata::default());
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
              "shortPath": "",
              "typePath": "",

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
