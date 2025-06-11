//! Module containing information about reflected types.
use bevy_reflect::{GenericInfo, NamedField, Reflect, TypeInfo, UnnamedField, VariantInfo};
use core::any::TypeId;
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    fmt::Debug,
    ops::{Bound, RangeBounds},
};

use crate::schemas::json_schema::{
    JsonSchemaBevyType, JsonSchemaVariant, SchemaKind, SchemaType, SchemaTypeVariant,
};

/// Enum representing a number in schema.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Reflect, PartialOrd)]
#[serde(untagged)]
pub enum SchemaNumber {
    /// Integer value.
    Int(i128),
    /// Floating-point value.
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
/// Represents a bound value that can be either inclusive or exclusive.
/// Used to define range constraints for numeric types in JSON schema.
#[derive(Clone, Debug, Serialize, Deserialize, Copy, PartialEq, Reflect)]
pub enum BoundValue {
    /// An inclusive bound that includes the specified value in the range.
    Inclusive(SchemaNumber),
    /// An exclusive bound that excludes the specified value from the range.
    Exclusive(SchemaNumber),
}

impl BoundValue {
    /// Returns the value if this is an inclusive bound, otherwise returns None.
    fn get_inclusive(&self) -> Option<SchemaNumber> {
        match self {
            BoundValue::Inclusive(v) => Some(*v),
            _ => None,
        }
    }
    /// Returns the value if this is an exclusive bound, otherwise returns None.
    fn get_exclusive(&self) -> Option<SchemaNumber> {
        match self {
            BoundValue::Exclusive(v) => Some(*v),
            _ => None,
        }
    }
}

/// Represents minimum and maximum value constraints for numeric types.
/// Used to define valid ranges for schema validation.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Copy, PartialEq, Reflect)]
pub struct MinMaxValues {
    /// The minimum bound value, if any.
    pub min: Option<BoundValue>,
    /// The maximum bound value, if any.
    pub max: Option<BoundValue>,
}

impl MinMaxValues {
    /// Checks if a given value falls within the defined range constraints.
    /// Returns true if the value is within bounds, false otherwise.
    pub fn in_range(&self, value: SchemaNumber) -> bool {
        if let Some(min) = self.min {
            if let Some(min_value) = min.get_inclusive() {
                if value < min_value {
                    return false;
                }
            } else if let Some(min_value) = min.get_exclusive() {
                if value <= min_value {
                    return false;
                }
            }
        }
        if let Some(max) = self.max {
            if let Some(max_value) = max.get_inclusive() {
                if value > max_value {
                    return false;
                }
            } else if let Some(max_value) = max.get_exclusive() {
                if value >= max_value {
                    return false;
                }
            }
        }
        true
    }
    /// Creates MinMaxValues from a reflected range type.
    /// Attempts to downcast the reflected value to the specified range type T
    /// and extract its bounds.
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

    /// Creates MinMaxValues from range bounds and a type identifier.
    /// Takes a tuple containing start bound, end bound, and TypeId to construct
    /// the appropriate range constraints.
    pub fn from_range<T>(value: (Bound<&T>, Bound<&T>, TypeId)) -> MinMaxValues
    where
        T: 'static + Into<SchemaNumber> + Copy + Debug,
    {
        let base: MinMaxValues = value.2.into();
        let min = match value.0 {
            Bound::Included(v) => Some(BoundValue::Inclusive((*v).into())),
            Bound::Excluded(v) => Some(BoundValue::Exclusive((*v).into())),
            Bound::Unbounded => base.min,
        };
        let max = match value.1 {
            Bound::Included(v) => Some(BoundValue::Inclusive((*v).into())),
            Bound::Excluded(v) => Some(BoundValue::Exclusive((*v).into())),
            Bound::Unbounded => base.max,
        };

        Self { min, max }
    }
}

impl From<TypeId> for MinMaxValues {
    fn from(value: TypeId) -> Self {
        let mut min: Option<BoundValue> = None;
        let mut max: Option<BoundValue> = None;
        if value.eq(&TypeId::of::<u8>()) {
            min = Some(BoundValue::Inclusive(0.into()));
            max = Some(BoundValue::Inclusive(u8::MAX.into()));
        } else if value.eq(&TypeId::of::<u16>()) {
            min = Some(BoundValue::Inclusive(0.into()));
            max = Some(BoundValue::Inclusive(u16::MAX.into()));
        } else if value.eq(&TypeId::of::<u32>()) {
            min = Some(BoundValue::Inclusive(0.into()));
            max = Some(BoundValue::Inclusive(u32::MAX.into()));
        } else if value.eq(&TypeId::of::<u64>()) {
            min = Some(BoundValue::Inclusive(0.into()));
        } else if value.eq(&TypeId::of::<u128>()) {
            min = Some(BoundValue::Inclusive(0.into()));
        } else if value.eq(&TypeId::of::<usize>()) {
            min = Some(BoundValue::Inclusive(0.into()));
        } else if value.eq(&TypeId::of::<i8>()) {
            min = Some(BoundValue::Inclusive(i8::MIN.into()));
            max = Some(BoundValue::Inclusive(i8::MAX.into()));
        } else if value.eq(&TypeId::of::<i16>()) {
            min = Some(BoundValue::Inclusive(i16::MIN.into()));
            max = Some(BoundValue::Inclusive(i16::MAX.into()));
        } else if value.eq(&TypeId::of::<i32>()) {
            min = Some(BoundValue::Inclusive(i32::MIN.into()));
            max = Some(BoundValue::Inclusive(i32::MAX.into()));
        }
        MinMaxValues { min, max }
    }
}
/// Enum representing the internal schema type information for different Rust types.
/// This enum categorizes how different types should be represented in JSON schema.
#[derive(Clone, Debug, Default)]
pub enum InternalSchemaType {
    /// Represents array-like types (Vec, arrays, lists, sets).
    Array {
        /// The TypeId of the element type contained in the array.
        element_type: TypeId,
        /// Optional type information for the element type.
        element_type_info: Option<TypeInfo>,
        /// Minimum number of elements allowed in the array.
        min_size: Option<u64>,
        /// Maximum number of elements allowed in the array.
        max_size: Option<u64>,
    },
    /// Holds all variants of an enum type.
    EnumHolder(Vec<VariantInfo>),
    /// Represents a single enum variant.
    EnumVariant(VariantInfo),
    /// Holds named fields for struct types.
    NamedFieldsHolder(Vec<NamedField>),
    /// Holds unnamed fields for tuple and tuple struct types.
    UnnamedFieldsHolder(Vec<UnnamedField>),
    /// Represents an Optional type (e.g., Option<T>).
    Optional {
        /// Generic information about the wrapped type T in Option<T>.
        generic: GenericInfo,
    },
    /// Represents a Map type (e.g., HashMap<K, V>).
    Map {
        /// The TypeId of the key type contained in the map.
        key_type: TypeId,
        /// Optional type information for the key type.
        key_type_info: Option<TypeInfo>,
        /// The TypeId of the value type contained in the map.
        value_type: TypeId,
        /// Optional type information for the value type.
        value_type_info: Option<TypeInfo>,
    },
    /// Default variant for regular primitive types and other simple types.
    #[default]
    Regular,
}

impl From<&SchemaTypeInfo> for Option<SchemaTypeVariant> {
    fn from(value: &SchemaTypeInfo) -> Self {
        match &value.internal_type {
            InternalSchemaType::Map { .. } => Some(SchemaTypeVariant::Single(SchemaType::Object)),
            InternalSchemaType::Array { .. } => Some(SchemaTypeVariant::Single(SchemaType::Array)),
            InternalSchemaType::EnumHolder { .. } => None,
            InternalSchemaType::EnumVariant(variant) => match variant {
                VariantInfo::Unit(_) => Some(SchemaTypeVariant::Single(SchemaType::String)),
                _ => Some(SchemaTypeVariant::Single(SchemaType::Object)),
            },
            InternalSchemaType::NamedFieldsHolder { .. } => {
                Some(SchemaTypeVariant::Single(SchemaType::Object))
            }
            InternalSchemaType::UnnamedFieldsHolder(unnamed_fields) => {
                if unnamed_fields.len() == 1 {
                    (&unnamed_fields[0].build_schema_type_info()).into()
                } else {
                    Some(SchemaTypeVariant::Single(SchemaType::Array))
                }
            }
            InternalSchemaType::Optional { generic } => {
                let schema_type = if let Some(SchemaTypeVariant::Single(gen_schema)) =
                    (&generic.ty().id().build_schema_type_info()).into()
                {
                    gen_schema
                } else {
                    SchemaType::Object
                };
                Some(SchemaTypeVariant::Multiple(vec![
                    schema_type,
                    SchemaType::Null,
                ]))
            }
            InternalSchemaType::Regular => match &value.type_id {
                Some(s) => Some(SchemaTypeVariant::Single(s.clone().into())),
                _ => None,
            },
        }
    }
}

/// Contains comprehensive information about a type's schema representation.
/// This struct aggregates all the necessary information to generate a JSON schema
/// from Rust type information obtained through reflection.
#[derive(Clone, Debug, Default)]
pub struct SchemaTypeInfo {
    /// The internal categorization of the schema type.
    pub internal_type: InternalSchemaType,
    /// Optional documentation string extracted from the type.
    pub documentation: Option<String>,
    /// The kind of schema (struct, enum, value, etc.).
    pub kind: SchemaKind,
    /// Optional Bevy reflection type information.
    pub type_info: Option<TypeInfo>,
    /// Optional TypeId for the type.
    pub type_id: Option<TypeId>,
    /// Numeric range constraints for the type.
    pub range: MinMaxValues,
}

impl Into<JsonSchemaVariant> for SchemaTypeInfo {
    fn into(self) -> JsonSchemaVariant {
        let schema_type: Option<SchemaTypeVariant> = (&self).into();
        let mut schema = JsonSchemaBevyType {
            schema_type: schema_type.clone(),
            kind: self.kind.clone(),
            description: self.documentation.clone(),
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
            minimum: self.range.min.as_ref().and_then(|r| r.get_inclusive()),
            maximum: self.range.max.as_ref().and_then(|r| r.get_inclusive()),
            exclusive_minimum: self.range.min.as_ref().and_then(|r| r.get_exclusive()),
            exclusive_maximum: self.range.max.as_ref().and_then(|r| r.get_exclusive()),
            ..Default::default()
        };

        match self.internal_type {
            InternalSchemaType::Map {
                key_type,
                key_type_info,
                value_type,
                value_type_info,
            } => {
                schema.additional_properties = match &value_type_info {
                    Some(info) => Some(info.build_schema()),
                    None => Some(value_type.build_schema()),
                };
                schema.value_type = match &value_type_info {
                    Some(info) => Some(info.build_schema()),
                    None => Some(value_type.build_schema()),
                };
                schema.key_type = match &key_type_info {
                    Some(info) => Some(info.build_schema()),
                    None => Some(key_type.build_schema()),
                };
            }
            InternalSchemaType::Regular => {}
            InternalSchemaType::EnumHolder(variants) => {
                schema.one_of = variants.iter().map(|v| v.build_schema()).collect();
            }
            InternalSchemaType::EnumVariant(variant_info) => match &variant_info {
                VariantInfo::Struct(struct_variant_info) => {
                    schema.kind = SchemaKind::Value;
                    schema.schema_type = Some(SchemaTypeVariant::Single(SchemaType::Object));
                    let internal_type = InternalSchemaType::NamedFieldsHolder(
                        struct_variant_info.iter().cloned().collect(),
                    );
                    let schema_field = SchemaTypeInfo {
                        internal_type,
                        documentation: variant_info.to_description(),
                        kind: SchemaKind::Struct,
                        type_info: None,
                        type_id: Some(struct_variant_info.type_id()),
                        range: MinMaxValues::default(),
                    };

                    schema.properties =
                        [(variant_info.name().to_string(), schema_field.into())].into();
                    schema.required = vec![variant_info.name().to_string()];
                }
                VariantInfo::Tuple(tuple_variant_info) => {
                    schema.kind = SchemaKind::Value;
                    schema.schema_type = Some(SchemaTypeVariant::Single(SchemaType::Object));
                    let internal_type = InternalSchemaType::UnnamedFieldsHolder(
                        tuple_variant_info.iter().cloned().collect(),
                    );
                    let schema_field = SchemaTypeInfo {
                        internal_type,
                        documentation: variant_info.to_description(),
                        kind: SchemaKind::Tuple,
                        type_info: None,
                        type_id: Some(tuple_variant_info.type_id()),
                        range: MinMaxValues::default(),
                    };
                    schema.properties =
                        [(variant_info.name().to_string(), schema_field.into())].into();
                    schema.required = vec![variant_info.name().to_string()];
                }
                VariantInfo::Unit(unit_variant_info) => {
                    return JsonSchemaVariant::const_value(
                        unit_variant_info.name().to_string(),
                        schema.description.clone(),
                    );
                }
            },
            InternalSchemaType::NamedFieldsHolder(named_fields) => {
                schema.additional_properties = Some(JsonSchemaVariant::BoolValue(false));
                schema.schema_type = Some(SchemaTypeVariant::Single(SchemaType::Object));
                schema.properties = named_fields
                    .iter()
                    .map(|field| (field.name().to_string(), field.build_schema()))
                    .collect();
                schema.required = named_fields
                    .iter()
                    .map(|field| field.name().to_string())
                    .collect();
            }
            InternalSchemaType::UnnamedFieldsHolder(unnamed_fields) => {
                if unnamed_fields.len() == 1 {
                    let new_schema = unnamed_fields[0].build_schema();
                    if let JsonSchemaVariant::Schema(new_schema_type) = new_schema {
                        schema = *new_schema_type;
                        schema.schema_type = schema_type.clone();
                        schema.description = self.documentation.clone();
                    } else {
                        return new_schema;
                    }
                } else {
                    schema.prefix_items = unnamed_fields.iter().map(|s| s.build_schema()).collect();
                    schema.min_items = Some(unnamed_fields.len() as u64);
                    schema.max_items = Some(unnamed_fields.len() as u64);
                }
            }
            InternalSchemaType::Array {
                element_type,
                element_type_info,
                min_size,
                max_size,
            } => {
                let items_schema = match element_type_info {
                    None => element_type.build_schema(),
                    Some(info) => info.build_schema(),
                };
                schema.items = Some(items_schema);
                schema.min_items = min_size;
                schema.max_items = max_size;
            }
            InternalSchemaType::Optional { generic } => {
                let schema_variant = generic.ty().id().build_schema();
                if let JsonSchemaVariant::Schema(value) = schema_variant {
                    schema = *value;
                    schema.schema_type = schema_type.clone();
                    schema.minimum = self.range.min.as_ref().and_then(|r| r.get_inclusive());
                    schema.maximum = self.range.max.as_ref().and_then(|r| r.get_inclusive());
                    schema.exclusive_minimum =
                        self.range.min.as_ref().and_then(|r| r.get_exclusive());
                    schema.exclusive_maximum =
                        self.range.max.as_ref().and_then(|r| r.get_exclusive());
                    schema.description = self.documentation;
                    schema.kind = SchemaKind::Optional;
                } else {
                    return schema_variant;
                }
            }
        }
        JsonSchemaVariant::Schema(Box::new(schema))
    }
}

/// Trait that builds the type information based on the reflected data.
pub trait SchemaInfoReflect {
    /// Returns the optional type information of the schema.
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
        if self.try_get_optional_info().is_some() {
            return SchemaKind::Optional;
        };
        if SchemaType::try_get_primitive_type_from_type_id(self.get_type()).is_some() {
            return SchemaKind::Value;
        }
        match self.try_get_type_info() {
            Some(type_info) => {
                return match type_info {
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
            None => {}
        }
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
            range: self.get_range_by_id(),
            type_id: Some(self.get_type()),
        }
    }

    /// Builds the internal type information based on the reflected data.
    fn build_internal_type(&self) -> InternalSchemaType {
        if let Some(generic) = self.try_get_optional_info() {
            return InternalSchemaType::Optional { generic };
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
                TypeInfo::Enum(enum_info) => {
                    return InternalSchemaType::EnumHolder(enum_info.iter().cloned().collect());
                }

                TypeInfo::List(list_info) => {
                    return InternalSchemaType::Array {
                        element_type: list_info.item_ty().id(),
                        element_type_info: list_info.item_info().cloned(),
                        min_size: None,
                        max_size: None,
                    }
                }
                TypeInfo::Set(set_info) => {
                    return InternalSchemaType::Array {
                        element_type: set_info.value_ty().id(),
                        element_type_info: None,
                        min_size: None,
                        max_size: None,
                    }
                }
                TypeInfo::Array(array_info) => {
                    return InternalSchemaType::Array {
                        element_type: array_info.item_ty().id(),
                        element_type_info: array_info.item_info().cloned(),
                        min_size: Some(array_info.capacity() as u64),
                        max_size: Some(array_info.capacity() as u64),
                    }
                }
                TypeInfo::Map(map_info) => {
                    return InternalSchemaType::Map {
                        key_type: map_info.key_ty().id(),
                        key_type_info: map_info.key_info().cloned(),
                        value_type: map_info.value_ty().id(),
                        value_type_info: map_info.value_info().cloned(),
                    };
                }
                //
                // TypeInfo::Opaque(opaque_info) => todo!(),
                _ => {}
            }
        }
        InternalSchemaType::Regular
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

    /// Creates MinMaxValues from a reflected range type.
    /// Attempts to downcast the reflected value to the specified range type T
    /// and extract its bounds.
    fn min_max_from_attribute<T, Y>(&self) -> Option<MinMaxValues>
    where
        T: 'static + RangeBounds<Y>,
        Y: 'static + Into<SchemaNumber> + Copy + Debug,
    {
        self.try_get_attribute_by_id(TypeId::of::<T>())
            .and_then(|reflect_value| MinMaxValues::from_reflect::<T, Y>(reflect_value))
    }

    /// Creates MinMaxValues from a reflected range type.
    /// Attempts to downcast the reflected value to the specified range type T
    /// and extract its bounds.
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

    /// Creates MinMaxValues from a reflected range type.
    /// Attempts to downcast the reflected value to the specified range type T
    /// and extract its bounds.
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
        } else if t.eq(&TypeId::of::<i64>()) {
            self.min_max_from_attribute_for_type::<i64>()
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
    use bevy_platform::collections::HashMap;
    use bevy_reflect::GetTypeRegistration;

    use super::*;

    #[test]
    fn integer_test() {
        let type_info = TypeId::of::<u16>().build_schema_type_info();
        let schema_type: Option<SchemaTypeVariant> = (&type_info).into();
        assert_eq!(type_info.range.min, Some(BoundValue::Inclusive(0.into())));
        assert_eq!(
            type_info.range.max,
            Some(BoundValue::Inclusive(u16::MAX.into()))
        );
        assert_eq!(
            schema_type,
            Some(SchemaTypeVariant::Single(SchemaType::Integer))
        );
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
        let schema_type: Option<SchemaTypeVariant> = (&type_info).into();
        assert_eq!(type_info.range.min, Some(BoundValue::Inclusive(10.into())));
        assert_eq!(type_info.range.max, Some(BoundValue::Inclusive(13.into())));
        assert_eq!(
            schema_type,
            Some(SchemaTypeVariant::Single(SchemaType::Integer))
        );
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
        let schema_type: Option<SchemaTypeVariant> = (&type_info).into();
        assert!(!type_info.range.in_range((-1).into()));
        assert!(type_info.range.in_range(0.into()));
        assert!(type_info.range.in_range(12.into()));
        assert!(!type_info.range.in_range(13.into()));
        assert_eq!(type_info.range.min, Some(BoundValue::Inclusive(0.into())));
        assert_eq!(type_info.range.max, Some(BoundValue::Exclusive(13.into())));
        assert_eq!(
            schema_type,
            Some(SchemaTypeVariant::Single(SchemaType::Integer))
        );
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
        let schema_type: Option<SchemaTypeVariant> = (&type_info).into();
        assert_eq!(type_info.range.min, Some(BoundValue::Inclusive(0.into())));
        assert_eq!(type_info.range.max, Some(BoundValue::Exclusive(13.into())));
        assert_eq!(
            schema_type,
            Some(SchemaTypeVariant::Single(SchemaType::Integer))
        );
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
            "{}",
            serde_json::to_string_pretty(
                &EnumTest::get_type_registration().type_info().build_schema()
            )
            .expect("")
        );
    }

    #[test]
    fn reflect_struct_with_array() {
        #[derive(Reflect, Default, Deserialize, Serialize)]
        pub struct ArrayComponent {
            pub arry: [i32; 3],
        }
        eprintln!(
            "{}",
            serde_json::to_string_pretty(
                &ArrayComponent::get_type_registration()
                    .type_info()
                    .build_schema()
            )
            .expect("")
        );
    }

    #[test]
    fn reflect_struct_with_hashmap() {
        #[derive(Reflect, Default, Deserialize, Serialize)]
        pub struct HashMapStruct {
            pub map: HashMap<i32, Option<i32>>,
        }
        assert!(serde_json::from_str::<HashMapStruct>(
            "{\"map\": {\"0\": 1, \"1\": 41, \"2\": null}}"
        )
        .is_ok());

        eprintln!(
            "{}",
            serde_json::to_string_pretty(
                &HashMapStruct::get_type_registration()
                    .type_info()
                    .build_schema()
            )
            .expect("")
        );
    }

    #[test]
    fn reflect_nested_struct() {
        #[derive(Reflect, Default, Deserialize, Serialize)]
        pub struct OtherStruct {
            pub field: String,
        }
        #[derive(Reflect, Default, Deserialize, Serialize)]
        pub struct NestedStruct {
            pub other: OtherStruct,
        }
        eprintln!(
            "{}",
            serde_json::to_string_pretty(
                &NestedStruct::get_type_registration()
                    .type_info()
                    .build_schema()
            )
            .expect("")
        );
    }
}
