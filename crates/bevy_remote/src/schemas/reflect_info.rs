//! Module containing information about reflected types.
use bevy_reflect::{
    GenericInfo, NamedField, Reflect, StructVariantInfo, TypeInfo, UnnamedField, VariantInfo,
};
use core::any::TypeId;
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    f32::consts::PI,
    fmt::Debug,
    ops::{Bound, RangeBounds},
};

use crate::schemas::json_schema::{
    JsonSchemaBevyType, JsonSchemaVariant, SchemaKind, SchemaType, SchemaTypeVariant,
};

/// Enum representing a number in schema.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Reflect)]
#[serde(untagged)]
pub enum SchemaNumber {
    /// Integer value.
    Int(i128),
    /// Always finite.
    Float(f64),
}

impl From<f32> for SchemaNumber {
    fn from(value: f32) -> Self {
        SchemaNumber::Float(value as f64)
    }
}

impl From<f64> for SchemaNumber {
    fn from(value: f64) -> Self {
        SchemaNumber::Float(value)
    }
}
impl From<u8> for SchemaNumber {
    fn from(value: u8) -> Self {
        SchemaNumber::Int(value as i128)
    }
}
impl From<u16> for SchemaNumber {
    fn from(value: u16) -> Self {
        SchemaNumber::Int(value as i128)
    }
}
impl From<u32> for SchemaNumber {
    fn from(value: u32) -> Self {
        SchemaNumber::Int(value as i128)
    }
}
impl From<u64> for SchemaNumber {
    fn from(value: u64) -> Self {
        SchemaNumber::Int(value as i128)
    }
}
impl From<usize> for SchemaNumber {
    fn from(value: usize) -> Self {
        SchemaNumber::Int(value as i128)
    }
}
impl From<i8> for SchemaNumber {
    fn from(value: i8) -> Self {
        SchemaNumber::Int(value as i128)
    }
}
impl From<i16> for SchemaNumber {
    fn from(value: i16) -> Self {
        SchemaNumber::Int(value as i128)
    }
}
impl From<i32> for SchemaNumber {
    fn from(value: i32) -> Self {
        SchemaNumber::Int(value as i128)
    }
}
impl From<i64> for SchemaNumber {
    fn from(value: i64) -> Self {
        SchemaNumber::Int(value as i128)
    }
}
impl From<isize> for SchemaNumber {
    fn from(value: isize) -> Self {
        SchemaNumber::Int(value as i128)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, Copy, PartialEq, Reflect)]
pub struct MinMaxValues {
    pub min: Option<SchemaNumber>,
    pub min_exclusive: Option<SchemaNumber>,
    pub max: Option<SchemaNumber>,
    pub max_exclusive: Option<SchemaNumber>,
}

impl MinMaxValues {
    pub fn from_reflect<T, Y>(reflect_val: &dyn Reflect) -> Option<MinMaxValues>
    where
        T: 'static + RangeBounds<Y>,
        Y: 'static + Into<SchemaNumber> + Copy + Debug,
    {
        let range = reflect_val.downcast_ref::<T>()?;

        Some(Self::from_range((
            range.start_bound(),
            range.end_bound(),
            TypeId::of::<Y>(),
        )))
    }

    pub fn from_range<T>(value: (Bound<&T>, Bound<&T>, TypeId)) -> MinMaxValues
    where
        T: 'static + Into<SchemaNumber> + Copy + Debug,
    {
        let base: MinMaxValues = value.2.into();
        let (min, min_exclusive) = match value.0 {
            Bound::Included(v) => (Some((*v).into()), None),
            Bound::Excluded(v) => (None, Some((*v).into())),
            Bound::Unbounded => (base.min, None),
        };
        let (max, max_exclusive) = match value.1 {
            Bound::Included(v) => (Some((*v).into()), None),
            Bound::Excluded(v) => (None, Some((*v).into())),
            Bound::Unbounded => (base.max, None),
        };

        Self {
            min,
            min_exclusive,
            max,
            max_exclusive,
        }
    }
}

impl From<TypeId> for MinMaxValues {
    fn from(value: TypeId) -> Self {
        let mut min: Option<SchemaNumber> = None;
        let mut max: Option<SchemaNumber> = None;
        if value.eq(&TypeId::of::<u8>()) {
            min = Some(0.into());
            max = Some(u8::MAX.into());
        } else if value.eq(&TypeId::of::<u16>()) {
            min = Some(0.into());
            max = Some(u16::MAX.into());
        } else if value.eq(&TypeId::of::<u32>()) {
            min = Some(0.into());
            max = Some(u32::MAX.into());
        } else if value.eq(&TypeId::of::<u64>()) {
            min = Some(0.into());
        } else if value.eq(&TypeId::of::<u128>()) {
            min = Some(0.into());
        } else if value.eq(&TypeId::of::<usize>()) {
            min = Some(0.into());
        } else if value.eq(&TypeId::of::<i8>()) {
            min = Some(i8::MIN.into());
            max = Some(i8::MAX.into());
        } else if value.eq(&TypeId::of::<i16>()) {
            min = Some(i16::MIN.into());
            max = Some(i16::MAX.into());
        } else if value.eq(&TypeId::of::<i32>()) {
            min = Some(i32::MIN.into());
            max = Some(i32::MAX.into());
        }
        MinMaxValues {
            min,
            max,
            min_exclusive: None,
            max_exclusive: None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub enum InternalSchemaType {
    Primitive {
        range: MinMaxValues,
        schema_type: SchemaType,
    },
    EnumVariant(VariantInfo),
    NamedFieldsHolder(Vec<NamedField>),
    UnnamedFieldsHolder(Vec<UnnamedField>),
    Optional {
        generic: GenericInfo,
        range: MinMaxValues,
        schema_type: SchemaType,
    },
    #[default]
    Regular,
}

#[derive(Clone, Debug, Default)]
pub struct SchemaTypeInfo {
    pub internal_type: InternalSchemaType,
    pub documentation: Option<String>,
    pub kind: SchemaKind,
    pub type_info: Option<TypeInfo>,
}

impl Into<JsonSchemaVariant> for SchemaTypeInfo {
    fn into(self) -> JsonSchemaVariant {
        match self.internal_type {
            InternalSchemaType::Primitive { range, schema_type } => {
                JsonSchemaVariant::Schema(Box::new(JsonSchemaBevyType {
                    kind: self.kind,
                    schema_type: Some(SchemaTypeVariant::Single(schema_type)),
                    minimum: range.min,
                    maximum: range.max,
                    exclusive_minimum: range.min_exclusive,
                    exclusive_maximum: range.max_exclusive,
                    description: self.documentation,
                    ..Default::default()
                }))
            }
            InternalSchemaType::Regular => {
                JsonSchemaVariant::Schema(Box::new(JsonSchemaBevyType {
                    type_path: self
                        .type_info
                        .as_ref()
                        .and_then(|s| Some(s.type_path_table().path().to_owned()))
                        .unwrap_or_default(),
                    short_path: self
                        .type_info
                        .as_ref()
                        .and_then(|s| Some(s.type_path_table().short_path().to_owned()))
                        .unwrap_or_default(),
                    crate_name: self
                        .type_info
                        .as_ref()
                        .and_then(|s| s.type_path_table().crate_name().map(str::to_owned)),
                    module_path: self
                        .type_info
                        .as_ref()
                        .and_then(|s| s.type_path_table().module_path().map(str::to_owned)),
                    description: self.documentation,
                    kind: self.kind,
                    ..Default::default()
                }))
            }
            InternalSchemaType::EnumVariant(variant_info) => match &variant_info {
                VariantInfo::Struct(struct_variant_info) => {
                    let internal_type = InternalSchemaType::NamedFieldsHolder(
                        struct_variant_info.iter().cloned().collect(),
                    );
                    let schema = SchemaTypeInfo {
                        internal_type,
                        documentation: variant_info.to_description(),
                        kind: SchemaKind::Struct,
                        type_info: None,
                    };
                    JsonSchemaVariant::Schema(Box::new(JsonSchemaBevyType {
                        description: self.documentation,
                        kind: SchemaKind::Value,
                        schema_type: Some(SchemaTypeVariant::Single(SchemaType::Object)),
                        properties: [(variant_info.name().to_string(), schema.into())].into(),
                        required: vec![variant_info.name().to_string()],
                        ..Default::default()
                    }))
                }
                VariantInfo::Tuple(tuple_variant_info) => {
                    let internal_type = InternalSchemaType::UnnamedFieldsHolder(
                        tuple_variant_info.iter().cloned().collect(),
                    );
                    let schema = SchemaTypeInfo {
                        internal_type,
                        documentation: variant_info.to_description(),
                        kind: SchemaKind::Tuple,
                        type_info: None,
                    };
                    JsonSchemaVariant::Schema(Box::new(JsonSchemaBevyType {
                        description: self.documentation,
                        kind: SchemaKind::Value,
                        schema_type: Some(SchemaTypeVariant::Single(SchemaType::Object)),
                        properties: [(variant_info.name().to_string(), schema.into())].into(),
                        required: vec![variant_info.name().to_string()],
                        ..Default::default()
                    }))
                }
                VariantInfo::Unit(unit_variant_info) => JsonSchemaVariant::const_value(
                    unit_variant_info.name().to_string(),
                    self.documentation,
                ),
            },
            InternalSchemaType::NamedFieldsHolder(named_fields) => {
                JsonSchemaVariant::Schema(Box::new(JsonSchemaBevyType {
                    kind: self.kind,
                    schema_type: Some(SchemaTypeVariant::Single(SchemaType::Object)),
                    description: self.documentation,
                    additional_properties: Some(false),
                    properties: named_fields
                        .iter()
                        .map(|field| (field.name().to_string(), field.build_schema()))
                        .collect(),
                    required: named_fields
                        .iter()
                        .map(|field| field.name().to_string())
                        .collect(),
                    type_path: self
                        .type_info
                        .as_ref()
                        .and_then(|s| Some(s.type_path_table().path().to_owned()))
                        .unwrap_or_default(),
                    short_path: self
                        .type_info
                        .as_ref()
                        .and_then(|s| Some(s.type_path_table().short_path().to_owned()))
                        .unwrap_or_default(),
                    crate_name: self
                        .type_info
                        .as_ref()
                        .and_then(|s| s.type_path_table().crate_name().map(str::to_owned)),
                    module_path: self
                        .type_info
                        .as_ref()
                        .and_then(|s| s.type_path_table().module_path().map(str::to_owned)),
                    ..Default::default()
                }))
            }
            InternalSchemaType::UnnamedFieldsHolder(unnamed_fields) => {
                if unnamed_fields.len() == 1 {
                    let s = unnamed_fields[0].build_schema();
                    if let JsonSchemaVariant::Schema(mut schema) = s {
                        schema.kind = self.kind;
                        schema.description = self.documentation;

                        schema.type_path = self
                            .type_info
                            .as_ref()
                            .and_then(|s| Some(s.type_path_table().path().to_owned()))
                            .unwrap_or_default();
                        schema.short_path = self
                            .type_info
                            .as_ref()
                            .and_then(|s| Some(s.type_path_table().short_path().to_owned()))
                            .unwrap_or_default();
                        schema.crate_name = self
                            .type_info
                            .as_ref()
                            .and_then(|s| s.type_path_table().crate_name().map(str::to_owned));
                        schema.module_path = self
                            .type_info
                            .as_ref()
                            .and_then(|s| s.type_path_table().module_path().map(str::to_owned));
                        JsonSchemaVariant::Schema(schema)
                    } else {
                        s
                    }
                } else {
                    JsonSchemaVariant::Schema(Box::new(JsonSchemaBevyType {
                        description: self.documentation,
                        kind: self.kind,
                        additional_properties: Some(false),
                        prefix_items: unnamed_fields.iter().map(|s| s.build_schema()).collect(),
                        type_path: self
                            .type_info
                            .as_ref()
                            .and_then(|s| Some(s.type_path_table().path().to_owned()))
                            .unwrap_or_default(),
                        short_path: self
                            .type_info
                            .as_ref()
                            .and_then(|s| Some(s.type_path_table().short_path().to_owned()))
                            .unwrap_or_default(),
                        crate_name: self
                            .type_info
                            .as_ref()
                            .and_then(|s| s.type_path_table().crate_name().map(str::to_owned)),
                        module_path: self
                            .type_info
                            .as_ref()
                            .and_then(|s| s.type_path_table().module_path().map(str::to_owned)),
                        ..Default::default()
                    }))
                }
            }
            InternalSchemaType::Optional {
                generic,
                range,
                schema_type,
            } => {
                let schema_variant = generic.ty().id().build_schema();
                if let JsonSchemaVariant::Schema(mut value) = schema_variant {
                    value.minimum = range.min;
                    value.maximum = range.max;
                    value.exclusive_minimum = range.min_exclusive;
                    value.exclusive_maximum = range.max_exclusive;
                    value.description = self.documentation;
                    value.kind = SchemaKind::Optional;
                    value.schema_type = Some(SchemaTypeVariant::Multiple(vec![
                        schema_type,
                        SchemaType::Null,
                    ]));
                    JsonSchemaVariant::Schema(value)
                } else {
                    schema_variant
                }
            }
        }
    }
}

/// Trait that builds the type information based on the reflected data.
pub trait SchemaInfoReflect {
    fn try_get_optional_info(&self) -> Option<GenericInfo> {
        let type_info = self.try_get_type_info()?;
        let TypeInfo::Enum(enum_info) = type_info else {
            return None;
        };
        if let Some(generic) = enum_info.generics().first() {
            if enum_info.contains_variant("Some")
                && enum_info.contains_variant("None")
                && enum_info.variant_len() == 2
            {
                return Some(generic.clone());
            }
        }
        None
    }
    /// Returns the type information of the schema.
    fn try_get_type_info(&self) -> Option<TypeInfo>;
    /// Returns the Bevy kind of the schema.
    fn get_kind(&self) -> SchemaKind {
        SchemaKind::Value
    }
    /// Builds the type information based on the reflected data.
    fn build_schema(&self) -> JsonSchemaVariant {
        self.build_schema_type_info().into()
    }
    /// Builds the type information based on the reflected data.
    fn build_schema_type_info(&self) -> SchemaTypeInfo {
        let internal_type = self.build_internal_type();
        SchemaTypeInfo {
            type_info: self.try_get_type_info(),
            internal_type,
            documentation: self.to_description(),
            kind: self.get_kind(),
        }
    }

    fn build_internal_type(&self) -> InternalSchemaType {
        if let Some(generic) = self.try_get_optional_info() {
            let range = self.get_range_by_id();
            let schema_type: SchemaType = generic.ty().id().into();
            return InternalSchemaType::Optional {
                generic,
                range,
                schema_type,
            };
        }
        if let Some(type_info) = self.try_get_type_info() {
            match type_info {
                TypeInfo::Struct(struct_info) => {
                    return InternalSchemaType::NamedFieldsHolder(
                        struct_info.iter().cloned().collect(),
                    );
                }
                TypeInfo::TupleStruct(tuple_struct_info) => {
                    return InternalSchemaType::UnnamedFieldsHolder(
                        tuple_struct_info.iter().cloned().collect(),
                    );
                }
                TypeInfo::Tuple(tuple_info) => {
                    return InternalSchemaType::UnnamedFieldsHolder(
                        tuple_info.iter().cloned().collect(),
                    );
                }
                // TypeInfo::Enum(enum_info) => {}

                // TypeInfo::List(list_info) => todo!(),
                // TypeInfo::Array(array_info) => todo!(),
                // TypeInfo::Map(map_info) => todo!(),
                // TypeInfo::Set(set_info) => todo!(),
                //
                // TypeInfo::Opaque(opaque_info) => todo!(),
                _ => {}
            }
        }

        let primitive_type = SchemaType::try_get_primitive_type_from_type_id(self.get_type());
        if let Some(s) = primitive_type {
            InternalSchemaType::Primitive {
                schema_type: s,
                range: self.get_range_by_id(),
            }
        } else {
            InternalSchemaType::Regular
        }
    }

    /// Builds the description based on the reflected data.
    fn to_description(&self) -> Option<String> {
        self.get_docs()
            .map(|s| s.trim().replace("\n", "").to_string())
    }

    /// Returns the documentation of the reflected data.
    fn get_docs(&self) -> Option<&str> {
        None
    }

    /// Get the underlaying TypeId
    fn get_type(&self) -> TypeId;

    /// Try to get the attribute by id
    fn try_get_attribute_by_id(&self, _id: ::core::any::TypeId) -> Option<&dyn Reflect> {
        None
    }

    fn min_max_from_attribute<T, Y>(&self) -> Option<MinMaxValues>
    where
        T: 'static + RangeBounds<Y>,
        Y: 'static + Into<SchemaNumber> + Copy + Debug,
    {
        self.try_get_attribute_by_id(TypeId::of::<T>())
            .and_then(|reflect_value| MinMaxValues::from_reflect::<T, Y>(reflect_value))
    }

    fn min_max_from_attribute_for_type<T>(&self) -> Option<MinMaxValues>
    where
        T: 'static + Into<SchemaNumber> + Copy + Debug,
    {
        let s = self.min_max_from_attribute::<core::ops::RangeInclusive<T>, T>();
        if s.is_some() {
            return s;
        }
        let s = self.min_max_from_attribute::<core::ops::Range<T>, T>();
        if s.is_some() {
            return s;
        }
        let s = self.min_max_from_attribute::<core::ops::RangeTo<T>, T>();
        if s.is_some() {
            return s;
        }
        let s = self.min_max_from_attribute::<core::ops::RangeToInclusive<T>, T>();
        if s.is_some() {
            return s;
        }
        let s = self.min_max_from_attribute::<core::ops::RangeFrom<T>, T>();
        if s.is_some() {
            return s;
        }
        let s = self.min_max_from_attribute::<core::ops::RangeFull, T>();
        if s.is_some() {
            return s;
        }
        None
    }

    fn get_range_by_id(&self) -> MinMaxValues {
        let t = match self.try_get_optional_info() {
            Some(info) => info.ty().id(),
            None => self.get_type(),
        };
        let result = if t.eq(&TypeId::of::<u8>()) {
            self.min_max_from_attribute_for_type::<u8>()
        } else if t.eq(&TypeId::of::<i8>()) {
            self.min_max_from_attribute_for_type::<i8>()
        } else if t.eq(&TypeId::of::<u16>()) {
            self.min_max_from_attribute_for_type::<u16>()
        } else if t.eq(&TypeId::of::<usize>()) {
            self.min_max_from_attribute_for_type::<usize>()
        } else if t.eq(&TypeId::of::<isize>()) {
            self.min_max_from_attribute_for_type::<isize>()
        } else if t.eq(&TypeId::of::<i16>()) {
            self.min_max_from_attribute_for_type::<i16>()
        } else if t.eq(&TypeId::of::<u32>()) {
            self.min_max_from_attribute_for_type::<u32>()
        } else if t.eq(&TypeId::of::<i32>()) {
            self.min_max_from_attribute_for_type::<i32>()
        } else if t.eq(&TypeId::of::<u64>()) {
            self.min_max_from_attribute_for_type::<u64>()
        } else if t.eq(&TypeId::of::<i64>()) {
            self.min_max_from_attribute_for_type::<i64>()
        } else if t.eq(&TypeId::of::<f32>()) {
            self.min_max_from_attribute_for_type::<f32>()
        } else if t.eq(&TypeId::of::<f64>()) {
            self.min_max_from_attribute_for_type::<f64>()
        } else {
            None
        };
        result.unwrap_or(t.into())
    }
}

impl SchemaInfoReflect for UnnamedField {
    fn try_get_type_info(&self) -> Option<TypeInfo> {
        self.type_info().and_then(|info| Some(info.clone()))
    }
    #[cfg(feature = "documentation")]
    fn get_docs(&self) -> Option<&str> {
        self.docs()
    }
    fn get_type(&self) -> TypeId {
        self.type_id()
    }

    fn try_get_attribute_by_id(&self, id: ::core::any::TypeId) -> Option<&dyn Reflect> {
        self.get_attribute_by_id(id)
    }
}

impl SchemaInfoReflect for NamedField {
    fn try_get_type_info(&self) -> Option<TypeInfo> {
        self.type_info().and_then(|info| Some(info.clone()))
    }
    #[cfg(feature = "documentation")]
    fn get_docs(&self) -> Option<&str> {
        self.docs()
    }
    fn get_type(&self) -> TypeId {
        self.type_id()
    }

    fn try_get_attribute_by_id(&self, id: ::core::any::TypeId) -> Option<&dyn Reflect> {
        self.get_attribute_by_id(id)
    }
}

impl SchemaInfoReflect for VariantInfo {
    fn try_get_type_info(&self) -> Option<TypeInfo> {
        None
    }
    #[cfg(feature = "documentation")]
    fn get_docs(&self) -> Option<&str> {
        match self {
            VariantInfo::Unit(info) => info.docs(),
            VariantInfo::Tuple(info) => info.docs(),
            VariantInfo::Struct(info) => info.docs(),
        }
    }

    fn build_internal_type(&self) -> InternalSchemaType {
        InternalSchemaType::EnumVariant(self.clone())
    }

    fn get_type(&self) -> TypeId {
        self.type_id()
    }

    fn try_get_attribute_by_id(&self, id: ::core::any::TypeId) -> Option<&dyn Reflect> {
        self.get_attribute_by_id(id)
    }
}

impl SchemaInfoReflect for TypeInfo {
    fn try_get_type_info(&self) -> Option<TypeInfo> {
        Some(self.clone())
    }
    fn get_kind(&self) -> SchemaKind {
        match self {
            TypeInfo::Struct(_) => SchemaKind::Struct,
            TypeInfo::TupleStruct(_) => SchemaKind::TupleStruct,
            TypeInfo::Tuple(_) => SchemaKind::Tuple,
            TypeInfo::List(_) => SchemaKind::List,
            TypeInfo::Array(_) => SchemaKind::Array,
            TypeInfo::Map(_) => SchemaKind::Map,
            TypeInfo::Set(_) => SchemaKind::Set,
            TypeInfo::Enum(_) => SchemaKind::Enum,
            TypeInfo::Opaque(_) => SchemaKind::Opaque,
        }
    }
    #[cfg(feature = "documentation")]
    fn get_docs(&self) -> Option<&str> {
        self.docs()
    }
    fn get_type(&self) -> TypeId {
        if let Some(generic) = self.generics().first() {
            generic.type_id()
        } else {
            self.type_id()
        }
    }
}

impl SchemaInfoReflect for TypeId {
    fn try_get_type_info(&self) -> Option<TypeInfo> {
        None
    }
    fn get_type(&self) -> TypeId {
        *self
    }
}

#[cfg(test)]
mod tests {
    use bevy_reflect::GetTypeRegistration;

    use super::*;

    #[test]
    fn integer_test() {
        let type_info = TypeId::of::<u16>().build_schema_type_info();
        let InternalSchemaType::Primitive { range, schema_type } = type_info.internal_type else {
            return;
        };
        assert_eq!(range.min, Some(0.into()));
        assert_eq!(range.max, Some(u16::MAX.into()));
        assert_eq!(schema_type, SchemaType::Integer);
    }

    #[test]
    fn custom_range_test() {
        #[derive(Reflect, Default, Deserialize, Serialize)]
        pub struct StructTest {
            /// Test documentation
            #[reflect(@10..=13_i32)]
            no_value: i32,
        }
        let struct_info = StructTest::get_type_registration()
            .type_info()
            .as_struct()
            .expect("Should not fail");
        let field_info = struct_info.field("no_value").expect("Should not fail");
        let type_info = field_info.build_schema_type_info();
        let InternalSchemaType::Primitive { range, schema_type } = type_info.internal_type else {
            return;
        };
        assert_eq!(range.min, Some(10.into()));
        assert_eq!(range.max, Some(13.into()));
        assert_eq!(schema_type, SchemaType::Integer);
        assert_eq!(
            type_info.documentation,
            Some("Test documentation".to_string())
        );
    }

    #[test]
    fn custom_range_test_usize() {
        #[derive(Reflect, Default, Deserialize, Serialize)]
        pub struct StructTest {
            /// Test documentation
            #[reflect(@..13_usize)]
            no_value: usize,
        }
        eprintln!(
            "{}",
            serde_json::to_string_pretty(
                &StructTest::get_type_registration()
                    .type_info()
                    .build_schema()
            )
            .expect("Should not happened")
        );
        let struct_info = StructTest::get_type_registration()
            .type_info()
            .as_struct()
            .expect("Should not fail");
        let field_info = struct_info.field("no_value").expect("Should not fail");
        let type_info = field_info.build_schema_type_info();
        let InternalSchemaType::Primitive { range, schema_type } = type_info.internal_type else {
            return;
        };
        eprintln!("Range: {:#?}", range);
        assert_eq!(range.min, Some(0.into()));
        assert_eq!(range.max, None);
        assert_eq!(range.max_exclusive, Some(13.into()));
        assert_eq!(schema_type, SchemaType::Integer);
        assert_eq!(
            type_info.documentation,
            Some("Test documentation".to_string())
        );
    }

    #[test]
    fn custom_tuple_test_usize() {
        #[derive(Reflect, Default, Deserialize, Serialize)]
        pub struct TupleTest(
            /// Test documentation
            #[reflect(@..13_usize)]
            pub usize,
        );
        eprintln!(
            "{}",
            serde_json::to_string_pretty(
                &TupleTest::get_type_registration()
                    .type_info()
                    .build_schema()
            )
            .expect("SD")
        );
        let struct_info = TupleTest::get_type_registration()
            .type_info()
            .as_tuple_struct()
            .expect("Should not fail");
        let field_info = struct_info.iter().next().expect("Should not fail");
        let type_info = field_info.build_schema_type_info();
        let InternalSchemaType::Primitive { range, schema_type } = type_info.internal_type else {
            return;
        };
        assert_eq!(range.min, Some(0.into()));
        assert_eq!(range.max, None);
        assert_eq!(range.max_exclusive, Some(13.into()));
        assert_eq!(schema_type, SchemaType::Integer);
        assert_eq!(
            type_info.documentation,
            Some("Test documentation".to_string())
        );
    }

    #[test]
    fn custom_enum_test() {
        #[derive(Reflect, Default, Deserialize, Serialize)]
        pub enum EnumTest {
            /// Variant documentation
            #[default]
            Variant1,
            Variant2 {
                field1: String,
                field2: u32,
            },
            Variant3(isize, usize),
            Variant4(usize),
        }
        eprintln!(
            "{:#?}",
            EnumTest::get_type_registration().type_info().build_schema()
        );
        let enum_info = EnumTest::get_type_registration()
            .type_info()
            .as_enum()
            .expect("Should not fail");
        for field in enum_info.iter() {
            let type_info = field.build_schema();
            eprintln!(
                "{}: {}",
                field.name(),
                serde_json::to_string_pretty(&type_info).unwrap()
            );
        }
    }
}
