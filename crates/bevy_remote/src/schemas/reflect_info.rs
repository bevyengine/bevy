//! Module containing information about reflected types.
use alloc::borrow::Cow;
use alloc::sync::Arc;
use bevy_derive::{Deref, DerefMut};
use bevy_platform::collections::HashMap;
use bevy_reflect::attributes::CustomAttributes;
use bevy_reflect::{
    EnumInfo, GenericInfo, NamedField, Reflect, Type, TypeInfo, TypePathTable, TypeRegistration,
    UnnamedField, VariantInfo,
};
use bevy_utils::{default, TypeIdMap};
use core::any::TypeId;
use core::fmt;
use core::fmt::{Display, Formatter};
use core::slice::Iter;
use core::{
    any::Any,
    fmt::Debug,
    ops::{Bound, RangeBounds},
};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::schemas::json_schema::{
    JsonSchemaBevyType, JsonSchemaVariant, SchemaKind, SchemaType, SchemaTypeVariant,
};
use crate::schemas::{ReflectJsonSchemaForceAsArray, SchemaTypesMetadata};

#[derive(
    Debug,
    Clone,
    PartialEq,
    Default,
    Reflect,
    Deref,
    Hash,
    Eq,
    Ord,
    PartialOrd,
    Serialize,
    Deserialize,
)]
/// Reference id of the type.
pub struct TypeReferenceId(Cow<'static, str>);

impl Display for TypeReferenceId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TypeReferenceId {
    /// Returns the type path of the reference.
    pub fn type_path(&self) -> String {
        self.replace("-", "::")
    }
}

impl From<&Type> for TypeReferenceId {
    fn from(t: &Type) -> Self {
        TypeReferenceId(t.path().replace("::", "-").into())
    }
}

impl From<&TypePathTable> for TypeReferenceId {
    fn from(t: &TypePathTable) -> Self {
        TypeReferenceId(t.path().replace("::", "-").into())
    }
}
impl From<&str> for TypeReferenceId {
    fn from(t: &str) -> Self {
        TypeReferenceId(t.replace("::", "-").into())
    }
}

/// Information about the attributes of a field.
#[derive(Clone, Debug)]
pub struct FieldInformation {
    /// Field specific data
    field: SchemaFieldData,
    /// Type information of the field.
    type_info: TypeInformation,
}

/// Information about the field type.
#[derive(Clone, Debug, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub enum FieldType {
    /// Named field type.
    Named,
    /// Unnamed field type.
    #[default]
    Unnamed,
    /// Named field type that is stored as unnamed. Example: glam Vec3.
    ForceUnnamed,
}

/// Information about the attributes of a field.
#[derive(Clone, Debug, Deref, DerefMut, Default)]
pub struct FieldsInformation {
    /// Fields information.
    #[deref]
    fields: Vec<FieldInformation>,
    /// Field type information.
    fields_type: FieldType,
}

impl From<&TypeInformation> for Option<FieldsInformation> {
    fn from(value: &TypeInformation) -> Self {
        let info = value.try_get_type_info()?;
        let (fields, fields_type) = match info {
            TypeInfo::Struct(struct_info) => (
                get_fields_information(struct_info.iter()),
                if value.is_forced_as_array() {
                    FieldType::ForceUnnamed
                } else {
                    FieldType::Named
                },
            ),
            TypeInfo::TupleStruct(tuple_struct_info) => (
                get_fields_information(tuple_struct_info.iter()),
                FieldType::Unnamed,
            ),
            TypeInfo::Tuple(tuple_info) => (
                get_fields_information(tuple_info.iter()),
                FieldType::Unnamed,
            ),
            TypeInfo::List(_)
            | TypeInfo::Array(_)
            | TypeInfo::Map(_)
            | TypeInfo::Set(_)
            | TypeInfo::Enum(_)
            | TypeInfo::Opaque(_) => return None,
        };
        Some(FieldsInformation {
            fields,
            fields_type,
        })
    }
}

#[derive(Clone, Debug, Deref)]
/// Information about the attributes of a field.
pub struct AttributesInformation(Arc<TypeIdMap<Box<dyn Reflect>>>);

impl From<&CustomAttributes> for AttributesInformation {
    fn from(attributes: &CustomAttributes) -> Self {
        let map = attributes
            .iter()
            .flat_map(|(id, attr)| attr.reflect_clone().map(|attr| (*id, attr)))
            .collect();
        AttributesInformation(Arc::new(map))
    }
}

impl AttributeInfoReflect for AttributesInformation {
    fn try_get_attribute_by_id(&self, id: TypeId) -> Option<&dyn Reflect> {
        self.get(&id).map(|s| &**s)
    }
}

impl AttributeInfoReflect for CustomAttributes {
    fn try_get_attribute_by_id(&self, id: TypeId) -> Option<&dyn Reflect> {
        self.get_by_id(id)
    }
}
/// Enum representing different types of type information available for reflection.
///
/// This enum provides a unified interface for accessing type information from various
/// sources in the Bevy reflection system, allowing for flexible handling of type
/// metadata during schema generation.
#[derive(Clone, Debug)]
pub enum TypeInformation {
    /// Contains a complete type registration with all associated metadata.
    ///
    /// This variant holds a full `TypeRegistration` which includes type info,
    /// type data, and other registration details.
    TypeRegistration(TypeRegistration),

    /// Contains detailed type information about the structure of a type.
    ///
    /// This variant holds a `TypeInfo` which describes the internal structure
    /// of a type (struct, enum, tuple, etc.) and its fields or variants.
    TypeInfo(Box<TypeInfo>),

    /// Contains information about a specific enum variant.
    ///
    /// This variant holds a `VariantInfo` which describes a single variant
    /// of an enum, including its name, fields, and other metadata.
    VariantInfo(Box<VariantInfo>),

    /// Contains basic type information without detailed structure.
    ///
    /// This variant holds a `Type` which provides basic type metadata
    /// like type path and ID, but without detailed structural information.
    Type(Box<Type>),

    /// Contains only the type identifier.
    ///
    /// This variant holds just a `TypeId` which uniquely identifies a type
    /// but provides no additional metadata about its structure or properties.
    TypeId(TypeId),
}

impl TypeInformation {
    /// Checks for custom schema.
    pub fn try_get_custom_schema(&self) -> Option<&super::ReflectJsonSchema> {
        if let Self::TypeRegistration(reg) = self {
            reg.data::<super::ReflectJsonSchema>()
        } else {
            None
        }
    }
    /// Builds a `TypeReferenceId` from the type path.
    pub fn try_get_type_reference_id(&self) -> Option<TypeReferenceId> {
        if let Some(schema) = self.try_get_custom_schema() {
            if schema.0.id.is_empty() {
                None
            } else {
                Some(schema.0.id.trim().into())
            }
        } else if self.is_primitive_type() {
            None
        } else if let Some(optional) = self.try_get_optional_info() {
            Some(optional.type_path().into())
        } else if let Some(s) = self.try_get_type_info() {
            if s.as_array().is_ok() || s.as_list().is_ok() || s.as_map().is_ok() {
                None
            } else {
                self.try_get_type_path_table().map(|t| t.path().into())
            }
        } else {
            self.try_get_type_path_table().map(|t| t.path().into())
        }
    }
    /// Returns true if the stored type is a primitive one.
    pub fn is_primitive_type(&self) -> bool {
        self.try_get_primitive_type().is_some()
    }
    /// Returns the primitive type if the stored type is a primitive one.
    pub fn try_get_primitive_type(&self) -> Option<SchemaType> {
        SchemaType::try_get_primitive_type_from_type_id(self.type_id())
    }
    /// Converts the type information into a schema type information.
    pub fn to_schema_type_info(self) -> SchemaTypeInfo {
        let stored_fields = (&self).into();
        SchemaTypeInfo {
            ty_info: self,
            field_data: None,
            stored_fields,
            reflect_type_data: None,
        }
    }
    /// Converts the type information into a schema type information.
    pub fn to_schema_type_info_with_metadata(
        self,
        metadata: &SchemaTypesMetadata,
    ) -> SchemaTypeInfo {
        let stored_fields = (&self).into();
        let reflect_type_data = if let Self::TypeRegistration(reg) = &self {
            Some(metadata.get_registered_reflect_types(reg))
        } else {
            None
        };
        SchemaTypeInfo {
            ty_info: self,
            field_data: None,
            stored_fields,
            reflect_type_data,
        }
    }
    /// Returns the documentation of the type.
    #[cfg(feature = "documentation")]
    pub fn get_docs(&self) -> Option<Cow<'static, str>> {
        let docs = match self {
            TypeInformation::TypeRegistration(type_registration) => {
                type_registration.type_info().docs()
            }
            TypeInformation::TypeInfo(type_info) => type_info.docs(),
            TypeInformation::VariantInfo(variant_info) => variant_info.docs(),
            _ => None,
        };

        docs.map(|docs| docs.trim().replace("\n", "").into())
    }

    /// Returns the documentation of the type.
    #[cfg(not(feature = "documentation"))]
    pub fn get_docs(&self) -> Option<&str> {
        None
    }

    /// Returns the type information of the type.
    pub fn try_get_type_info(&self) -> Option<&TypeInfo> {
        match self {
            TypeInformation::TypeInfo(type_info) => Some(&**type_info),
            TypeInformation::TypeRegistration(reg) => Some(reg.type_info()),
            _ => None,
        }
    }

    /// Returns the type of the type.
    pub fn try_get_type(&self) -> Option<&Type> {
        match self {
            TypeInformation::TypeInfo(type_info) => Some(type_info.ty()),
            TypeInformation::TypeRegistration(reg) => Some(reg.type_info().ty()),
            TypeInformation::Type(t) => Some(&**t),
            _ => None,
        }
    }

    /// Returns whether the type is forced as an array.
    pub fn is_forced_as_array(&self) -> bool {
        match self {
            TypeInformation::TypeRegistration(type_registration) => type_registration
                .data::<ReflectJsonSchemaForceAsArray>()
                .is_some(),
            _ => false,
        }
    }

    /// Returns the type path table of the schema.
    pub fn try_get_type_path_table(&self) -> Option<&TypePathTable> {
        match self {
            TypeInformation::Type(t) => Some(t.type_path_table()),
            TypeInformation::TypeId(_) | TypeInformation::VariantInfo(_) => None,
            TypeInformation::TypeInfo(type_info) => Some(type_info.type_path_table()),
            TypeInformation::TypeRegistration(type_registration) => {
                Some(type_registration.type_info().type_path_table())
            }
        }
    }
    /// Builds range information based on the type ID.
    pub fn get_range(&self) -> MinMaxValues {
        self.type_id().into()
    }
    /// Returns the type ID of the schema.
    pub fn type_id(&self) -> TypeId {
        if let Some(s) = self.try_get_optional_info() {
            return s.type_id();
        }
        match self {
            Self::TypeId(id) => *id,
            Self::Type(t) => t.id(),
            Self::TypeInfo(type_info) => {
                if let TypeInfo::Opaque(o) = &**type_info {
                    o.type_id()
                } else {
                    type_info.type_id()
                }
            }
            TypeInformation::TypeRegistration(type_registration) => type_registration.type_id(),
            TypeInformation::VariantInfo(variant_info) => match &**variant_info {
                VariantInfo::Struct(struct_info) => struct_info.type_id(),
                VariantInfo::Tuple(tuple_variant_info) => {
                    if tuple_variant_info.field_len() == 1 {
                        tuple_variant_info.field_at(0).expect("").type_id()
                    } else {
                        tuple_variant_info.type_id()
                    }
                }
                VariantInfo::Unit(unit_variant_info) => unit_variant_info.type_id(),
            },
        }
    }
    /// Returns the optional type information of the schema.
    pub fn try_get_optional_info(&self) -> Option<&GenericInfo> {
        let Some(TypeInfo::Enum(enum_info)) = self.try_get_type_info() else {
            return None;
        };
        Self::try_get_optional_from_info(enum_info)
    }
    /// Try to get the optional type information from the enum information.
    pub fn try_get_optional_from_info(enum_info: &EnumInfo) -> Option<&GenericInfo> {
        let generic = enum_info.generics().first()?;
        if enum_info.variant_len() != 2
            || !enum_info.contains_variant("Some")
            || !enum_info.contains_variant("None")
        {
            return None;
        }

        Some(generic)
    }
}

impl Default for TypeInformation {
    fn default() -> Self {
        Self::TypeId(TypeId::of::<()>())
    }
}

impl From<&TypeInformation> for SchemaKind {
    fn from(value: &TypeInformation) -> Self {
        let type_info = value.try_get_type_info();
        let schema_type: SchemaType = if let Some(type_info) = type_info {
            match type_info {
                TypeInfo::Struct(_) => return SchemaKind::Struct,
                TypeInfo::TupleStruct(_) => return SchemaKind::TupleStruct,
                TypeInfo::Tuple(_) => return SchemaKind::Tuple,
                TypeInfo::List(_) => return SchemaKind::List,
                TypeInfo::Array(_) => return SchemaKind::Array,
                TypeInfo::Map(_) => return SchemaKind::Map,
                TypeInfo::Set(_) => return SchemaKind::Set,
                TypeInfo::Opaque(o) => o.type_id().into(),
                TypeInfo::Enum(enum_info) => {
                    return if TypeInformation::try_get_optional_from_info(enum_info).is_some() {
                        SchemaKind::Optional
                    } else {
                        SchemaKind::Enum
                    };
                }
            }
        } else {
            if let TypeInformation::VariantInfo(_) = &value {
                return SchemaKind::Value;
            }
            value.type_id().into()
        };
        match schema_type {
            SchemaType::Object => SchemaKind::Struct,
            SchemaType::Array => SchemaKind::Array,
            _ => SchemaKind::Value,
        }
    }
}

impl TryFrom<&TypeInformation> for TypeReferenceId {
    type Error = ();

    fn try_from(value: &TypeInformation) -> Result<Self, Self::Error> {
        match value.try_get_type_path_table() {
            Some(table) => Ok(table.into()),
            None => Err(()),
        }
    }
}

/// Represents the data of a field in a schema.
#[derive(Clone)]
pub struct SchemaFieldData {
    /// Name of the field.
    pub name: Option<Cow<'static, str>>,
    /// Index of the field. Can be provided for named fields when the data is obtained from containing struct definition.
    pub index: Option<usize>,
    /// Description of the field.
    pub description: Option<Cow<'static, str>>,
    /// Attributes of the field.
    pub attributes: AttributesInformation,
}

impl Debug for SchemaFieldData {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("SchemaFieldData")
            .field("name", &self.name)
            .field("index", &self.index)
            .field("description", &self.description)
            .finish()
    }
}

impl SchemaFieldData {
    /// Returns the name of the field.
    pub fn to_name(&self) -> Cow<'static, str> {
        match &self.name {
            Some(name) => name.clone(),
            None => Cow::Owned(format!("[{}]", self.index.unwrap_or(0))),
        }
    }
    /// Returns the index of the field.
    pub fn index(&self) -> usize {
        self.index.unwrap_or(0)
    }
}

/// Stores information about the location and id of a reference in a JSON schema.
#[derive(Debug, Clone, PartialEq, Default, Reflect, Hash, Eq, Ord, PartialOrd)]
pub struct TypeReferencePath {
    /// The location of the reference in the JSON schema.
    pub localization: ReferenceLocation,
    /// The id of the reference.
    pub id: TypeReferenceId,
}

impl TypeReferencePath {
    /// Checks if the reference is local.
    pub fn is_local(&self) -> bool {
        self.localization == ReferenceLocation::Definitions
            || self.localization == ReferenceLocation::Components
    }

    /// Creates a new `TypeReferencePath` with the given type path at the Definitions location.
    pub fn definition(id: impl Into<TypeReferenceId>) -> Self {
        TypeReferencePath::new_ref(ReferenceLocation::Definitions, id)
    }
    /// Creates a new `TypeReferencePath` with the given location and type path.
    pub fn new_ref<I: Into<TypeReferenceId>>(localization: ReferenceLocation, id: I) -> Self {
        TypeReferencePath {
            localization,
            id: id.into(),
        }
    }

    /// Returns the type path of the reference.
    pub fn type_path(&self) -> String {
        self.id.replace("-", "::")
    }

    /// Changes the localization of the reference.
    pub fn change_localization(&mut self, new_localization: ReferenceLocation) {
        if self.localization.eq(&ReferenceLocation::Url) {
            return;
        }
        self.localization = new_localization;
    }
}
impl Display for TypeReferencePath {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.localization, self.id)
    }
}

impl Serialize for TypeReferencePath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}", self))
    }
}

struct TypeReferencePathVisitor;

impl<'de> Visitor<'de> for TypeReferencePathVisitor {
    type Value = TypeReferencePath;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("an string with a '#' prefix")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if let Some(definition) = value.strip_prefix(&ReferenceLocation::Definitions.to_string()) {
            Ok(TypeReferencePath::new_ref(
                ReferenceLocation::Definitions,
                definition,
            ))
        } else if let Some(component) =
            value.strip_prefix(&ReferenceLocation::Components.to_string())
        {
            Ok(TypeReferencePath::new_ref(
                ReferenceLocation::Components,
                component,
            ))
        } else if let Some(component) = value.strip_prefix(&ReferenceLocation::Url.to_string()) {
            Ok(TypeReferencePath::new_ref(
                ReferenceLocation::Url,
                component,
            ))
        } else {
            Err(E::custom("Invalid reference path"))
        }
    }
}
impl<'de> Deserialize<'de> for TypeReferencePath {
    fn deserialize<D>(deserializer: D) -> Result<TypeReferencePath, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(TypeReferencePathVisitor)
    }
}

#[derive(
    Debug, Deserialize, Clone, Copy, PartialEq, Default, Reflect, Hash, Eq, Ord, PartialOrd,
)]
#[serde(rename_all = "lowercase")]
/// Stores information about the location of a reference in a JSON schema.
pub enum ReferenceLocation {
    #[default]
    /// used by json schema
    Definitions,
    /// used by `OpenRPC`
    Components,
    /// used by schemas
    Url,
}

impl Display for ReferenceLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ReferenceLocation::Definitions => write!(f, "#/$defs/"),
            ReferenceLocation::Components => write!(f, "#/components/"),
            ReferenceLocation::Url => write!(f, "https://"),
        }
    }
}

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

/// Extension trait for `BoundValue` that provides methods to retrieve the value based on the bound type.
pub trait BoundValueExt {
    /// Returns the value if this is an inclusive bound, otherwise returns None.
    fn get_inclusive(&self) -> Option<SchemaNumber>;
    /// Returns the value if this is an exclusive bound, otherwise returns None.
    fn get_exclusive(&self) -> Option<SchemaNumber>;
}

impl BoundValueExt for BoundValue {
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

impl BoundValueExt for Option<BoundValue> {
    /// Returns the value if this is an inclusive bound, otherwise returns None.
    fn get_inclusive(&self) -> Option<SchemaNumber> {
        let Some(b) = self else {
            return None;
        };
        b.get_inclusive()
    }
    /// Returns the value if this is an exclusive bound, otherwise returns None.
    fn get_exclusive(&self) -> Option<SchemaNumber> {
        let Some(b) = self else {
            return None;
        };
        b.get_exclusive()
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
    /// Creates `MinMaxValues` from a reflected range type.
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

    /// Creates `MinMaxValues` from range bounds and a type identifier.
    /// Takes a tuple containing start bound, end bound, and `TypeId` to construct
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
        } else if value.eq(&TypeId::of::<u64>())
            || value.eq(&TypeId::of::<usize>())
            || value.eq(&TypeId::of::<u128>())
        {
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
        /// Element type information for the array.
        element_ty: TypeInformation,
        /// Minimum number of elements allowed in the array.
        min_size: Option<u64>,
        /// Maximum number of elements allowed in the array.
        max_size: Option<u64>,
    },
    /// Holds all variants of an enum type.
    EnumHolder(Vec<VariantInfo>),
    /// Represents a single enum variant.
    EnumVariant(VariantInfo),
    /// Holds named fields for struct, tuple, and tuple struct types.
    FieldsHolder(FieldsInformation),
    /// Represents an Optional type (e.g., Option<T>).
    Optional {
        /// Generic information about the wrapped type T in Option<T>.
        generic: GenericInfo,
        /// Schema type information for the wrapped type T in Option<T>.
        schema_type_info: Box<SchemaTypeInfo>,
    },
    /// Represents a Map type (e.g., `HashMap`<K, V>).
    Map {
        /// Information about the key type contained in the map.
        key: TypeInformation,
        /// Information about the value type contained in the map.
        value: TypeInformation,
    },
    /// Variant for regular types.
    RegularType(Type),
    /// Variant for regular primitive types and other simple types.
    Regular(TypeId),
    /// Represents a type with no information.
    #[default]
    NoInfo,
}
impl From<&TypeInformation> for InternalSchemaType {
    fn from(value: &TypeInformation) -> Self {
        if let Some(type_info) = value.try_get_type_info() {
            match type_info {
                TypeInfo::Struct(struct_info) => {
                    let fields = get_fields_information(struct_info.iter());
                    let fields_type = if value.is_forced_as_array() {
                        FieldType::ForceUnnamed
                    } else {
                        FieldType::Named
                    };
                    InternalSchemaType::FieldsHolder(FieldsInformation {
                        fields,
                        fields_type,
                    })
                }
                TypeInfo::TupleStruct(info) => {
                    InternalSchemaType::FieldsHolder(FieldsInformation {
                        fields: get_fields_information(info.iter()),
                        fields_type: FieldType::Unnamed,
                    })
                }
                TypeInfo::Tuple(info) => InternalSchemaType::FieldsHolder(FieldsInformation {
                    fields: get_fields_information(info.iter()),
                    fields_type: FieldType::Unnamed,
                }),
                TypeInfo::Enum(enum_info) => {
                    match TypeInformation::try_get_optional_from_info(enum_info) {
                        Some(e) => InternalSchemaType::Optional {
                            generic: e.clone(),
                            schema_type_info: Box::new(SchemaTypeInfo::default()),
                        },
                        None => InternalSchemaType::EnumHolder(enum_info.iter().cloned().collect()),
                    }
                }

                TypeInfo::List(list_info) => InternalSchemaType::Array {
                    element_ty: (list_info.item_info(), &list_info.item_ty()).into(),
                    min_size: None,
                    max_size: None,
                },
                TypeInfo::Set(set_info) => InternalSchemaType::Array {
                    element_ty: (None, &set_info.value_ty()).into(),
                    min_size: None,
                    max_size: None,
                },
                TypeInfo::Array(array_info) => InternalSchemaType::Array {
                    element_ty: (array_info.item_info(), &array_info.item_ty()).into(),
                    min_size: Some(array_info.capacity() as u64),
                    max_size: Some(array_info.capacity() as u64),
                },
                TypeInfo::Map(map_info) => InternalSchemaType::Map {
                    key: (map_info.key_info(), &map_info.key_ty()).into(),
                    value: (map_info.value_info(), &map_info.value_ty()).into(),
                },
                TypeInfo::Opaque(t) => InternalSchemaType::RegularType(*t.ty()),
            }
        } else {
            match value {
                TypeInformation::VariantInfo(info) => {
                    InternalSchemaType::EnumVariant((**info).clone())
                }
                TypeInformation::Type(ty) => InternalSchemaType::RegularType(*ty.as_ref()),
                TypeInformation::TypeId(type_id) => InternalSchemaType::Regular(*type_id),
                _ => InternalSchemaType::NoInfo,
            }
        }
    }
}
impl From<&SchemaTypeInfo> for InternalSchemaType {
    fn from(value: &SchemaTypeInfo) -> Self {
        if let Some(s) = &value.stored_fields {
            InternalSchemaType::FieldsHolder(s.clone())
        } else {
            (&value.ty_info).into()
        }
    }
}

impl From<&InternalSchemaType> for Option<SchemaTypeVariant> {
    fn from(value: &InternalSchemaType) -> Self {
        match value {
            InternalSchemaType::Array { .. } => Some(SchemaTypeVariant::Single(SchemaType::Array)),
            InternalSchemaType::EnumVariant(variant) => match variant {
                VariantInfo::Tuple(t) => {
                    if t.field_len() == 1 {
                        let s: TypeInformation = t.field_at(0).expect("Should not happened").into();
                        let schema: InternalSchemaType = (&s).into();

                        (&schema).into()
                    } else {
                        Some(SchemaTypeVariant::Single(SchemaType::Array))
                    }
                }
                VariantInfo::Struct(_) => Some(SchemaTypeVariant::Single(SchemaType::Object)),
                VariantInfo::Unit(_) => Some(SchemaTypeVariant::Single(SchemaType::String)),
            },
            InternalSchemaType::FieldsHolder(fields) => match fields.fields_type {
                FieldType::Named => Some(SchemaTypeVariant::Single(SchemaType::Object)),
                FieldType::Unnamed if fields.fields.len() == 1 => {
                    let schema: InternalSchemaType = (&fields.fields[0].type_info).into();
                    (&schema).into()
                }
                _ => Some(SchemaTypeVariant::Single(SchemaType::Array)),
            },
            InternalSchemaType::Optional {
                generic,
                schema_type_info: _,
            } => {
                let schema: InternalSchemaType =
                    (&TypeInformation::Type(Box::new(*generic.ty()))).into();
                let s: Option<SchemaTypeVariant> = (&schema).into();
                Some(
                    s.unwrap_or(SchemaTypeVariant::Single(SchemaType::Object))
                        .with(SchemaType::Null),
                )
            }
            InternalSchemaType::Map { .. } => Some(SchemaTypeVariant::Single(SchemaType::Object)),
            InternalSchemaType::Regular(type_id) => {
                Some(SchemaTypeVariant::Single((*type_id).into()))
            }
            InternalSchemaType::RegularType(ty) => Some(SchemaTypeVariant::Single(ty.id().into())),
            InternalSchemaType::NoInfo | InternalSchemaType::EnumHolder(_) => None,
        }
    }
}

impl From<&SchemaTypeInfo> for Option<SchemaTypeVariant> {
    fn from(value: &SchemaTypeInfo) -> Self {
        let schema: InternalSchemaType = (&value.ty_info).into();
        (&schema).into()
    }
}

impl From<&FieldInformation> for SchemaTypeInfo {
    fn from(value: &FieldInformation) -> Self {
        Self {
            ty_info: value.type_info.clone(),
            field_data: Some(value.field.clone()),
            stored_fields: (&value.type_info).into(),
            reflect_type_data: None,
        }
    }
}

/// Contains comprehensive information about a type's schema representation.
/// This struct aggregates all the necessary information to generate a JSON schema
/// from Rust type information obtained through reflection.
#[derive(Clone, Debug, Default)]
pub struct SchemaDefinition {
    /// The type reference ID of the schema.
    pub id: Option<TypeReferenceId>,
    /// The JSON schema type of the schema.
    pub schema: JsonSchemaBevyType,
    /// The properties of the schema.
    pub definitions: HashMap<TypeReferenceId, SchemaTypeInfo>,
}

impl From<SchemaDefinition> for JsonSchemaBevyType {
    fn from(value: SchemaDefinition) -> Self {
        Self {
            definitions: value
                .definitions
                .iter()
                .map(|(id, schema)| (id.clone(), schema.to_definition().schema.into()))
                .collect(),
            ..value.schema
        }
    }
}

/// Contains comprehensive information about a type's schema representation.
/// This struct aggregates all the necessary information to generate a JSON schema
/// from Rust type information obtained through reflection.
#[derive(Clone, Debug, Default)]
pub struct SchemaTypeInfo {
    /// Information about the type of the schema.
    pub ty_info: TypeInformation,
    /// Field information for the type.
    pub field_data: Option<SchemaFieldData>,
    /// Fields stored in the type.
    pub stored_fields: Option<FieldsInformation>,
    /// Bevy specific field, names of the types that type reflects. Mapping of the names to the data types is provided by [`SchemaTypesMetadata`].
    pub reflect_type_data: Option<Vec<Cow<'static, str>>>,
}

impl SchemaTypeInfo {
    /// Get the documentation for the schema type.
    /// If the field has a description, it is returned.
    /// Otherwise, the documentation from the type information is returned.
    pub fn get_docs(&self) -> Option<Cow<'static, str>> {
        let docs = self
            .field_data
            .as_ref()
            .and_then(|field_data| field_data.description.clone())
            .or(self.ty_info.get_docs());
        docs.map(|docs| docs.trim().replace("\n", "").to_string().into())
    }

    /// Get the range of the schema type.
    /// Starting point is the type's minimum and maximum values, can be further restricted by field data.
    pub fn get_range(&self) -> MinMaxValues {
        let mut min_max = self.ty_info.get_range();
        if let Some(field_data) = &self.field_data {
            let range = field_data
                .attributes
                .get_range_by_id(self.ty_info.type_id());
            if let Some(field_range) = range {
                if field_range.min.is_some() {
                    min_max.min = field_range.min;
                }
                if field_range.max.is_some() {
                    min_max.max = field_range.max;
                }
            }
        }
        min_max
    }
    /// Converts the schema type information into a JSON schema reference.
    pub fn to_ref_schema(&self) -> JsonSchemaBevyType {
        let range = self.get_range();
        let description = self.get_docs();
        let internal_type: InternalSchemaType = (self).into();
        let (ref_type, schema_type) = (
            self.ty_info
                .try_get_type_reference_id()
                .map(TypeReferencePath::definition),
            self.into(),
        );

        let mut schema = JsonSchemaBevyType {
            description,
            minimum: range.min.get_inclusive(),
            maximum: range.max.get_inclusive(),
            exclusive_minimum: range.min.get_exclusive(),
            exclusive_maximum: range.max.get_exclusive(),
            kind: None,
            ref_type,
            schema_type,
            ..default()
        };
        match internal_type {
            InternalSchemaType::Array {
                element_ty,
                min_size,
                max_size,
            } => {
                schema.ref_type = None;
                let items_schema = SchemaTypeInfo {
                    ty_info: element_ty.clone(),
                    field_data: None,
                    stored_fields: None,
                    reflect_type_data: None,
                };
                schema.items = Some(items_schema.to_ref_schema().into());
                schema.min_items = min_size;
                schema.max_items = max_size;
            }
            InternalSchemaType::EnumHolder(_)
            | InternalSchemaType::EnumVariant(_)
            | InternalSchemaType::FieldsHolder(_)
            | InternalSchemaType::Map { key: _, value: _ } => {
                schema.ref_type = None;
            }
            _ => {}
        }

        schema
    }

    /// Converts the schema type information into a JSON schema definition.
    pub fn to_definition(&self) -> SchemaDefinition {
        let mut id: Option<TypeReferenceId> = self.ty_info.try_get_type_reference_id();
        let mut definitions: HashMap<TypeReferenceId, SchemaTypeInfo> = HashMap::new();
        if let Some(custom_schema) = &self.ty_info.try_get_custom_schema() {
            return SchemaDefinition {
                id,
                schema: custom_schema.0.clone(),
                definitions,
            };
        }
        let range = self.ty_info.get_range();

        let (type_path, short_path, crate_name, module_path) =
            if let Some(type_path_table) = self.ty_info.try_get_type_path_table() {
                (
                    type_path_table.path().into(),
                    type_path_table.short_path().into(),
                    type_path_table.crate_name().map(Into::into),
                    type_path_table.module_path().map(Into::into),
                )
            } else {
                (Cow::default(), Cow::default(), None, None)
            };
        let schema_id = id
            .as_ref()
            .map(|id| Cow::Owned(format!("urn:bevy:{}", id)))
            .unwrap_or_default();
        let mut schema = JsonSchemaBevyType {
            id: schema_id,
            description: self.ty_info.get_docs(),
            type_path,
            short_path,
            crate_name,
            module_path,
            kind: Some((&self.ty_info).into()),
            minimum: range.min.get_inclusive(),
            maximum: range.max.get_inclusive(),
            exclusive_minimum: range.min.get_exclusive(),
            exclusive_maximum: range.max.get_exclusive(),
            schema_type: self.into(),
            reflect_type_data: self.reflect_type_data.clone().unwrap_or_default(),
            ..default()
        };
        let internal_type: InternalSchemaType = (self).into();
        match internal_type {
            InternalSchemaType::Map { key, value } => {
                let key: SchemaTypeInfo = SchemaTypeInfo {
                    ty_info: key.clone(),
                    field_data: None,
                    stored_fields: None,
                    reflect_type_data: None,
                };
                let value: SchemaTypeInfo = SchemaTypeInfo {
                    ty_info: value.clone(),
                    field_data: None,
                    stored_fields: None,
                    reflect_type_data: None,
                };
                if !key.ty_info.is_primitive_type() {
                    let SchemaDefinition {
                        id,
                        schema: _,
                        definitions: field_definitions,
                    } = key.to_definition();
                    if let Some(id) = id {
                        definitions.insert(id, key.clone());
                        definitions.extend(field_definitions);
                    }
                }
                if !value.ty_info.is_primitive_type() {
                    let SchemaDefinition {
                        id,
                        schema: _,
                        definitions: field_definitions,
                    } = value.to_definition();
                    if let Some(id) = id {
                        definitions.insert(id, value.clone());
                        definitions.extend(field_definitions);
                    }
                }
                schema.additional_properties = Some(key.to_ref_schema().into());
                schema.value_type = Some(value.to_ref_schema().into());
                schema.key_type = Some(key.to_ref_schema().into());
            }
            InternalSchemaType::Regular(_)
            | InternalSchemaType::RegularType(_)
            | InternalSchemaType::NoInfo => {}
            InternalSchemaType::EnumHolder(variants) => {
                let schema_fields: Vec<(Cow<'static, str>, SchemaDefinition)> = variants
                    .iter()
                    .map(|variant| {
                        (
                            variant.name().into(),
                            SchemaTypeInfo::from(variant).to_definition(),
                        )
                    })
                    .collect();
                schema.one_of = schema_fields
                    .iter()
                    .map(|(_, definition)| definition.schema.clone().into())
                    .collect();
                definitions.extend(schema_fields.iter().flat_map(|s| s.1.definitions.clone()));
            }
            InternalSchemaType::EnumVariant(variant_info) => {
                schema.kind = Some(SchemaKind::Value);
                schema.schema_type = Some(SchemaTypeVariant::Single(SchemaType::Object));
                let ty_info: TypeInformation = (&variant_info).into();
                let field_data: Option<SchemaFieldData> = Some((&variant_info).into());
                id = None;
                match &variant_info {
                    VariantInfo::Struct(struct_variant_info) => {
                        let fields = get_fields_information(struct_variant_info.iter());

                        let schema_field = SchemaTypeInfo {
                            ty_info,
                            field_data,
                            stored_fields: Some(FieldsInformation {
                                fields,
                                fields_type: FieldType::Named,
                            }),
                            reflect_type_data: None,
                        };
                        let definition = schema_field.to_definition();

                        schema.properties =
                            [(variant_info.name().into(), definition.schema.into())].into();
                        schema.required = vec![variant_info.name().into()];
                        definitions.extend(definition.definitions);
                    }
                    VariantInfo::Tuple(tuple_variant_info) => {
                        let stored_fields = get_fields_information(tuple_variant_info.iter());
                        let schema_field = SchemaTypeInfo {
                            ty_info,
                            field_data: None,
                            stored_fields: Some(FieldsInformation {
                                fields: stored_fields,

                                fields_type: FieldType::Unnamed,
                            }),
                            reflect_type_data: None,
                        };
                        let definition = schema_field.to_definition();

                        schema.properties =
                            [(variant_info.name().into(), definition.schema.into())].into();
                        definitions.extend(definition.definitions);
                        schema.required = vec![variant_info.name().into()];
                    }
                    VariantInfo::Unit(unit_variant_info) => {
                        let schema_field = SchemaTypeInfo {
                            ty_info,
                            field_data,
                            stored_fields: None,
                            reflect_type_data: None,
                        };
                        return SchemaDefinition {
                            id: None,
                            schema: JsonSchemaBevyType {
                                const_value: Some(unit_variant_info.name().to_string().into()),
                                schema_type: Some(SchemaTypeVariant::Single(SchemaType::String)),
                                description: schema_field.get_docs(),
                                kind: Some(SchemaKind::Value),
                                ..Default::default()
                            },
                            definitions: HashMap::new(),
                        };
                    }
                }
            }
            InternalSchemaType::FieldsHolder(fields) => match fields.fields_type {
                FieldType::Named => {
                    schema.additional_properties = Some(JsonSchemaVariant::BoolValue(false));
                    schema.schema_type = Some(SchemaTypeVariant::Single(SchemaType::Object));
                    let schema_fields: Vec<(Cow<'static, str>, SchemaTypeInfo)> = fields
                        .fields
                        .iter()
                        .map(|field| (field.field.to_name(), SchemaTypeInfo::from(field)))
                        .collect();
                    schema.properties = schema_fields
                        .iter()
                        .map(|(name, schema)| (name.clone(), schema.to_ref_schema().into()))
                        .collect();
                    for (_, field_schema) in schema_fields {
                        if field_schema.ty_info.is_primitive_type() {
                            continue;
                        }
                        let SchemaDefinition {
                            id,
                            schema: _,
                            definitions: field_definitions,
                        } = field_schema.to_definition();
                        definitions.extend(field_definitions);
                        let Some(id) = id else { continue };
                        if !definitions.contains_key(&id) {
                            definitions.insert(id, field_schema);
                        }
                    }
                    schema.required = fields
                        .fields
                        .iter()
                        .map(|field| field.field.to_name())
                        .collect();
                }
                FieldType::Unnamed if fields.fields.len() == 1 => {
                    let field_schema = SchemaTypeInfo::from(&fields.fields[0]);
                    let SchemaDefinition {
                        id,
                        schema: new_schema_type,
                        definitions: field_definitions,
                    } = field_schema.to_definition();
                    definitions.extend(field_definitions);
                    if let Some(id) = id {
                        definitions.insert(id, field_schema);
                    }
                    schema = new_schema_type;
                    schema.schema_type = self.into();
                    schema.description = self.get_docs();
                }
                s => {
                    let schema_fields: Vec<SchemaTypeInfo> =
                        fields.fields.iter().map(SchemaTypeInfo::from).collect();
                    schema.prefix_items = schema_fields
                        .iter()
                        .map(|field| {
                            let field_schema = if s == FieldType::ForceUnnamed
                                && field
                                    .field_data
                                    .as_ref()
                                    .is_some_and(|f| f.description.is_none())
                            {
                                if let Some(field_data) = field.field_data.as_ref() {
                                    let description = field_data.name.clone();
                                    SchemaTypeInfo {
                                        field_data: Some(SchemaFieldData {
                                            description,
                                            ..field_data.clone()
                                        }),
                                        ..field.clone()
                                    }
                                    .to_ref_schema()
                                } else {
                                    field.to_ref_schema()
                                }
                            } else {
                                field.to_ref_schema()
                            };

                            field_schema.into()
                        })
                        .collect();
                    for field_schema in schema_fields {
                        if field_schema.ty_info.is_primitive_type() {
                            continue;
                        }
                        let SchemaDefinition {
                            id,
                            schema: _,
                            definitions: field_definitions,
                        } = field_schema.to_definition();
                        definitions.extend(field_definitions);
                        let Some(id) = id else { continue };
                        if !definitions.contains_key(&id) {
                            definitions.insert(id, field_schema);
                        }
                    }
                    schema.min_items = Some(fields.fields.len() as u64);
                    schema.max_items = Some(fields.fields.len() as u64);
                }
            },
            InternalSchemaType::Array {
                element_ty,
                min_size,
                max_size,
            } => {
                id = None;
                let items_schema = SchemaTypeInfo {
                    ty_info: element_ty.clone(),
                    field_data: None,
                    stored_fields: None,
                    reflect_type_data: None,
                };
                schema.items = Some(items_schema.to_ref_schema().into());
                schema.min_items = min_size;
                schema.max_items = max_size;

                if !items_schema.ty_info.is_primitive_type() {
                    let SchemaDefinition {
                        id,
                        schema: _,
                        definitions: field_definitions,
                    } = items_schema.to_definition();
                    definitions.extend(field_definitions);
                    if let Some(id) = id {
                        definitions.insert(id, items_schema);
                    }
                }
            }
            InternalSchemaType::Optional {
                generic,
                ref schema_type_info,
            } => {
                let schema_optional = SchemaTypeInfo {
                    ty_info: TypeInformation::Type(Box::new(*generic.ty())),
                    ..(**schema_type_info).clone()
                };
                return schema_optional.to_definition();
            }
        }
        SchemaDefinition {
            id,
            schema,
            definitions,
        }
    }
}

/// Traits for getting attribute information from a reflected value.
pub trait AttributeInfoReflect {
    /// Try to get the attribute by id
    fn try_get_attribute_by_id(&self, _id: ::core::any::TypeId) -> Option<&dyn Reflect>;

    /// Creates `MinMaxValues` from a reflected range type.
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

    /// Creates `MinMaxValues` from a reflected range type.
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

    /// Creates `MinMaxValues` from a reflected range type.
    /// Attempts to downcast the reflected value to the specified range type T
    /// and extract its bounds.
    fn get_range_by_id(&self, t: TypeId) -> Option<MinMaxValues> {
        if t.eq(&TypeId::of::<u8>()) {
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
        } else if t.eq(&TypeId::of::<f32>()) {
            self.min_max_from_attribute_for_type::<f32>()
        } else if t.eq(&TypeId::of::<f64>()) {
            self.min_max_from_attribute_for_type::<f64>()
        } else {
            None
        }
    }
}

impl<'a, T> From<&'a T> for SchemaTypeInfo
where
    T: 'static,
    TypeInformation: From<&'a T>,
    SchemaFieldData: From<&'a T>,
{
    fn from(value: &'a T) -> Self {
        let ty_info: TypeInformation = value.into();
        let field_data: SchemaFieldData = value.into();
        let stored_fields = (&ty_info).into();
        SchemaTypeInfo {
            ty_info,
            field_data: Some(field_data),
            stored_fields,
            reflect_type_data: None,
        }
    }
}

/// Builds a JSON schema variant from a value.
pub fn build_schema<'a, T>(value: &'a T) -> JsonSchemaVariant
where
    T: 'static,
    SchemaTypeInfo: From<&'a T>,
{
    let schema: SchemaTypeInfo = value.into();
    schema.to_definition().schema.into()
}

impl From<&UnnamedField> for SchemaFieldData {
    fn from(value: &UnnamedField) -> Self {
        let attributes: AttributesInformation = value.custom_attributes().into();
        #[cfg(feature = "documentation")]
        let description = value.docs().map(|s| Cow::Owned(s.to_owned()));
        #[cfg(not(feature = "documentation"))]
        let description = None;
        SchemaFieldData {
            attributes,
            name: None,
            index: Some(value.index()),
            description,
        }
    }
}
impl From<&NamedField> for SchemaFieldData {
    fn from(value: &NamedField) -> Self {
        let attributes: AttributesInformation = value.custom_attributes().into();
        #[cfg(feature = "documentation")]
        let description = value.docs().map(|s| Cow::Owned(s.to_owned()));
        #[cfg(not(feature = "documentation"))]
        let description = None;
        SchemaFieldData {
            name: Some(value.name().into()),
            index: None,
            description,
            attributes,
        }
    }
}

impl From<(Option<&'static TypeInfo>, &Type)> for TypeInformation {
    fn from(value: (Option<&'static TypeInfo>, &Type)) -> Self {
        match value.0 {
            Some(info) => TypeInformation::TypeInfo(Box::new(info.clone())),
            None => (*value.1).into(),
        }
    }
}
impl From<&NamedField> for TypeInformation {
    fn from(value: &NamedField) -> Self {
        (value.type_info(), value.ty()).into()
    }
}
impl From<&UnnamedField> for TypeInformation {
    fn from(value: &UnnamedField) -> Self {
        (value.type_info(), value.ty()).into()
    }
}

impl From<&VariantInfo> for TypeInformation {
    fn from(value: &VariantInfo) -> Self {
        TypeInformation::VariantInfo(Box::new(value.clone()))
    }
}
impl From<&VariantInfo> for SchemaFieldData {
    fn from(value: &VariantInfo) -> Self {
        #[cfg(feature = "documentation")]
        let description = value.docs().map(|s| Cow::Owned(s.to_owned()));
        #[cfg(not(feature = "documentation"))]
        let description = None;
        SchemaFieldData {
            name: Some(value.name().to_owned().into()),
            index: None,
            description,
            attributes: value.custom_attributes().into(),
        }
    }
}

impl From<&TypeInfo> for TypeInformation {
    fn from(value: &TypeInfo) -> Self {
        TypeInformation::TypeInfo(Box::new(value.clone()))
    }
}

impl From<&TypeRegistration> for TypeInformation {
    fn from(value: &TypeRegistration) -> Self {
        TypeInformation::TypeRegistration(value.clone())
    }
}

impl From<Type> for TypeInformation {
    fn from(value: Type) -> Self {
        TypeInformation::Type(Box::new(value))
    }
}
impl From<&TypeId> for TypeInformation {
    fn from(value: &TypeId) -> Self {
        TypeInformation::TypeId(*value)
    }
}
impl From<TypeId> for TypeInformation {
    fn from(value: TypeId) -> Self {
        TypeInformation::TypeId(value)
    }
}

fn get_fields_information<'a, 'b, T>(iterator: Iter<'a, T>) -> Vec<FieldInformation>
where
    SchemaFieldData: From<&'a T>,
    TypeInformation: From<&'a T>,
{
    iterator
        .enumerate()
        .map(|(index, field)| FieldInformation {
            field: SchemaFieldData {
                index: Some(index),
                ..field.into()
            },
            type_info: field.into(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use bevy_platform::collections::HashMap;
    use bevy_reflect::GetTypeRegistration;

    use super::*;

    #[test]
    fn integer_test() {
        let type_info = TypeInformation::from(&TypeId::of::<u16>()).to_schema_type_info();
        let schema_type: Option<SchemaTypeVariant> = (&type_info).into();
        assert_eq!(
            type_info.get_range().min,
            Some(BoundValue::Inclusive(0.into()))
        );
        assert_eq!(
            type_info.get_range().max,
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
        let type_info = SchemaTypeInfo::from(field_info);
        let schema_type: Option<SchemaTypeVariant> = (&type_info).into();
        let range = type_info.get_range();
        assert_eq!(range.min, Some(BoundValue::Inclusive(10.into())));
        assert_eq!(range.max, Some(BoundValue::Inclusive(13.into())));
        assert_eq!(
            schema_type,
            Some(SchemaTypeVariant::Single(SchemaType::Integer))
        );
        assert_eq!(type_info.get_docs(), Some("Test documentation".into()));
    }

    #[test]
    fn custom_range_test_usize() {
        #[derive(Reflect, Default, Deserialize, Serialize)]
        pub struct StructTest {
            /// Test documentation
            #[reflect(@..13_usize)]
            no_value: usize,
        }
        let struct_info = StructTest::get_type_registration()
            .type_info()
            .as_struct()
            .expect("Should not fail");
        let field_info = struct_info.field("no_value").expect("Should not fail");
        let type_info = SchemaTypeInfo::from(field_info);
        let schema_type: Option<SchemaTypeVariant> = (&type_info).into();
        let range = type_info.get_range();
        assert!(!range.in_range((-1).into()));
        assert!(range.in_range(0.into()));
        assert!(range.in_range(12.into()));
        assert!(!range.in_range(13.into()));
        assert_eq!(range.min, Some(BoundValue::Inclusive(0.into())));
        assert_eq!(range.max, Some(BoundValue::Exclusive(13.into())));
        assert_eq!(
            schema_type,
            Some(SchemaTypeVariant::Single(SchemaType::Integer))
        );
        assert_eq!(type_info.get_docs(), Some("Test documentation".into()));
    }
    #[test]
    fn other_ss_test() {
        #[derive(Reflect, Default, Deserialize, Serialize)]
        struct Foo {
            /// Test doc
            a: u16,
        }
        let atr = bevy_ecs::reflect::AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<bevy_math::Vec3>();
            register.register_type_data::<bevy_math::Vec3, ReflectJsonSchemaForceAsArray>();
        }
        let type_registry = atr.read();
        let declaration = TypeInformation::from(
            type_registry
                .get(TypeId::of::<bevy_math::Vec3>())
                .expect(""),
        )
        .to_schema_type_info();
        let _: JsonSchemaBevyType = declaration.to_definition().into();
    }

    #[test]
    fn custom_tuple_test_usize() {
        #[derive(Reflect, Default, Deserialize, Serialize)]
        pub struct TupleTest(
            /// Test documentation
            #[reflect(@..13_usize)]
            pub usize,
        );

        let struct_info = TupleTest::get_type_registration()
            .type_info()
            .as_tuple_struct()
            .expect("Should not fail");
        let field_info = struct_info.iter().next().expect("Should not fail");
        let type_info = SchemaTypeInfo::from(field_info);
        let schema_type: Option<SchemaTypeVariant> = (&type_info).into();
        let range = type_info.get_range();
        assert_eq!(range.min, Some(BoundValue::Inclusive(0.into())));
        assert_eq!(range.max, Some(BoundValue::Exclusive(13.into())));
        assert_eq!(
            schema_type,
            Some(SchemaTypeVariant::Single(SchemaType::Integer))
        );
        assert_eq!(type_info.get_docs(), Some("Test documentation".into()));
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
    }

    #[test]
    fn reflect_struct_with_array() {
        #[derive(Reflect, Default, Deserialize, Serialize)]
        pub struct ArrayComponent {
            pub array: [i32; 3],
        }
        let type_info =
            TypeInformation::from(&ArrayComponent::get_type_registration()).to_schema_type_info();
        let _: JsonSchemaBevyType = type_info.to_definition().into();
    }

    #[test]
    fn reflect_multiple_definitions() {
        #[derive(Reflect, Default, Deserialize, Serialize)]
        pub struct BaseStruct {
            pub base_field: i32,
            pub second_field: i32,
        }
        #[derive(Reflect, Default, Deserialize, Serialize)]
        pub struct ArrayComponent {
            pub array: [BaseStruct; 3],
        }
        #[derive(Reflect, Default, Deserialize, Serialize)]
        pub struct ArrayComponentWithMoreVariants {
            pub array: [BaseStruct; 3],
            pub list: Vec<BaseStruct>,
            pub optional: Option<BaseStruct>,
        }
        let type_info = TypeInformation::from(&ArrayComponent::get_type_registration())
            .to_schema_type_info()
            .to_definition();
        let type_info_second =
            TypeInformation::from(&ArrayComponentWithMoreVariants::get_type_registration())
                .to_schema_type_info()
                .to_definition();
        assert_eq!(
            type_info.definitions.len(),
            type_info_second.definitions.len()
        );
        // let schema: JsonSchemaBevyType = type_info_second.into();
        // eprintln!("{}", serde_json::to_string_pretty(&schema).expect(""));
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
        let type_info =
            TypeInformation::from(&HashMapStruct::get_type_registration()).to_schema_type_info();
        let _: JsonSchemaBevyType = type_info.to_definition().into();
    }

    #[test]
    fn reflect_nested_struct() {
        #[derive(Reflect, Default, Deserialize, Serialize)]
        pub struct OtherStruct {
            pub field: String,
        }
        #[derive(Reflect, Default, Deserialize, Serialize)]
        pub struct SecondStruct {
            pub field: String,
            pub other: OtherStruct,
        }
        #[derive(Reflect, Default, Deserialize, Serialize)]
        pub struct ThirdStruct {
            pub array_strings: Vec<String>,
            pub array_structs: [OtherStruct; 5],
            pub map_strings: HashMap<String, i32>,
        }
        #[derive(Reflect, Default, Deserialize, Serialize)]
        pub struct NestedStruct {
            pub other: OtherStruct,
            pub second: SecondStruct,
            pub third: ThirdStruct,
        }
        let type_info =
            TypeInformation::from(&NestedStruct::get_type_registration()).to_schema_type_info();
        let _s: JsonSchemaBevyType = type_info.to_definition().into();
        // eprintln!("{}", serde_json::to_string_pretty(&s).expect("msg"));
        // eprintln!(
        //     "{}",
        //     serde_json::to_string_pretty(&NestedStruct::default()).expect("msg")
        // );
    }
}
