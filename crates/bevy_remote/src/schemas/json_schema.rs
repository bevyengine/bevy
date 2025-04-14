//! Module with JSON Schema type for Bevy Registry Types.
//!  It tries to follow this standard: <https://json-schema.org/specification>
use bevy_ecs::reflect::{ReflectComponent, ReflectResource};
use bevy_platform::collections::HashMap;
use bevy_reflect::{
    prelude::ReflectDefault, NamedField, OpaqueInfo, ReflectDeserialize, ReflectSerialize,
    TypeInfo, TypeRegistration, VariantInfo,
};
use core::any::TypeId;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

/// Exports schema info for a given type
pub fn export_type(reg: &TypeRegistration) -> (String, JsonSchemaBevyType) {
    (reg.type_info().type_path().to_owned(), reg.into())
}

fn get_registered_reflect_types(reg: &TypeRegistration) -> Vec<String> {
    // Vec could be moved to allow registering more types by game maker.
    let registered_reflect_types: [(TypeId, &str); 5] = [
        { (TypeId::of::<ReflectComponent>(), "Component") },
        { (TypeId::of::<ReflectResource>(), "Resource") },
        { (TypeId::of::<ReflectDefault>(), "Default") },
        { (TypeId::of::<ReflectSerialize>(), "Serialize") },
        { (TypeId::of::<ReflectDeserialize>(), "Deserialize") },
    ];
    let mut result = Vec::new();
    for (id, name) in registered_reflect_types {
        if reg.data_by_id(id).is_some() {
            result.push(name.to_owned());
        }
    }
    result
}

impl From<&TypeRegistration> for JsonSchemaBevyType {
    fn from(reg: &TypeRegistration) -> Self {
        let t = reg.type_info();
        let binding = t.type_path_table();

        let short_path = binding.short_path();
        let type_path = binding.path();
        let mut typed_schema = JsonSchemaBevyType {
            reflect_types: get_registered_reflect_types(reg),
            short_path: short_path.to_owned(),
            type_path: type_path.to_owned(),
            crate_name: binding.crate_name().map(str::to_owned),
            module_path: binding.module_path().map(str::to_owned),
            ..Default::default()
        };
        match t {
            TypeInfo::Struct(info) => {
                typed_schema.properties = info
                    .iter()
                    .map(|field| (field.name().to_owned(), field.ty().ref_type()))
                    .collect::<HashMap<_, _>>();
                typed_schema.required = info
                    .iter()
                    .filter(|field| !field.type_path().starts_with("core::option::Option"))
                    .map(|f| f.name().to_owned())
                    .collect::<Vec<_>>();
                typed_schema.additional_properties = Some(false);
                typed_schema.schema_type = SchemaType::Object;
                typed_schema.kind = SchemaKind::Struct;
            }
            TypeInfo::Enum(info) => {
                typed_schema.kind = SchemaKind::Enum;

                let simple = info
                    .iter()
                    .all(|variant| matches!(variant, VariantInfo::Unit(_)));
                if simple {
                    typed_schema.schema_type = SchemaType::String;
                    typed_schema.one_of = info
                        .iter()
                        .map(|variant| match variant {
                            VariantInfo::Unit(v) => v.name().into(),
                            _ => unreachable!(),
                        })
                        .collect::<Vec<_>>();
                } else {
                    typed_schema.schema_type = SchemaType::Object;
                    typed_schema.one_of = info
                .iter()
                .map(|variant| match variant {
                    VariantInfo::Struct(v) => json!({
                        "type": "object",
                        "kind": "Struct",
                        "typePath": format!("{}::{}", type_path, v.name()),
                        "shortPath": v.name(),
                        "properties": v
                            .iter()
                            .map(|field| (field.name().to_owned(), field.ref_type()))
                            .collect::<Map<_, _>>(),
                        "additionalProperties": false,
                        "required": v
                            .iter()
                            .filter(|field| !field.type_path().starts_with("core::option::Option"))
                            .map(NamedField::name)
                            .collect::<Vec<_>>(),
                    }),
                    VariantInfo::Tuple(v) => json!({
                        "type": "array",
                        "kind": "Tuple",
                        "typePath": format!("{}::{}", type_path, v.name()),
                        "shortPath": v.name(),
                        "prefixItems": v
                            .iter()
                            .map(SchemaJsonReference::ref_type)
                            .collect::<Vec<_>>(),
                        "items": false,
                    }),
                    VariantInfo::Unit(v) => json!({
                        "typePath": format!("{}::{}", type_path, v.name()),
                        "shortPath": v.name(),
                    }),
                })
                .collect::<Vec<_>>();
                }
            }
            TypeInfo::TupleStruct(info) => {
                typed_schema.schema_type = SchemaType::Array;
                typed_schema.kind = SchemaKind::TupleStruct;
                typed_schema.prefix_items = info
                    .iter()
                    .map(SchemaJsonReference::ref_type)
                    .collect::<Vec<_>>();
                typed_schema.items = Some(false.into());
            }
            TypeInfo::List(info) => {
                typed_schema.schema_type = SchemaType::Array;
                typed_schema.kind = SchemaKind::List;
                typed_schema.items = info.item_ty().ref_type().into();
            }
            TypeInfo::Array(info) => {
                typed_schema.schema_type = SchemaType::Array;
                typed_schema.kind = SchemaKind::Array;
                typed_schema.items = info.item_ty().ref_type().into();
            }
            TypeInfo::Map(info) => {
                typed_schema.schema_type = SchemaType::Object;
                typed_schema.kind = SchemaKind::Map;
                typed_schema.key_type = info.key_ty().ref_type().into();
                typed_schema.value_type = info.value_ty().ref_type().into();
            }
            TypeInfo::Tuple(info) => {
                typed_schema.schema_type = SchemaType::Array;
                typed_schema.kind = SchemaKind::Tuple;
                typed_schema.prefix_items = info
                    .iter()
                    .map(SchemaJsonReference::ref_type)
                    .collect::<Vec<_>>();
                typed_schema.items = Some(false.into());
            }
            TypeInfo::Set(info) => {
                typed_schema.schema_type = SchemaType::Set;
                typed_schema.kind = SchemaKind::Set;
                typed_schema.items = info.value_ty().ref_type().into();
            }
            TypeInfo::Opaque(info) => {
                typed_schema.schema_type = info.map_json_type();
                typed_schema.kind = SchemaKind::Value;
            }
        };
        typed_schema
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
    #[serde(rename = "type")]
    pub schema_type: SchemaType,
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
    pub properties: HashMap<String, Value>,
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

    /// Represents a floating-point number.
    Float,

    /// Represents an unsigned integer.
    Uint,

    /// Represents a signed integer.
    Int,

    /// Represents an object with key-value pairs.
    Object,

    /// Represents an array of values.
    Array,

    /// Represents a boolean value (true or false).
    Boolean,

    /// Represents a set of unique values.
    Set,

    /// Represents a null value.
    #[default]
    Null,
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
            "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => SchemaType::Uint,
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => SchemaType::Int,
            "f32" | "f64" => SchemaType::Float,
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
    use bevy_ecs::{component::Component, reflect::AppTypeRegistry, resource::Resource};
    use bevy_reflect::Reflect;

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
        let (_, schema) = export_type(&foo_registration);

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
        let (_, schema) = export_type(&foo_registration);
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
        let (_, schema) = export_type(&foo_registration);
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
        let (_, schema) = export_type(&foo_registration);
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
        let (_, schema) = export_type(&foo_registration);
        let schema_as_value = serde_json::to_value(&schema).expect("Should serialize");
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
              "type": {
                "$ref": "#/$defs/f32"
              }
            },
          },
          "required": [
            "a"
          ]
        });
        assert_eq!(schema_as_value, value);
    }
}
