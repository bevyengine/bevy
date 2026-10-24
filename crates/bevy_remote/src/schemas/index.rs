//! A read-only index over a set of [`JsonSchemaBevyType`] schemas.
//!
//! The index can be built from a local [`TypeRegistry`] or from a `registry.schema` response,
//! so local and remote inspectors can read type structure through the same API.
use alloc::{
    borrow::Cow,
    string::{String, ToString},
    vec::Vec,
};
use bevy_ecs::component::Components;
use bevy_platform::collections::HashMap;
use bevy_reflect::TypeRegistry;
use serde_json::Value;

use crate::schemas::{
    json_schema::{export_type, JsonSchemaBevyType, SchemaAnnotations, SchemaKind},
    SchemaTypesMetadata,
};

const REF_PREFIX: &str = "#/";

/// A set of [`JsonSchemaBevyType`] schemas indexed by type path, resolving `$ref`s between them.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SchemaRegistry {
    schemas: HashMap<String, JsonSchemaBevyType>,
}

impl SchemaRegistry {
    /// Builds an index from schemas keyed by type path.
    ///
    /// For a `registry.schema` response, pass the `schema` map of the parsed
    /// [`BrpJsonSchemaResponse`](crate::builtin_methods::BrpJsonSchemaResponse).
    pub fn from_schemas(schemas: impl IntoIterator<Item = (String, JsonSchemaBevyType)>) -> Self {
        Self {
            schemas: schemas.into_iter().collect(),
        }
    }

    /// Builds an index of every type in a local [`TypeRegistry`].
    pub fn from_type_registry(
        registry: &TypeRegistry,
        metadata: &SchemaTypesMetadata,
        components: &Components,
    ) -> Self {
        Self::from_schemas(registry.iter().map(|reg| {
            let (path, schema) = export_type(reg, metadata, components, registry);
            (path.into_owned(), schema)
        }))
    }

    /// Returns the schema of the type with the given type path.
    pub fn get(&self, type_path: &str) -> Option<TypeSchema<'_>> {
        self.schemas.get(type_path).map(|schema| TypeSchema {
            registry: self,
            schema,
        })
    }

    fn resolve<'a>(&'a self, value: &Value) -> Option<TypeSchema<'a>> {
        let path = value.get("$ref")?.as_str()?.strip_prefix(REF_PREFIX)?;
        self.get(path)
    }

    fn named_fields<'a>(
        &'a self,
        properties: impl Iterator<Item = (&'a String, &'a Value)>,
    ) -> Vec<SchemaField<'a>> {
        let mut fields: Vec<_> = properties
            .map(|(name, property)| SchemaField {
                name: Cow::Borrowed(name),
                index: property
                    .get("fieldIndex")
                    .and_then(Value::as_u64)
                    .and_then(|index| usize::try_from(index).ok()),
                ty: self.resolve(property),
                annotations: annotations(property),
            })
            .collect();
        fields.sort_by(|a, b| {
            (a.index.is_none(), a.index, &a.name).cmp(&(b.index.is_none(), b.index, &b.name))
        });
        fields
    }

    fn unnamed_fields<'a>(&'a self, items: &'a [Value]) -> Vec<SchemaField<'a>> {
        items
            .iter()
            .enumerate()
            .map(|(index, item)| SchemaField {
                name: Cow::Owned(index.to_string()),
                index: Some(index),
                ty: self.resolve(item),
                annotations: annotations(item),
            })
            .collect()
    }
}

fn annotations(value: &Value) -> Option<SchemaAnnotations> {
    serde_json::from_value(value.get("annotations")?.clone()).ok()
}

/// The schema of a single type, borrowed from a [`SchemaRegistry`].
#[derive(Debug, Clone, Copy)]
pub struct TypeSchema<'a> {
    registry: &'a SchemaRegistry,
    schema: &'a JsonSchemaBevyType,
}

impl<'a> TypeSchema<'a> {
    /// The full type path of the type.
    pub fn type_path(&self) -> &'a str {
        &self.schema.type_path
    }

    /// The kind of the type.
    pub fn kind(&self) -> &'a SchemaKind {
        &self.schema.kind
    }

    /// The fields of a struct, tuple struct or tuple, in declaration order.
    ///
    /// Tuple-like fields are named by their position. Struct fields without a `fieldIndex`
    /// come after the indexed ones, sorted by name.
    pub fn fields(&self) -> impl Iterator<Item = SchemaField<'a>> + use<'a> {
        let fields = match self.schema.kind {
            SchemaKind::Struct => self.registry.named_fields(self.schema.properties.iter()),
            SchemaKind::Tuple | SchemaKind::TupleStruct => {
                self.registry.unnamed_fields(&self.schema.prefix_items)
            }
            _ => Vec::new(),
        };
        fields.into_iter()
    }

    /// The variants of an enum, in declaration order.
    pub fn variants(&self) -> impl Iterator<Item = SchemaVariant<'a>> + use<'a> {
        let registry = self.registry;
        let variants = match self.schema.kind {
            SchemaKind::Enum => self.schema.one_of.as_slice(),
            _ => &[],
        };
        variants.iter().filter_map(move |variant| {
            let name = variant
                .as_str()
                .or_else(|| variant.get("shortPath")?.as_str())?;
            Some(SchemaVariant {
                name,
                registry,
                variant,
            })
        })
    }

    /// The element type of a list, array or set.
    pub fn items(&self) -> Option<TypeSchema<'a>> {
        match self.schema.kind {
            SchemaKind::List | SchemaKind::Array | SchemaKind::Set => {
                self.registry.resolve(self.schema.items.as_ref()?)
            }
            _ => None,
        }
    }

    /// The key and value types of a map.
    pub fn key_value(&self) -> Option<(TypeSchema<'a>, TypeSchema<'a>)> {
        let key = self.registry.resolve(self.schema.key_type.as_ref()?)?;
        let value = self.registry.resolve(self.schema.value_type.as_ref()?)?;
        Some((key, value))
    }

    /// The default value of the type, if it registers `ReflectDefault`.
    pub fn default_value(&self) -> Option<&'a Value> {
        self.schema.default.as_ref()
    }

    /// The type's annotations, such as `tooltip` or `readOnly`.
    pub fn annotations(&self) -> Option<&'a SchemaAnnotations> {
        self.schema.annotations.as_ref()
    }
}

/// A field of a struct, tuple, or enum variant.
#[derive(Debug, Clone)]
pub struct SchemaField<'a> {
    /// The field name, or its position for tuple-like fields.
    pub name: Cow<'a, str>,
    /// The position of the field in its declaration, if known.
    pub index: Option<usize>,
    /// The type of the field, or `None` if its `$ref` does not resolve.
    pub ty: Option<TypeSchema<'a>>,
    /// The field's annotations, such as `range` or `unit`.
    pub annotations: Option<SchemaAnnotations>,
}

/// A variant of an enum.
#[derive(Debug, Clone, Copy)]
pub struct SchemaVariant<'a> {
    /// The variant name.
    pub name: &'a str,
    registry: &'a SchemaRegistry,
    variant: &'a Value,
}

impl<'a> SchemaVariant<'a> {
    /// The fields of a struct or tuple variant, in declaration order.
    pub fn fields(&self) -> impl Iterator<Item = SchemaField<'a>> + use<'a> {
        let fields = if let Some(Value::Object(properties)) = self.variant.get("properties") {
            self.registry.named_fields(properties.iter())
        } else if let Some(Value::Array(items)) = self.variant.get("prefixItems") {
            self.registry.unnamed_fields(items)
        } else {
            Vec::new()
        };
        fields.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin_methods::BrpJsonSchemaResponse;
    use bevy_annotations::{Step, Tooltip, Unit};
    use bevy_ecs::reflect::AppTypeRegistry;
    use bevy_platform::collections::HashMap;
    use bevy_reflect::{prelude::ReflectDefault, Reflect, TypePath};
    use serde_json::json;

    #[derive(Reflect, Default)]
    #[reflect(Default, @Tooltip("Inner settings"))]
    struct Inner {
        #[reflect(@Unit("m"))]
        value: f32,
    }

    #[derive(Reflect, Default)]
    #[reflect(Default)]
    struct Outer {
        zeta: u8,
        inner: Inner,
        alpha: Option<i32>,
    }

    #[derive(Reflect, Default)]
    #[reflect(Default)]
    enum Shape {
        #[default]
        Empty,
        Circle(f32),
        Rect {
            width: f32,
            height: f32,
        },
    }

    #[derive(Reflect)]
    struct Pair(#[reflect(@Step(2.0))] u8, Inner);

    #[derive(Reflect)]
    struct Collections {
        list: Vec<Inner>,
        map: HashMap<String, Inner>,
    }

    fn type_registry() -> AppTypeRegistry {
        let registry = AppTypeRegistry::default();
        {
            let mut registry = registry.write();
            registry.register::<Outer>();
            registry.register::<Shape>();
            registry.register::<Pair>();
            registry.register::<Collections>();
        }
        registry
    }

    fn local_index() -> SchemaRegistry {
        SchemaRegistry::from_type_registry(
            &type_registry().read(),
            &SchemaTypesMetadata::default(),
            &Components::default(),
        )
    }

    fn field_names<'a>(fields: impl Iterator<Item = SchemaField<'a>>) -> Vec<String> {
        fields.map(|field| field.name.into_owned()).collect()
    }

    fn field_type<'a>(schema: TypeSchema<'a>, name: &str) -> Option<TypeSchema<'a>> {
        schema.fields().find(|field| field.name == name)?.ty
    }

    #[test]
    fn local_and_round_tripped_schemas_match() {
        let local = local_index();
        let response = BrpJsonSchemaResponse {
            id: "urn:bevy-remote:registry".into(),
            schema: local.schemas.clone().into_iter().collect(),
        };
        let json = serde_json::to_string(&response).expect("Should serialize");
        let remote: BrpJsonSchemaResponse =
            serde_json::from_str(&json).expect("Should deserialize");
        let remote = SchemaRegistry::from_schemas(remote.schema);
        assert_eq!(local, remote);

        for path in [
            Outer::type_path(),
            Shape::type_path(),
            Pair::type_path(),
            Collections::type_path(),
        ] {
            let (local, remote) = (local.get(path).unwrap(), remote.get(path).unwrap());
            assert_eq!(field_names(local.fields()), field_names(remote.fields()));
            assert_eq!(local.default_value(), remote.default_value());
        }
    }

    #[test]
    fn fields_follow_declaration_order() {
        let index = local_index();
        let outer = index.get(Outer::type_path()).unwrap();
        assert_eq!(outer.kind(), &SchemaKind::Struct);
        assert_eq!(field_names(outer.fields()), ["zeta", "inner", "alpha"]);
        assert_eq!(
            outer.fields().map(|field| field.index).collect::<Vec<_>>(),
            [Some(0), Some(1), Some(2)]
        );

        let shape = index.get(Shape::type_path()).unwrap();
        let rect = shape
            .variants()
            .find(|variant| variant.name == "Rect")
            .unwrap();
        assert_eq!(field_names(rect.fields()), ["width", "height"]);
    }

    #[test]
    fn refs_resolve_to_nested_types() {
        let index = local_index();
        let outer = index.get(Outer::type_path()).unwrap();
        let inner = field_type(outer, "inner").unwrap();
        assert_eq!(inner.type_path(), Inner::type_path());
        let value = field_type(inner, "value").unwrap();
        assert_eq!(value.type_path(), "f32");
        assert_eq!(value.kind(), &SchemaKind::Value);

        let pair = index.get(Pair::type_path()).unwrap();
        assert_eq!(field_names(pair.fields()), ["0", "1"]);
        assert_eq!(
            field_type(pair, "1").unwrap().type_path(),
            Inner::type_path()
        );
    }

    #[test]
    fn unknown_ref_gives_none() {
        let schema = JsonSchemaBevyType {
            type_path: "Broken".into(),
            properties: [(
                "missing".into(),
                json!({ "$ref": "#/does::not::Exist", "fieldIndex": 0 }),
            )]
            .into(),
            ..Default::default()
        };
        let index = SchemaRegistry::from_schemas([("Broken".into(), schema)]);
        let field = index.get("Broken").unwrap().fields().next().unwrap();
        assert_eq!(field.name, "missing");
        assert!(field.ty.is_none());
        assert!(index.get("does::not::Exist").is_none());
    }

    #[test]
    fn annotations_on_types_and_fields() {
        let index = local_index();
        let inner = index.get(Inner::type_path()).unwrap();
        assert_eq!(
            inner.annotations().and_then(|a| a.tooltip.as_deref()),
            Some("Inner settings")
        );
        let value = inner.fields().next().unwrap();
        assert_eq!(value.annotations.and_then(|a| a.unit), Some("m".to_owned()));

        let pair = index.get(Pair::type_path()).unwrap();
        let steps: Vec<_> = pair
            .fields()
            .map(|field| field.annotations.and_then(|a| a.step))
            .collect();
        assert_eq!(steps, [Some(2.0), None]);
        assert!(index
            .get(Outer::type_path())
            .unwrap()
            .annotations()
            .is_none());
    }

    #[test]
    fn enum_variants_with_fields() {
        let index = local_index();
        let shape = index.get(Shape::type_path()).unwrap();
        assert_eq!(shape.kind(), &SchemaKind::Enum);
        let variants: Vec<_> = shape
            .variants()
            .map(|variant| (variant.name, field_names(variant.fields())))
            .collect();
        assert_eq!(
            variants,
            [
                ("Empty", Vec::<String>::new()),
                ("Circle", vec!["0".to_owned()]),
                ("Rect", vec!["width".to_owned(), "height".to_owned()]),
            ]
        );
        let circle = shape.variants().nth(1).unwrap();
        let radius = circle.fields().next().unwrap().ty.unwrap();
        assert_eq!(radius.type_path(), "f32");
    }

    #[test]
    fn list_items_and_map_key_value() {
        let index = local_index();
        let collections = index.get(Collections::type_path()).unwrap();

        let list = field_type(collections, "list").unwrap();
        assert_eq!(list.kind(), &SchemaKind::List);
        assert_eq!(list.items().unwrap().type_path(), Inner::type_path());
        assert!(list.key_value().is_none());

        let map = field_type(collections, "map").unwrap();
        assert_eq!(map.kind(), &SchemaKind::Map);
        let (key, value) = map.key_value().unwrap();
        assert_eq!(key.type_path(), String::type_path());
        assert_eq!(value.type_path(), Inner::type_path());
        assert!(map.items().is_none());
    }

    #[test]
    fn default_value_is_present() {
        let index = local_index();
        let outer = index.get(Outer::type_path()).unwrap();
        assert_eq!(
            outer.default_value(),
            Some(&json!({ "zeta": 0, "inner": { "value": 0.0 }, "alpha": null }))
        );
        assert!(index
            .get(Collections::type_path())
            .unwrap()
            .default_value()
            .is_none());
    }

    #[test]
    fn missing_field_index_falls_back_to_name_order() {
        let index = local_index();
        let mut schemas = index.schemas.clone();
        let outer = schemas.get_mut(Outer::type_path()).unwrap();
        for property in outer.properties.values_mut() {
            property.as_object_mut().unwrap().remove("fieldIndex");
        }
        let index = SchemaRegistry::from_schemas(schemas);
        let outer = index.get(Outer::type_path()).unwrap();
        assert_eq!(field_names(outer.fields()), ["alpha", "inner", "zeta"]);
        assert!(outer.fields().all(|field| field.index.is_none()));
    }
}
