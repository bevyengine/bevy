//! Module containing information about reflected types.
use alloc::borrow::Cow;
use alloc::sync::Arc;
use bevy_derive::{Deref, DerefMut};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_reflect::attributes::CustomAttributes;
use bevy_reflect::{
    EnumInfo, GenericInfo, NamedField, Reflect, Type, TypeInfo, TypePathTable, TypeRegistration,
    TypeRegistry, UnnamedField, VariantInfo,
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
use std::num::{
    NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU128,
    NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
};

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
pub(crate) struct FieldInformation {
    /// Field specific data
    field: SchemaFieldData,
    /// Type information of the field.
    type_info: TypeInformation,
}

/// Information about the field type.
#[derive(Clone, Debug, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub(crate) enum FieldType {
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
pub(crate) struct FieldsInformation {
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
pub(crate) struct AttributesInformation(Arc<TypeIdMap<Box<dyn Reflect>>>);

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
pub(crate) enum TypeInformation {
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
    /// Find the type registration in the registry.
    pub fn find_in_registry<'a>(&self, registry: &'a TypeRegistry) -> Option<&'a TypeRegistration> {
        match self.try_get_type_path_table() {
            Some(path_table) => registry.get_with_type_path(path_table.path()),
            None => registry.get(self.type_id()),
        }
    }

    /// Try to get a regex pattern for the type.
    pub fn try_get_regex_for_type(&self) -> Option<Cow<'static, str>> {
        let primitive_type = self.try_get_primitive_type()?;
        let pattern: Option<Cow<'static, str>> = match primitive_type {
            SchemaType::String => Some(".*".into()),
            SchemaType::Number => Some("\\d+(?:\\.'\\d+)?".into()),
            SchemaType::Integer => Some("^(0|-*[1-9]+[0-9]*)$".into()),
            _ => None,
        };
        pattern
    }

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
            if schema.0.type_path.is_empty() {
                None
            } else {
                Some(TypeReferenceId::from(&*schema.0.type_path))
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
        let internal_schema_type = (&self).into();
        SchemaTypeInfo {
            ty_info: self,
            field_data: None,
            internal_schema_type,
            reflect_type_data: None,
        }
    }
    /// Converts the type information into a schema type information.
    pub fn to_schema_type_info_with_metadata(
        self,
        metadata: &SchemaTypesMetadata,
    ) -> SchemaTypeInfo {
        let reflect_type_data = if let Self::TypeRegistration(reg) = &self {
            Some(metadata.get_registered_reflect_types(reg))
        } else {
            None
        };
        let internal_schema_type = (&self).into();
        SchemaTypeInfo {
            ty_info: self,
            field_data: None,
            internal_schema_type,
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
            TypeInformation::TypeRegistration(type_registration) => {
                type_registration.type_info().type_id()
            }
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
pub(crate) struct SchemaFieldData {
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
        serializer.serialize_str(&format!("{self}"))
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
    /// Used by JSON Schema
    Urn,
}

impl Display for ReferenceLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ReferenceLocation::Definitions => write!(f, "#/$defs/"),
            ReferenceLocation::Components => write!(f, "#/components/"),
            ReferenceLocation::Url => write!(f, "https://"),
            ReferenceLocation::Urn => write!(f, "urn:"),
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

impl From<&JsonSchemaBevyType> for MinMaxValues {
    fn from(value: &JsonSchemaBevyType) -> Self {
        let min = match (&value.exclusive_minimum, &value.minimum) {
            (Some(ex), None) => Some(BoundValue::Exclusive(ex.clone())),
            (_, Some(inclusive)) => Some(BoundValue::Inclusive(*inclusive)),
            _ => None,
        };
        let max = match (&value.exclusive_maximum, &value.maximum) {
            (Some(ex), None) => Some(BoundValue::Exclusive(ex.clone())),
            (_, Some(inclusive)) => Some(BoundValue::Inclusive(*inclusive)),
            _ => None,
        };
        MinMaxValues { min, max }
    }
}

impl MinMaxValues {
    /// Checks if a given value falls within the defined range constraints.
    /// Returns true if the value is within bounds, false otherwise.
    pub fn in_range(&self, value: impl Into<SchemaNumber>) -> bool {
        let value = value.into();
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
        if value.eq(&TypeId::of::<NonZeroU8>()) {
            min = Some(BoundValue::Inclusive(1.into()));
            max = Some(BoundValue::Inclusive(u8::MAX.into()));
        } else if value.eq(&TypeId::of::<u8>()) {
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
        } else if value.eq(&TypeId::of::<i8>()) || value.eq(&TypeId::of::<NonZeroI8>()) {
            min = Some(BoundValue::Inclusive(i8::MIN.into()));
            max = Some(BoundValue::Inclusive(i8::MAX.into()));
        } else if value.eq(&TypeId::of::<i16>()) || value.eq(&TypeId::of::<NonZeroI16>()) {
            min = Some(BoundValue::Inclusive(i16::MIN.into()));
            max = Some(BoundValue::Inclusive(i16::MAX.into()));
        } else if value.eq(&TypeId::of::<i32>()) || value.eq(&TypeId::of::<NonZeroI32>()) {
            min = Some(BoundValue::Inclusive(i32::MIN.into()));
            max = Some(BoundValue::Inclusive(i32::MAX.into()));
        }
        MinMaxValues { min, max }
    }
}

pub(super) fn is_non_zero_number_type(t: TypeId) -> bool {
    t.eq(&TypeId::of::<NonZeroI8>())
        || t.eq(&TypeId::of::<NonZeroI16>())
        || t.eq(&TypeId::of::<NonZeroI32>())
        || t.eq(&TypeId::of::<NonZeroI64>())
        || t.eq(&TypeId::of::<NonZeroI128>())
        || t.eq(&TypeId::of::<NonZeroIsize>())
        || t.eq(&TypeId::of::<NonZeroU8>())
        || t.eq(&TypeId::of::<NonZeroU16>())
        || t.eq(&TypeId::of::<NonZeroU32>())
        || t.eq(&TypeId::of::<NonZeroU64>())
        || t.eq(&TypeId::of::<NonZeroU128>())
        || t.eq(&TypeId::of::<NonZeroUsize>())
}

/// Enum representing the internal schema type information for different Rust types.
/// This enum categorizes how different types should be represented in JSON schema.
#[derive(Clone, Debug, Default)]
pub(crate) enum InternalSchemaType {
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
    /// Represents an Optional type (e.g., `Option<T>`).
    Optional {
        /// Generic information about the wrapped type `T` in `Option<T>`.
        generic: GenericInfo,
        /// Schema type information for the wrapped type `T` in `Option<T>`.
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

impl InternalSchemaType {
    /// Returns the dependencies of the type.
    pub(super) fn get_dependencies(&self, registry: &TypeRegistry) -> HashSet<TypeId> {
        let mut dependencies = HashSet::new();
        match &self {
            InternalSchemaType::Array {
                element_ty,
                min_size: _,
                max_size: _,
            } => {
                if let Some(reg) = element_ty.find_in_registry(registry) {
                    let info = TypeInformation::from(reg);
                    if !info.is_primitive_type() {
                        let subschema: InternalSchemaType = (&info).into();
                        dependencies.insert(reg.type_id());
                        dependencies.extend(subschema.get_dependencies(registry));
                    }
                }
            }
            InternalSchemaType::EnumHolder(variant_infos) => {
                for variant_info in variant_infos {
                    let sub_schema = InternalSchemaType::EnumVariant(variant_info.clone());
                    dependencies.extend(sub_schema.get_dependencies(registry));
                }
            }
            InternalSchemaType::EnumVariant(variant_info) => match variant_info {
                VariantInfo::Struct(struct_variant_info) => {
                    for field in struct_variant_info.iter() {
                        if let Some(reg) = registry.get(field.type_id()) {
                            let info = TypeInformation::from(reg);
                            if !info.is_primitive_type() {
                                let subschema: InternalSchemaType = (&info).into();
                                dependencies.insert(info.type_id());
                                dependencies.extend(subschema.get_dependencies(registry));
                            }
                        }
                    }
                }
                VariantInfo::Tuple(tuple_variant_info) => {
                    for field in tuple_variant_info.iter() {
                        if let Some(reg) = registry.get(field.type_id()) {
                            let info = TypeInformation::from(reg);
                            if !info.is_primitive_type() {
                                let subschema: InternalSchemaType = (&info).into();
                                dependencies.insert(info.type_id());
                                dependencies.extend(subschema.get_dependencies(registry));
                            }
                        }
                    }
                }
                VariantInfo::Unit(_) => {}
            },
            InternalSchemaType::FieldsHolder(fields_information) => {
                for field in fields_information.iter() {
                    let Some(reg) = field.type_info.find_in_registry(registry) else {
                        continue;
                    };
                    if SchemaType::try_get_primitive_type_from_type_id(reg.type_id()).is_some() {
                        continue;
                    }
                    let info = TypeInformation::from(reg);
                    let subschema: InternalSchemaType = (&info).into();
                    dependencies.insert(info.type_id());
                    dependencies.extend(subschema.get_dependencies(registry));
                }
            }
            InternalSchemaType::Optional {
                generic,
                schema_type_info: _,
            } => {
                if let Some(reg) = registry.get(generic.type_id()) {
                    let info = TypeInformation::from(reg);
                    if !info.is_primitive_type() {
                        let subschema: InternalSchemaType = (&info).into();
                        dependencies.insert(info.type_id());
                        dependencies.extend(subschema.get_dependencies(registry));
                    }
                }
            }
            InternalSchemaType::Map { key, value } => {
                if let Some(reg) = registry.get(key.type_id()) {
                    let info = TypeInformation::from(reg);
                    if !info.is_primitive_type() {
                        let subschema: InternalSchemaType = (&info).into();
                        dependencies.insert(info.type_id());
                        dependencies.extend(subschema.get_dependencies(registry));
                    }
                }
                if let Some(reg) = registry.get(value.type_id()) {
                    let info = TypeInformation::from(reg);
                    if !info.is_primitive_type() {
                        let subschema: InternalSchemaType = (&info).into();
                        dependencies.insert(info.type_id());
                        dependencies.extend(subschema.get_dependencies(registry));
                    }
                }
            }
            InternalSchemaType::RegularType(ty) => {
                _ = dependencies.insert(ty.type_id());
            }
            InternalSchemaType::Regular(t) => {
                _ = dependencies.insert(*t);
            }
            InternalSchemaType::NoInfo => {}
        }
        dependencies
    }
}

impl From<&TypeInformation> for InternalSchemaType {
    fn from(value: &TypeInformation) -> Self {
        let field_information: Option<FieldsInformation> = value.into();
        if let Some(fields_info) = field_information {
            return InternalSchemaType::FieldsHolder(fields_info);
        }
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
            InternalSchemaType::Map { .. } => Some(SchemaTypeVariant::Single(SchemaType::Object)),
            InternalSchemaType::Regular(type_id) => {
                Some(SchemaTypeVariant::Single((*type_id).into()))
            }
            InternalSchemaType::RegularType(ty) => Some(SchemaTypeVariant::Single(ty.id().into())),
            InternalSchemaType::NoInfo
            | InternalSchemaType::EnumHolder(_)
            | InternalSchemaType::Optional {
                generic: _,
                schema_type_info: _,
            } => None,
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
            internal_schema_type: (&value.type_info).into(),
            reflect_type_data: None,
        }
    }
}

/// Contains comprehensive information about a type's schema representation.
/// This struct aggregates all the necessary information to generate a JSON schema
/// from Rust type information obtained through reflection.
#[derive(Clone, Debug, Default)]
pub(crate) struct SchemaDefinition {
    /// The type reference ID of the schema.
    pub id: Option<TypeReferenceId>,
    /// The JSON schema type of the schema.
    pub schema: JsonSchemaBevyType,
    /// The properties of the schema.
    pub definitions: HashMap<TypeReferenceId, SchemaTypeInfo>,
    /// Missing definitions of the schema.
    /// Could be the case for the types that are stored as generic arguments.
    pub dependencies: Vec<TypeId>,
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
pub(crate) struct SchemaTypeInfo {
    /// Information about the type of the schema.
    pub ty_info: TypeInformation,
    /// Field information for the type.
    pub field_data: Option<SchemaFieldData>,
    /// Bevy specific field, names of the types that type reflects. Mapping of the names to the data types is provided by [`SchemaTypesMetadata`].
    pub reflect_type_data: Option<Vec<Cow<'static, str>>>,
    /// Internal schema type information.
    pub internal_schema_type: InternalSchemaType,
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
        let ref_type = self
            .ty_info
            .try_get_type_reference_id()
            .map(TypeReferencePath::definition);

        // If there is reference specified it is not need for specifying type
        let schema_type = if ref_type.is_none() {
            self.into()
        } else {
            None
        };

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
        match self.internal_schema_type.clone() {
            InternalSchemaType::Array {
                element_ty,
                min_size,
                max_size,
            } => {
                schema.ref_type = None;
                schema.schema_type = Some(SchemaTypeVariant::Single(SchemaType::Array));
                let items_schema = element_ty.to_schema_type_info();
                schema.items = Some(items_schema.to_ref_schema().into());
                schema.min_items = min_size;
                schema.max_items = max_size;
            }
            InternalSchemaType::Map { key, value } => {
                schema.ref_type = None;
                schema.schema_type = Some(SchemaTypeVariant::Single(SchemaType::Object));
                schema.kind = Some(SchemaKind::Map);
                let key_info = key.clone().to_schema_type_info();
                schema.key_type = Some(key_info.to_ref_schema().into());
                let value_info = value.clone().to_schema_type_info();
                schema.value_type = Some(value_info.to_ref_schema().into());

                if let Some(p) = key.try_get_regex_for_type() {
                    schema.pattern_properties = [(p, value_info.to_ref_schema().into())].into();
                    schema.additional_properties = Some(JsonSchemaVariant::BoolValue(false));
                }
            }
            InternalSchemaType::EnumVariant(_) => {
                schema.ref_type = None;
            }
            InternalSchemaType::Optional {
                generic,
                ref schema_type_info,
            } => {
                let schema_optional = SchemaTypeInfo {
                    ty_info: TypeInformation::Type(Box::new(*generic.ty())),
                    ..(**schema_type_info).clone()
                };
                schema.ref_type = None;
                schema.one_of = vec![
                    Box::new(JsonSchemaBevyType {
                        schema_type: Some(SchemaTypeVariant::Single(SchemaType::Null)),
                        ..Default::default()
                    }),
                    Box::new(schema_optional.to_ref_schema()),
                ];
            }
            _ => {
                if let Some(primitive) = self.ty_info.try_get_primitive_type() {
                    schema.not = if is_non_zero_number_type(self.ty_info.type_id()) {
                        Some(Box::new(JsonSchemaBevyType {
                            const_value: Some(0.into()),
                            ..default()
                        }))
                    } else {
                        None
                    };

                    schema.type_path = self
                        .ty_info
                        .try_get_type_path_table()
                        .map(|t| Cow::Owned(t.path().to_string()))
                        .unwrap_or_default();

                    schema.kind = Some(SchemaKind::Value);
                    schema.schema_type = Some(SchemaTypeVariant::Single(primitive));
                }
            }
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
                dependencies: Default::default(),
            };
        }
        let mut dependencies: HashSet<TypeId> = HashSet::new();
        let range = self.get_range();

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

        let not = if is_non_zero_number_type(self.ty_info.type_id()) {
            Some(Box::new(JsonSchemaBevyType {
                const_value: Some(0.into()),
                ..default()
            }))
        } else {
            None
        };

        let mut schema = JsonSchemaBevyType {
            description: self.ty_info.get_docs(),
            not,
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
        match self.internal_schema_type.clone() {
            InternalSchemaType::Map { key, value } => {
                let key = key.to_schema_type_info();
                let value = value.to_schema_type_info();
                if key.ty_info.try_get_primitive_type().is_some() {
                    if let Some(p) = key.ty_info.try_get_regex_for_type() {
                        schema.pattern_properties = [(p, value.to_ref_schema().into())].into();
                        schema.additional_properties = Some(JsonSchemaVariant::BoolValue(false));
                    }
                } else {
                    {
                        let SchemaDefinition {
                            id,
                            schema: _,
                            definitions: field_definitions,
                            dependencies: key_dependencies,
                        } = key.to_definition();
                        dependencies.extend(key_dependencies);
                        if let Some(id) = id {
                            definitions.insert(id, key.clone());
                            definitions.extend(field_definitions);
                        }
                    }
                }
                if !value.ty_info.is_primitive_type() {
                    let SchemaDefinition {
                        id,
                        schema: _,
                        definitions: field_definitions,
                        dependencies: value_dependencies,
                    } = value.to_definition();

                    dependencies.extend(value_dependencies);
                    if let Some(id) = id {
                        definitions.insert(id, value.clone());
                        definitions.extend(field_definitions);
                    }
                }
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
                            internal_schema_type: InternalSchemaType::FieldsHolder(
                                FieldsInformation {
                                    fields,
                                    fields_type: FieldType::Named,
                                },
                            ),
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
                            internal_schema_type: InternalSchemaType::FieldsHolder(
                                FieldsInformation {
                                    fields: stored_fields,

                                    fields_type: FieldType::Unnamed,
                                },
                            ),
                            reflect_type_data: None,
                        };
                        let definition = schema_field.to_definition();

                        schema.properties =
                            [(variant_info.name().into(), definition.schema.into())].into();
                        definitions.extend(definition.definitions);
                        schema.required = vec![variant_info.name().into()];
                    }
                    VariantInfo::Unit(unit_variant_info) => {
                        let internal_schema_type = (&ty_info).into();
                        let schema_field = SchemaTypeInfo {
                            ty_info,
                            field_data,
                            internal_schema_type,
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
                            dependencies: dependencies.iter().cloned().collect(),
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
                        dependencies.insert(field_schema.ty_info.type_id());
                        // let SchemaDefinition {
                        //     id,
                        //     schema: _,
                        //     definitions: field_definitions,
                        //     missing_definitions: field_missing_definitions,
                        // } = field_schema.to_definition();
                        // missing_definitions.extend(field_missing_definitions);
                        // definitions.extend(field_definitions);
                        // let Some(id) = id else { continue };
                        // if !definitions.contains_key(&id) {
                        //     definitions.insert(id, field_schema);
                        // }
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
                        dependencies: field_dependencies,
                    } = field_schema.to_definition();
                    dependencies.extend(field_dependencies);
                    definitions.extend(field_definitions);
                    if let Some(id) = id {
                        definitions.insert(id.clone(), field_schema);
                        schema.ref_type = Some(TypeReferencePath::definition(id));
                    } else {
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
                        schema = JsonSchemaBevyType {
                            short_path,
                            type_path,
                            module_path,
                            crate_name,
                            kind: Some(SchemaKind::TupleStruct),
                            schema_type: self.into(),
                            description: self.get_docs(),
                            ..new_schema_type
                        };
                    }
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
                            dependencies: field_dependencies,
                        } = field_schema.to_definition();
                        definitions.extend(field_definitions);
                        dependencies.extend(field_dependencies);
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
                let items_schema = element_ty.to_schema_type_info();
                schema.items = Some(items_schema.to_ref_schema().into());
                schema.min_items = min_size;
                schema.max_items = max_size;

                if !items_schema.ty_info.is_primitive_type() {
                    let SchemaDefinition {
                        id,
                        schema: _,
                        definitions: field_definitions,
                        dependencies: field_dependencies,
                    } = items_schema.to_definition();
                    dependencies.extend(field_dependencies);
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
                if SchemaType::try_get_primitive_type_from_type_id(generic.type_id()).is_none() {
                    dependencies.insert(generic.type_id());
                }
                schema.ref_type = None;
                schema.schema_type = None;
                schema.one_of = vec![
                    Box::new(JsonSchemaBevyType {
                        schema_type: Some(SchemaTypeVariant::Single(SchemaType::Null)),
                        ..Default::default()
                    }),
                    Box::new(schema_optional.to_ref_schema()),
                ];
            }
        }
        SchemaDefinition {
            id,
            schema,
            definitions,
            dependencies: dependencies.into_iter().collect(),
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
        let internal_schema_type = (&ty_info).into();
        SchemaTypeInfo {
            ty_info,
            field_data: Some(field_data),
            internal_schema_type,
            reflect_type_data: None,
        }
    }
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

pub(crate) trait TypeDefinitionBuilder {
    /// Builds a JSON schema for a given type ID.
    fn build_schema_for_type_id(
        &self,
        type_id: TypeId,
        metadata: &SchemaTypesMetadata,
    ) -> Option<(Option<TypeReferenceId>, JsonSchemaBevyType)>;
    /// Returns a set of type IDs that are dependencies of the given type ID.
    fn get_type_dependencies(&self, type_id: TypeId) -> HashSet<TypeId>;
    /// Builds a JSON schema for a given type ID with definitions.
    fn build_schema_for_type_id_with_definitions(
        &self,
        type_id: TypeId,
        metadata: &SchemaTypesMetadata,
    ) -> Option<JsonSchemaBevyType>;
}

impl TypeDefinitionBuilder for TypeRegistry {
    fn build_schema_for_type_id(
        &self,
        type_id: TypeId,
        metadata: &SchemaTypesMetadata,
    ) -> Option<(Option<TypeReferenceId>, JsonSchemaBevyType)> {
        let type_reg = self.get(type_id)?;
        let type_info: TypeInformation = type_reg.into();
        let schema_info = type_info.to_schema_type_info_with_metadata(metadata);
        let mut id: Option<TypeReferenceId> = schema_info.ty_info.try_get_type_reference_id();
        let mut definitions: HashMap<TypeReferenceId, SchemaTypeInfo> = HashMap::new();
        if let Some(custom_schema) = &schema_info.ty_info.try_get_custom_schema() {
            return Some((id, custom_schema.0.clone()));
        }
        let range = schema_info.get_range();

        let (type_path, short_path, crate_name, module_path) =
            if let Some(type_path_table) = schema_info.ty_info.try_get_type_path_table() {
                (
                    type_path_table.path().into(),
                    type_path_table.short_path().into(),
                    type_path_table.crate_name().map(Into::into),
                    type_path_table.module_path().map(Into::into),
                )
            } else {
                (Cow::default(), Cow::default(), None, None)
            };

        let not = if is_non_zero_number_type(schema_info.ty_info.type_id()) {
            Some(Box::new(JsonSchemaBevyType {
                const_value: Some(0.into()),
                ..default()
            }))
        } else {
            None
        };

        let mut schema = JsonSchemaBevyType {
            description: schema_info.ty_info.get_docs(),
            not,
            type_path,
            short_path,
            crate_name,
            module_path,
            kind: Some((&schema_info.ty_info).into()),
            minimum: range.min.get_inclusive(),
            maximum: range.max.get_inclusive(),
            exclusive_minimum: range.min.get_exclusive(),
            exclusive_maximum: range.max.get_exclusive(),
            schema_type: (&schema_info).into(),
            reflect_type_data: schema_info.reflect_type_data.clone().unwrap_or_default(),
            ..default()
        };
        match schema_info.internal_schema_type.clone() {
            InternalSchemaType::Map { key, value } => {
                let key = key.to_schema_type_info();
                let value = value.to_schema_type_info();
                if key.ty_info.try_get_primitive_type().is_some() {
                    if let Some(p) = key.ty_info.try_get_regex_for_type() {
                        schema.pattern_properties = [(p, value.to_ref_schema().into())].into();
                        schema.additional_properties = Some(JsonSchemaVariant::BoolValue(false));
                    }
                }
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
                            internal_schema_type: InternalSchemaType::FieldsHolder(
                                FieldsInformation {
                                    fields,
                                    fields_type: FieldType::Named,
                                },
                            ),
                            reflect_type_data: None,
                        };
                        let definition = schema_field.to_definition();

                        schema.properties =
                            [(variant_info.name().into(), definition.schema.into())].into();
                        schema.required = vec![variant_info.name().into()];
                    }
                    VariantInfo::Tuple(tuple_variant_info) => {
                        let stored_fields = get_fields_information(tuple_variant_info.iter());
                        let schema_field = SchemaTypeInfo {
                            ty_info,
                            field_data: None,
                            internal_schema_type: InternalSchemaType::FieldsHolder(
                                FieldsInformation {
                                    fields: stored_fields,

                                    fields_type: FieldType::Unnamed,
                                },
                            ),
                            reflect_type_data: None,
                        };
                        let definition = schema_field.to_definition();

                        schema.properties =
                            [(variant_info.name().into(), definition.schema.into())].into();
                        schema.required = vec![variant_info.name().into()];
                    }
                    VariantInfo::Unit(unit_variant_info) => {
                        let internal_schema_type = (&ty_info).into();
                        let schema_field = SchemaTypeInfo {
                            ty_info,
                            field_data,
                            internal_schema_type,
                            reflect_type_data: None,
                        };
                        return Some((
                            None,
                            JsonSchemaBevyType {
                                const_value: Some(unit_variant_info.name().to_string().into()),
                                schema_type: Some(SchemaTypeVariant::Single(SchemaType::String)),
                                description: schema_field.get_docs(),
                                kind: Some(SchemaKind::Value),
                                ..Default::default()
                            },
                        ));
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
                        definitions: _,
                        dependencies: _,
                    } = field_schema.to_definition();
                    if let Some(id) = id {
                        definitions.insert(id.clone(), field_schema);
                        schema.ref_type = Some(TypeReferencePath::definition(id));
                    } else {
                        let (type_path, short_path, crate_name, module_path) =
                            if let Some(type_path_table) =
                                schema_info.ty_info.try_get_type_path_table()
                            {
                                (
                                    type_path_table.path().into(),
                                    type_path_table.short_path().into(),
                                    type_path_table.crate_name().map(Into::into),
                                    type_path_table.module_path().map(Into::into),
                                )
                            } else {
                                (Cow::default(), Cow::default(), None, None)
                            };
                        schema = JsonSchemaBevyType {
                            short_path,
                            type_path,
                            module_path,
                            crate_name,
                            kind: Some(SchemaKind::TupleStruct),
                            schema_type: (&schema_info).into(),
                            description: schema_info.get_docs(),
                            ..new_schema_type
                        };
                    }
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
                let items_schema = element_ty.to_schema_type_info();
                schema.items = Some(items_schema.to_ref_schema().into());
                schema.min_items = min_size;
                schema.max_items = max_size;
            }
            InternalSchemaType::Optional {
                generic,
                ref schema_type_info,
            } => {
                let schema_optional = SchemaTypeInfo {
                    ty_info: TypeInformation::Type(Box::new(*generic.ty())),
                    ..(**schema_type_info).clone()
                };
                schema.ref_type = None;
                schema.schema_type = None;
                schema.one_of = vec![
                    Box::new(JsonSchemaBevyType {
                        schema_type: Some(SchemaTypeVariant::Single(SchemaType::Null)),
                        ..Default::default()
                    }),
                    Box::new(schema_optional.to_ref_schema()),
                ];
            }
        }
        Some((id, schema))
    }

    fn get_type_dependencies(&self, type_id: TypeId) -> HashSet<TypeId> {
        let Some(type_reg) = self.get(type_id) else {
            return HashSet::new();
        };
        let internal_schema_type: InternalSchemaType = (&TypeInformation::from(type_reg)).into();

        internal_schema_type.get_dependencies(self)
    }

    fn build_schema_for_type_id_with_definitions(
        &self,
        type_id: TypeId,
        metadata: &SchemaTypesMetadata,
    ) -> Option<JsonSchemaBevyType> {
        let Some((_, mut schema)) = self.build_schema_for_type_id(type_id, metadata) else {
            return None;
        };
        let dependencies = self.get_type_dependencies(type_id);
        eprintln!("{} -> {:#?}", schema.type_path, dependencies.len());
        schema.definitions = dependencies
            .into_iter()
            .flat_map(|id| {
                let result = self.build_schema_for_type_id(id, metadata);
                let Some((Some(schema_id), schema)) = result else {
                    return None;
                };
                Some((schema_id, Box::new(schema)))
            })
            .collect();
        Some(schema)
    }
}

#[cfg(test)]
pub(super) mod tests {
    use bevy_ecs::{component::Component, name::Name, reflect::AppTypeRegistry};
    use bevy_platform::collections::HashMap;
    use bevy_reflect::GetTypeRegistration;

    use crate::schemas::json_schema::TypeRegistrySchemaReader;

    use super::*;

    /// Validate a JSON schema against a set of valid and invalid instances.
    pub fn validate<T: GetTypeRegistration + Serialize + Default>(
        schema: JsonSchemaBevyType,
        valid_instances: &[T],
        valid_values: &[serde_json::Value],
        invalid_values: &[serde_json::Value],
    ) {
        let schema_value = serde_json::to_value(&schema).unwrap();
        let schema_validator = jsonschema::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .build(&schema_value)
            .expect("Failed to build schema validator");
        let default_value = serde_json::to_value(T::default()).unwrap();
        assert!(
            schema_validator.validate(&default_value).is_ok(),
            "Default value is invalid: {}, schema: {}",
            default_value,
            serde_json::to_string_pretty(&schema_value).unwrap_or_default()
        );
        let mut errors = Vec::new();
        let valid_instances: Vec<serde_json::Value> = valid_instances
            .iter()
            .flat_map(|s| serde_json::to_value(s).ok())
            .collect();
        for value in valid_instances.iter() {
            if let Err(error) = schema_validator.validate(value) {
                errors.push((error, value.clone()));
            }
        }
        for value in valid_values {
            if let Err(error) = schema_validator.validate(&value) {
                errors.push((error, value.clone()));
            }
        }
        assert!(
            errors.is_empty(),
            "Failed to validate valid instances, errors: {:?}, schema: {}",
            errors,
            serde_json::to_string_pretty(&schema_value).unwrap_or_default()
        );
        for value in invalid_values {
            assert!(
                schema_validator.validate(&value).is_err(),
                "Validation should fail for invalid value: {}, schema: {}",
                value,
                serde_json::to_string_pretty(&schema_value).unwrap_or_default()
            );
        }
    }

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
        assert!(!range.in_range(-1));
        assert!(range.in_range(0));
        assert!(range.in_range(12));
        assert!(!range.in_range(13));
        assert_eq!(range.min, Some(BoundValue::Inclusive(0.into())));
        assert_eq!(range.max, Some(BoundValue::Exclusive(13.into())));
        assert_eq!(
            schema_type,
            Some(SchemaTypeVariant::Single(SchemaType::Integer))
        );
        assert_eq!(type_info.get_docs(), Some("Test documentation".into()));
    }

    #[cfg(feature = "bevy_math")]
    #[test]
    fn other_ss_test() {
        #[derive(Reflect, Default, Deserialize, Serialize)]
        struct Foo {
            /// Test doc
            a: u16,
        }
        validate::<Foo>(
            TypeInformation::TypeRegistration(Foo::get_type_registration())
                .to_schema_type_info()
                .to_definition()
                .into(),
            &[Foo { a: 5 }, Foo { a: 1111 }],
            &[serde_json::json!({"a": 5}), serde_json::json!({"a": 1})],
            &[
                serde_json::json!({"a": 1111111}),
                serde_json::json!({"ab": -5555}),
                serde_json::json!({"a": 5555,"b": 5555}),
            ],
        );
        let atr = AppTypeRegistry::default();
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
        let schema: JsonSchemaBevyType = declaration.to_definition().into();

        validate::<bevy_math::Vec3>(
            schema,
            &[
                bevy_math::Vec3::new(5.0, 4.0, 4.0),
                bevy_math::Vec3::new(25.0, 4.0, 4.0),
            ],
            &[
                serde_json::json!([0, 4, 5]),
                serde_json::json!([5.1, 5.2, 5.3]),
            ],
            &[
                serde_json::json!([5.1, 5.2]),
                serde_json::json!([5.1, 5.2, 4, 4]),
                serde_json::json!({"x": 5.1, "y": 5.2, "z": 5.3}),
            ],
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
        validate::<TupleTest>(
            TypeInformation::TypeRegistration(TupleTest::get_type_registration())
                .to_schema_type_info()
                .to_definition()
                .into(),
            &[TupleTest(10), TupleTest(11), TupleTest(0)],
            &[serde_json::json!(5), serde_json::json!(1)],
            &[serde_json::json!(55), serde_json::json!(-5555)],
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
        let type_info =
            TypeInformation::from(&EnumTest::get_type_registration()).to_schema_type_info();
        let schema: JsonSchemaBevyType = type_info.to_definition().into();
        validate::<EnumTest>(
            schema,
            &[
                EnumTest::Variant1,
                EnumTest::Variant2 {
                    field1: "test".into(),
                    field2: 42,
                },
                EnumTest::Variant3(1, 2),
                EnumTest::Variant4(3),
            ],
            &[],
            &[],
        );
    }

    #[test]
    fn name_field_test() {
        #[derive(Reflect, Default, Deserialize, Serialize)]
        pub struct EnumTest {
            pub name: Name,
        }
        let type_info =
            TypeInformation::from(&EnumTest::get_type_registration()).to_schema_type_info();
        let schema: JsonSchemaBevyType = type_info.to_definition().into();
        eprintln!(
            "{}",
            serde_json::to_string_pretty(&schema).unwrap_or_default()
        );
    }

    #[test]
    fn optional_tests() {
        #[derive(Reflect, Default, Deserialize, Serialize)]
        pub struct ArrayComponent {
            pub array: [u8; 3],
        }
        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<ArrayComponent>();
            register.register::<Option<ArrayComponent>>();
        }
        let type_registry = atr.read();
        let schema = type_registry
            .export_type_json_schema::<Option<ArrayComponent>>(&Default::default())
            .expect("Failed to export type JSON schema");
        validate::<Option<ArrayComponent>>(
            schema,
            &[None, Some(ArrayComponent { array: [5, 1, 9] })],
            &[
                serde_json::json!({"array": [1, 2, 3]}),
                serde_json::Value::Null,
            ],
            &[serde_json::json!({"array": [1999, 2, 3]})],
        );
    }

    #[test]
    fn reflect_struct_with_array() {
        #[derive(Reflect, Default, Deserialize, Serialize)]
        pub struct ArrayComponent {
            pub array: [u8; 3],
        }
        let type_info =
            TypeInformation::from(&ArrayComponent::get_type_registration()).to_schema_type_info();
        let schema: JsonSchemaBevyType = type_info.to_definition().into();
        validate::<ArrayComponent>(
            schema,
            &[
                ArrayComponent::default(),
                ArrayComponent { array: [1, 2, 3] },
                ArrayComponent { array: [4, 5, 6] },
                ArrayComponent { array: [7, 8, 9] },
            ],
            &[],
            &[
                serde_json::json!({"array": [0,5]}),
                serde_json::json!({"array": [0,5,-1]}),
                serde_json::json!({"aa": [0,5,5]}),
                serde_json::json!({"array": [0,5,5,5]}),
                serde_json::json!({"array": [0.1,5.1,5]}),
            ],
        );
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

        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<ArrayComponent>();
            register.register::<ArrayComponentWithMoreVariants>();
        }
        let types = atr.read();
        let schema = types
            .build_schema_for_type_id_with_definitions(
                TypeId::of::<ArrayComponent>(),
                &Default::default(),
            )
            .expect("");
        let schema_second = types
            .build_schema_for_type_id_with_definitions(
                TypeId::of::<ArrayComponentWithMoreVariants>(),
                &Default::default(),
            )
            .expect("");
        assert_eq!(schema.definitions.len(), schema_second.definitions.len());
        validate::<ArrayComponentWithMoreVariants>(schema_second, &[], &[], &[]);
    }

    #[test]
    fn reflect_struct_with_hashmap() {
        #[derive(Reflect, Default, Deserialize, Serialize)]
        pub struct HashMapStruct {
            pub map: HashMap<i32, Option<i32>>,
        }
        let type_info =
            TypeInformation::from(&HashMapStruct::get_type_registration()).to_schema_type_info();
        let schema: JsonSchemaBevyType = type_info.to_definition().into();
        validate::<HashMapStruct>(
            schema,
            &[HashMapStruct {
                map: [(5, Some(10)), (15, Some(20)), (-25, Some(30))].into(),
            }],
            &[
                serde_json::json!({"map": {"-5": 10}}),
                serde_json::json!({"map": {"5": None::<i32>}}),
            ],
            &[
                serde_json::json!({"map": {"5.5": 10}}),
                serde_json::json!({"map": {"s": 10}}),
            ],
        );
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

        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<NestedStruct>();
        }
        let types = atr.read();
        let schema = types
            .build_schema_for_type_id_with_definitions(
                TypeId::of::<NestedStruct>(),
                &Default::default(),
            )
            .expect("");

        assert_eq!(
            schema.definitions.len(),
            3,
            "Expected 3 definitions, but got {}, schema: {}",
            schema.definitions.len(),
            serde_json::to_string_pretty(&schema).unwrap_or_default()
        );
        validate::<NestedStruct>(
            schema,
            &[NestedStruct {
                other: OtherStruct { field: "s".into() },
                ..Default::default()
            }],
            &[],
            &[],
        );
    }

    #[test]
    fn reflect_tuple_struct_with_one_field_that_is_struct() {
        use bevy_ecs::prelude::ReflectComponent;
        use bevy_reflect::prelude::ReflectDefault;

        #[derive(Reflect, Default, Deserialize, Serialize)]
        pub struct ThirdStruct {
            pub map_strings: HashMap<String, i32>,
        }
        #[derive(Reflect, Default, Deserialize, Serialize, Component)]
        #[reflect(Component, Default)]
        /// A tuple struct with one field.
        pub struct TupleStruct(pub HashMap<String, i32>);

        let type_info =
            TypeInformation::from(&TupleStruct::get_type_registration()).to_schema_type_info();
        let schema: JsonSchemaBevyType = type_info.to_definition().into();
        validate::<TupleStruct>(
            schema,
            &[TupleStruct(
                [
                    ("s".to_string(), 0),
                    ("b".to_string(), 5),
                    ("c".to_string(), 10),
                ]
                .into(),
            )],
            &[serde_json::json!({"json": 5})],
            &[serde_json::json!("json")],
        );
    }

    #[test]
    fn reflect_non_zero_type() {
        #[derive(Reflect, Deserialize, Serialize, Component)]
        /// A tuple struct with one field.
        pub struct TupleStruct(pub NonZeroI8);
        impl Default for TupleStruct {
            fn default() -> Self {
                TupleStruct(NonZeroI8::new(15i8).expect("Should not fail"))
            }
        }

        let type_info =
            TypeInformation::from(&TupleStruct::get_type_registration()).to_schema_type_info();
        let schema: JsonSchemaBevyType = type_info.to_definition().into();

        validate::<TupleStruct>(
            schema,
            &[TupleStruct(NonZeroI8::new(115i8).expect("Should not fail"))],
            &[
                serde_json::json!(15),
                serde_json::json!(50),
                serde_json::json!(-49),
            ],
            &[serde_json::json!(0), serde_json::Value::Null],
        );
    }

    #[test]
    fn reflect_tuple_struct_with_one_field() {
        use bevy_ecs::prelude::ReflectComponent;
        use bevy_reflect::prelude::ReflectDefault;
        #[derive(Reflect, Deserialize, Serialize, Component)]
        #[reflect(Component, Default)]
        /// A tuple struct with one field.
        pub struct TupleStruct(#[reflect(@10..=50i8)] pub i8);
        impl Default for TupleStruct {
            fn default() -> Self {
                TupleStruct(15)
            }
        }
        let type_info =
            TypeInformation::from(&TupleStruct::get_type_registration()).to_schema_type_info();
        let s: JsonSchemaBevyType = type_info.to_definition().into();
        let range: MinMaxValues = (&s).into();
        assert!(!range.in_range(51));
        assert!(range.in_range(15));
        assert!(range.in_range(50));
        assert!(!range.in_range(51));

        validate::<TupleStruct>(
            s,
            &[TupleStruct(15)],
            &[
                serde_json::json!(15),
                serde_json::json!(50),
                serde_json::json!(49),
                serde_json::json!(10),
            ],
            &[
                serde_json::json!(9),
                serde_json::json!(51),
                serde_json::json!(-1),
                serde_json::json!(5.3),
                serde_json::Value::Null,
            ],
        );
    }
}
