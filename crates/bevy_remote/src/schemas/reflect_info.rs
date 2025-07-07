//! Module containing information about reflected types.
use crate::schemas::json_schema::{
    JsonSchemaBevyType, JsonSchemaVariant, SchemaKind, SchemaType, SchemaTypeVariant,
};
use crate::schemas::{ReflectJsonSchemaForceAsArray, SchemaTypesMetadata};
use alloc::borrow::Cow;
use alloc::sync::Arc;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::name::Name;
use bevy_platform::collections::{HashMap, HashSet};
use bevy_platform::sync::LazyLock;
use bevy_reflect::attributes::CustomAttributes;
use bevy_reflect::{
    EnumInfo, GenericInfo, NamedField, Reflect, Type, TypeInfo, TypePathTable, TypeRegistration,
    TypeRegistry, UnnamedField, VariantInfo,
};
use bevy_utils::{default, TypeIdMap};
use core::any::TypeId;
use core::fmt;
use core::fmt::{Display, Formatter};
use core::num::{
    NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU128,
    NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
};
use core::slice::Iter;
use core::{
    any::Any,
    fmt::Debug,
    ops::{Bound, RangeBounds},
};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Default)]
pub(super) struct PrimitiveTypeInfo {
    pub schema_type: SchemaType,
    pub schema_kind: SchemaKind,
    pub min: Option<BoundValue>,
    pub max: Option<BoundValue>,
    pub not: Option<SchemaNumber>,
}

const PRIMITIVE_VALUE: PrimitiveTypeInfo = PrimitiveTypeInfo {
    schema_kind: SchemaKind::Value,
    schema_type: SchemaType::Object,
    min: None,
    max: None,
    not: None,
};

const PRIMITIVE_FLOAT: PrimitiveTypeInfo = PrimitiveTypeInfo {
    schema_kind: SchemaKind::Value,
    schema_type: SchemaType::Number,
    ..PRIMITIVE_VALUE
};
const PRIMITIVE_INTEGER: PrimitiveTypeInfo = PrimitiveTypeInfo {
    schema_kind: SchemaKind::Value,
    schema_type: SchemaType::Integer,
    ..PRIMITIVE_VALUE
};
const PRIMITIVE_UNSIGNED_INTEGER: PrimitiveTypeInfo = PrimitiveTypeInfo {
    min: Some(BoundValue::Inclusive(SchemaNumber::Int(0))),
    ..PRIMITIVE_INTEGER
};
const PRIMITIVE_STRING: PrimitiveTypeInfo = PrimitiveTypeInfo {
    schema_type: SchemaType::String,
    ..PRIMITIVE_VALUE
};

pub(super) static BASE_TYPES_INFO: LazyLock<HashMap<TypeId, PrimitiveTypeInfo>> =
    LazyLock::new(|| {
        [
            (
                TypeId::of::<bool>(),
                PrimitiveTypeInfo {
                    schema_type: SchemaType::Boolean,
                    ..PRIMITIVE_VALUE
                },
            ),
            (TypeId::of::<f32>(), PRIMITIVE_FLOAT),
            (TypeId::of::<f64>(), PRIMITIVE_FLOAT),
            (
                TypeId::of::<i8>(),
                PrimitiveTypeInfo {
                    min: Some(BoundValue::Inclusive(SchemaNumber::Int(i8::MIN as i128))),
                    max: Some(BoundValue::Inclusive(SchemaNumber::Int(i8::MAX as i128))),
                    ..PRIMITIVE_INTEGER
                },
            ),
            (
                TypeId::of::<i16>(),
                PrimitiveTypeInfo {
                    min: Some(BoundValue::Inclusive(SchemaNumber::Int(i16::MIN as i128))),
                    max: Some(BoundValue::Inclusive(SchemaNumber::Int(i16::MAX as i128))),
                    ..PRIMITIVE_INTEGER
                },
            ),
            (
                TypeId::of::<i32>(),
                PrimitiveTypeInfo {
                    min: Some(BoundValue::Inclusive(SchemaNumber::Int(i32::MIN as i128))),
                    max: Some(BoundValue::Inclusive(SchemaNumber::Int(i32::MAX as i128))),
                    ..PRIMITIVE_INTEGER
                },
            ),
            (TypeId::of::<i64>(), PRIMITIVE_INTEGER),
            (TypeId::of::<i128>(), PRIMITIVE_INTEGER),
            (TypeId::of::<i128>(), PRIMITIVE_INTEGER),
            (TypeId::of::<isize>(), PRIMITIVE_INTEGER),
            (
                TypeId::of::<u8>(),
                PrimitiveTypeInfo {
                    max: Some(BoundValue::Inclusive(SchemaNumber::Int(u8::MAX as i128))),
                    ..PRIMITIVE_UNSIGNED_INTEGER
                },
            ),
            (
                TypeId::of::<u16>(),
                PrimitiveTypeInfo {
                    max: Some(BoundValue::Inclusive(SchemaNumber::Int(u16::MAX as i128))),
                    ..PRIMITIVE_UNSIGNED_INTEGER
                },
            ),
            (
                TypeId::of::<u32>(),
                PrimitiveTypeInfo {
                    max: Some(BoundValue::Inclusive(SchemaNumber::Int(u32::MAX as i128))),
                    ..PRIMITIVE_UNSIGNED_INTEGER
                },
            ),
            (TypeId::of::<u64>(), PRIMITIVE_UNSIGNED_INTEGER),
            (TypeId::of::<u128>(), PRIMITIVE_UNSIGNED_INTEGER),
            (TypeId::of::<usize>(), PRIMITIVE_UNSIGNED_INTEGER),
            (
                TypeId::of::<bevy_ecs::entity::Entity>(),
                PrimitiveTypeInfo {
                    schema_kind: SchemaKind::Struct,
                    min: Some(BoundValue::Exclusive(SchemaNumber::Int(0))),
                    max: Some(BoundValue::Exclusive(SchemaNumber::Int(u32::MAX as i128))),
                    ..PRIMITIVE_UNSIGNED_INTEGER
                },
            ),
            (
                TypeId::of::<NonZeroI8>(),
                PrimitiveTypeInfo {
                    min: Some(BoundValue::Inclusive(SchemaNumber::Int(i8::MIN as i128))),
                    max: Some(BoundValue::Inclusive(SchemaNumber::Int(i8::MAX as i128))),
                    not: Some(SchemaNumber::Int(0)),
                    ..PRIMITIVE_INTEGER
                },
            ),
            (
                TypeId::of::<NonZeroI16>(),
                PrimitiveTypeInfo {
                    min: Some(BoundValue::Inclusive(SchemaNumber::Int(i16::MIN as i128))),
                    max: Some(BoundValue::Inclusive(SchemaNumber::Int(i16::MAX as i128))),
                    not: Some(SchemaNumber::Int(0)),
                    ..PRIMITIVE_INTEGER
                },
            ),
            (
                TypeId::of::<NonZeroI32>(),
                PrimitiveTypeInfo {
                    min: Some(BoundValue::Inclusive(SchemaNumber::Int(i32::MIN as i128))),
                    max: Some(BoundValue::Inclusive(SchemaNumber::Int(i32::MAX as i128))),
                    not: Some(SchemaNumber::Int(0)),
                    ..PRIMITIVE_INTEGER
                },
            ),
            (
                TypeId::of::<NonZeroI64>(),
                PrimitiveTypeInfo {
                    not: Some(SchemaNumber::Int(0)),
                    ..PRIMITIVE_INTEGER
                },
            ),
            (
                TypeId::of::<NonZeroI128>(),
                PrimitiveTypeInfo {
                    not: Some(SchemaNumber::Int(0)),
                    ..PRIMITIVE_INTEGER
                },
            ),
            (
                TypeId::of::<NonZeroIsize>(),
                PrimitiveTypeInfo {
                    not: Some(SchemaNumber::Int(0)),
                    ..PRIMITIVE_INTEGER
                },
            ),
            (
                TypeId::of::<NonZeroUsize>(),
                PrimitiveTypeInfo {
                    min: Some(BoundValue::Inclusive(SchemaNumber::Int(1))),
                    ..PRIMITIVE_INTEGER
                },
            ),
            (
                TypeId::of::<NonZeroU8>(),
                PrimitiveTypeInfo {
                    min: Some(BoundValue::Inclusive(SchemaNumber::Int(1))),
                    max: Some(BoundValue::Inclusive(SchemaNumber::Int(u8::MAX as i128))),
                    ..PRIMITIVE_UNSIGNED_INTEGER
                },
            ),
            (
                TypeId::of::<NonZeroU16>(),
                PrimitiveTypeInfo {
                    min: Some(BoundValue::Inclusive(SchemaNumber::Int(1))),
                    max: Some(BoundValue::Inclusive(SchemaNumber::Int(u16::MAX as i128))),
                    ..PRIMITIVE_UNSIGNED_INTEGER
                },
            ),
            (
                TypeId::of::<NonZeroU32>(),
                PrimitiveTypeInfo {
                    min: Some(BoundValue::Inclusive(SchemaNumber::Int(1))),
                    max: Some(BoundValue::Inclusive(SchemaNumber::Int(u32::MAX as i128))),
                    ..PRIMITIVE_UNSIGNED_INTEGER
                },
            ),
            (
                TypeId::of::<NonZeroU64>(),
                PrimitiveTypeInfo {
                    min: Some(BoundValue::Inclusive(SchemaNumber::Int(1))),
                    ..PRIMITIVE_UNSIGNED_INTEGER
                },
            ),
            (
                TypeId::of::<NonZeroU128>(),
                PrimitiveTypeInfo {
                    min: Some(BoundValue::Inclusive(SchemaNumber::Int(1))),
                    ..PRIMITIVE_INTEGER
                },
            ),
            (TypeId::of::<String>(), PRIMITIVE_STRING),
            (TypeId::of::<Cow<str>>(), PRIMITIVE_STRING),
            (TypeId::of::<char>(), PRIMITIVE_STRING),
            (TypeId::of::<str>(), PRIMITIVE_STRING),
            (
                TypeId::of::<Name>(),
                PrimitiveTypeInfo {
                    schema_kind: SchemaKind::Struct,
                    ..PRIMITIVE_STRING
                },
            ),
        ]
        .into()
    });

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

impl From<&str> for TypeReferenceId {
    fn from(t: &str) -> Self {
        TypeReferenceId(
            t.replace("::", "-")
                .replace(", ", "+")
                .replace(")", "")
                .replace("(", "")
                .replace(">", "-")
                .replace("<", "-")
                .into(),
        )
    }
}

impl From<&Type> for TypeReferenceId {
    fn from(t: &Type) -> Self {
        t.path().into()
    }
}

impl From<&TypePathTable> for TypeReferenceId {
    fn from(t: &TypePathTable) -> Self {
        t.path().into()
    }
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
    fields: Vec<SchemaFieldData>,
    /// Field type information.
    fields_type: FieldType,
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

pub(super) trait OptionalInfoReader {
    fn try_get_optional(&self) -> Option<&GenericInfo>;
}

impl OptionalInfoReader for EnumInfo {
    fn try_get_optional(&self) -> Option<&GenericInfo> {
        let generic = self.generics().first()?;
        if self.variant_len() != 2
            || !self.contains_variant("Some")
            || !self.contains_variant("None")
        {
            return None;
        }
        Some(generic)
    }
}
impl OptionalInfoReader for TypeInfo {
    fn try_get_optional(&self) -> Option<&GenericInfo> {
        let TypeInfo::Enum(enum_info) = self else {
            return None;
        };
        enum_info.try_get_optional()
    }
}
impl OptionalInfoReader for TypeRegistration {
    fn try_get_optional(&self) -> Option<&GenericInfo> {
        self.type_info().try_get_optional()
    }
}

fn try_get_regex_for_type(id: TypeId) -> Option<Cow<'static, str>> {
    let data = BASE_TYPES_INFO.get(&id)?;
    match data.schema_type {
        SchemaType::Number => Some("\\d+(?:\\.'\\d+)?".into()),
        SchemaType::Integer => Some("^(0|-*[1-9]+[0-9]*)$".into()),
        _ => None,
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
    /// Type of the field.
    pub type_id: TypeId,
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
    /// Returns the description of the field.
    pub fn to_description(&self) -> Option<Cow<'static, str>> {
        self.description
            .as_ref()
            .map(|description| Cow::Owned(description.trim().replace("\n", "")))
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
impl From<SchemaNumber> for serde_json::Value {
    fn from(val: SchemaNumber) -> Self {
        match val {
            SchemaNumber::Int(value) => {
                let int: i64 = value.try_into().ok().unwrap_or_default();
                serde_json::Value::Number(int.into())
            }
            SchemaNumber::Float(value) => {
                serde_json::Value::Number(serde_json::Number::from_f64(value).unwrap())
            }
        }
    }
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
            (Some(ex), None) => Some(BoundValue::Exclusive(*ex)),
            (_, Some(inclusive)) => Some(BoundValue::Inclusive(*inclusive)),
            _ => None,
        };
        let max = match (&value.exclusive_maximum, &value.maximum) {
            (Some(ex), None) => Some(BoundValue::Exclusive(*ex)),
            (_, Some(inclusive)) => Some(BoundValue::Inclusive(*inclusive)),
            _ => None,
        };
        MinMaxValues { min, max }
    }
}

impl MinMaxValues {
    /// Combines two [`MinMaxValues`] instances
    pub fn with(self, other: MinMaxValues) -> MinMaxValues {
        let min = match (self.min, other.min) {
            (_, Some(other_min)) => Some(other_min),
            (min, _) => min,
        };
        let max = match (self.max, other.max) {
            (_, Some(other_max)) => Some(other_max),
            (max, _) => max,
        };
        MinMaxValues { min, max }
    }

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
        if let Some(info) = BASE_TYPES_INFO.get(&value) {
            MinMaxValues {
                min: info.min,
                max: info.max,
            }
        } else {
            MinMaxValues::default()
        }
    }
}

/// Enum representing the internal schema type information for different Rust types.
/// This enum categorizes how different types should be represented in JSON schema.
#[derive(Clone, Debug)]
pub(crate) enum InternalSchemaType {
    /// Represents array-like types (Vec, arrays, lists, sets).
    Array {
        /// Element type information for the array.
        element_ty: TypeId,
        /// Minimum number of elements allowed in the array.
        min_size: Option<u64>,
        /// Maximum number of elements allowed in the array.
        max_size: Option<u64>,
    },
    /// Holds all variants of an enum type.
    EnumHolder(Vec<VariantInfo>),
    /// Holds named fields for struct, tuple, and tuple struct types.
    FieldsHolder(FieldsInformation),
    /// Represents an Optional type (e.g., `Option<T>`).
    Optional {
        /// Generic information about the wrapped type `T` in `Option<T>`.
        generic: GenericInfo,
    },
    /// Represents a Map type (e.g., `HashMap`<K, V>).
    Map {
        /// Information about the key type contained in the map.
        key: TypeId,
        /// Information about the value type contained in the map.
        value: TypeId,
    },
    /// Represents a Primitive type (e.g., `i32`, `f64`, `bool`, etc.).
    PrimitiveType {
        /// The unique identifier for the primitive type.
        type_id: TypeId,
        /// The schema type of the primitive.
        primitive: SchemaType,
        /// Optional field data for the primitive type.
        field_data: Option<SchemaFieldData>,
    },
    /// Variant for regular primitive types and other simple types.
    Regular(TypeId),
}

impl Default for InternalSchemaType {
    fn default() -> Self {
        InternalSchemaType::Regular(TypeId::of::<()>())
    }
}

impl InternalSchemaType {
    fn is_optional(&self) -> bool {
        matches!(self, InternalSchemaType::Optional { .. })
    }
    pub(super) fn from_type_registration(
        value: &TypeRegistration,
        registry: &TypeRegistry,
    ) -> InternalSchemaType {
        if let Some(primitive) =
            SchemaType::try_get_primitive_type_from_type_id(value.type_info().type_id())
        {
            return InternalSchemaType::PrimitiveType {
                type_id: value.type_info().type_id(),
                primitive,
                field_data: None,
            };
        }
        match value.type_info() {
            TypeInfo::Struct(struct_info) => {
                let fields = get_fields_information(struct_info.iter());
                let fields_type = if value.data::<ReflectJsonSchemaForceAsArray>().is_some() {
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
                if info.field_len() == 1 {
                    let field = info.field_at(0).expect("HOW?");
                    let id = field.type_id();
                    let reg = registry.get(id).expect("HOW?");
                    let mut internal = InternalSchemaType::from_type_registration(reg, registry);
                    if let InternalSchemaType::PrimitiveType {
                        type_id: _,
                        primitive: _,
                        field_data,
                    } = &mut internal
                    {
                        *field_data = Some(field.into());
                    };
                    internal
                } else {
                    InternalSchemaType::FieldsHolder(FieldsInformation {
                        fields: get_fields_information(info.iter()),
                        fields_type: FieldType::Unnamed,
                    })
                }
            }
            TypeInfo::Tuple(info) => InternalSchemaType::FieldsHolder(FieldsInformation {
                fields: get_fields_information(info.iter()),
                fields_type: FieldType::Unnamed,
            }),
            TypeInfo::Enum(enum_info) => match enum_info.try_get_optional() {
                Some(e) => InternalSchemaType::Optional { generic: e.clone() },
                None => InternalSchemaType::EnumHolder(enum_info.iter().cloned().collect()),
            },

            TypeInfo::List(list_info) => InternalSchemaType::Array {
                element_ty: list_info.item_ty().id(),
                min_size: None,
                max_size: None,
            },
            TypeInfo::Set(set_info) => InternalSchemaType::Array {
                element_ty: set_info.value_ty().id(),
                min_size: None,
                max_size: None,
            },
            TypeInfo::Array(array_info) => InternalSchemaType::Array {
                element_ty: array_info.item_ty().id(),
                min_size: Some(array_info.capacity() as u64),
                max_size: Some(array_info.capacity() as u64),
            },
            TypeInfo::Map(map_info) => InternalSchemaType::Map {
                key: map_info.key_ty().id(),
                value: map_info.value_ty().id(),
            },
            TypeInfo::Opaque(t) => InternalSchemaType::Regular(t.ty().id()),
        }
    }
    /// Returns the dependencies of the type.
    pub(super) fn get_dependencies(&self, registry: &TypeRegistry) -> HashSet<TypeId> {
        let mut dependencies = HashSet::new();
        match &self {
            InternalSchemaType::Array {
                element_ty,
                min_size: _,
                max_size: _,
            } => {
                if let Some(reg) = registry.get(*element_ty) {
                    let subschema = InternalSchemaType::from_type_registration(reg, registry);
                    if !subschema.is_optional() {
                        dependencies.insert(reg.type_id());
                    }
                    dependencies.extend(subschema.get_dependencies(registry));
                }
            }
            InternalSchemaType::EnumHolder(variant_infos) => {
                for variant_info in variant_infos {
                    let subschema = match variant_info {
                        VariantInfo::Struct(struct_variant_info) => {
                            InternalSchemaType::FieldsHolder(FieldsInformation {
                                fields: get_fields_information(struct_variant_info.iter()),
                                fields_type: FieldType::Named,
                            })
                        }
                        VariantInfo::Tuple(tuple_variant_info) => {
                            InternalSchemaType::FieldsHolder(FieldsInformation {
                                fields: get_fields_information(tuple_variant_info.iter()),
                                fields_type: FieldType::Unnamed,
                            })
                        }
                        VariantInfo::Unit(_) => continue,
                    };
                    dependencies.extend(subschema.get_dependencies(registry));
                }
            }
            InternalSchemaType::FieldsHolder(fields_information) => {
                for field in fields_information.iter() {
                    if SchemaType::try_get_primitive_type_from_type_id(field.type_id).is_some() {
                        continue;
                    }
                    let Some(reg) = registry.get(field.type_id) else {
                        continue;
                    };
                    let subschema = InternalSchemaType::from_type_registration(reg, registry);
                    if !subschema.is_optional() {
                        dependencies.insert(field.type_id);
                    }
                    dependencies.extend(subschema.get_dependencies(registry));
                }
            }
            InternalSchemaType::Optional { generic } => {
                if let Some(reg) = registry.get(generic.type_id()) {
                    if SchemaType::try_get_primitive_type_from_type_id(reg.type_id()).is_none() {
                        let subschema = InternalSchemaType::from_type_registration(reg, registry);
                        if !subschema.is_optional() {
                            dependencies.insert(reg.type_id());
                        }
                        dependencies.extend(subschema.get_dependencies(registry));
                    }
                }
            }
            InternalSchemaType::Map { key, value } => {
                if let Some(reg) = registry.get(key.type_id()) {
                    if SchemaType::try_get_primitive_type_from_type_id(reg.type_id()).is_none() {
                        let subschema = InternalSchemaType::from_type_registration(reg, registry);
                        if !subschema.is_optional() {
                            dependencies.insert(reg.type_id());
                        }
                        dependencies.extend(subschema.get_dependencies(registry));
                    }
                }
                if let Some(reg) = registry.get(value.type_id()) {
                    if SchemaType::try_get_primitive_type_from_type_id(reg.type_id()).is_none() {
                        let subschema = InternalSchemaType::from_type_registration(reg, registry);
                        if !subschema.is_optional() {
                            dependencies.insert(reg.type_id());
                        }
                        dependencies.extend(subschema.get_dependencies(registry));
                    }
                }
            }
            InternalSchemaType::Regular(t) => {
                _ = dependencies.insert(*t);
            }
            InternalSchemaType::PrimitiveType { .. } => {}
        }
        dependencies
    }
}

impl From<&InternalSchemaType> for Option<SchemaTypeVariant> {
    fn from(value: &InternalSchemaType) -> Self {
        match value {
            InternalSchemaType::PrimitiveType {
                type_id: _,
                primitive,
                ..
            } => Some(SchemaTypeVariant::Single(*primitive)),
            InternalSchemaType::Array { .. } => Some(SchemaTypeVariant::Single(SchemaType::Array)),
            InternalSchemaType::FieldsHolder(fields) => match fields.fields_type {
                FieldType::Named => Some(SchemaTypeVariant::Single(SchemaType::Object)),
                FieldType::Unnamed if fields.fields.len() == 1 => {
                    let schema: SchemaType = fields.fields[0].type_id.into();
                    schema.into()
                }
                _ => Some(SchemaTypeVariant::Single(SchemaType::Array)),
            },
            InternalSchemaType::Map { .. } => Some(SchemaTypeVariant::Single(SchemaType::Object)),
            InternalSchemaType::Regular(type_id) => {
                Some(SchemaTypeVariant::Single((*type_id).into()))
            }
            InternalSchemaType::EnumHolder(_) | InternalSchemaType::Optional { generic: _ } => None,
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
            type_id: value.type_id(),
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
            type_id: value.type_id(),
        }
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
            type_id: value.type_id(),
        }
    }
}

fn get_fields_information<'a, 'b, T>(iterator: Iter<'a, T>) -> Vec<SchemaFieldData>
where
    SchemaFieldData: From<&'a T>,
{
    iterator
        .enumerate()
        .map(|(index, field)| SchemaFieldData {
            index: Some(index),
            ..field.into()
        })
        .collect()
}

pub(crate) fn variant_to_definition(
    variant: &VariantInfo,
    registry: &TypeRegistry,
) -> JsonSchemaBevyType {
    let field_data: SchemaFieldData = variant.into();
    let mut schema = JsonSchemaBevyType {
        description: field_data.to_description(),
        kind: Some(SchemaKind::Value),
        schema_type: Some(SchemaTypeVariant::Single(SchemaType::Object)),
        additional_properties: Some(JsonSchemaVariant::BoolValue(false)),
        ..Default::default()
    };
    let fields_info = match variant {
        VariantInfo::Unit(unit_variant_info) => {
            schema.const_value = Some(unit_variant_info.name().to_string().into());
            schema.schema_type = Some(SchemaTypeVariant::Single(SchemaType::String));
            schema.additional_properties = None;
            return schema;
        }
        VariantInfo::Struct(struct_variant_info) => FieldsInformation {
            fields: get_fields_information(struct_variant_info.iter()),
            fields_type: FieldType::Named,
        },
        VariantInfo::Tuple(tuple_variant_info) => FieldsInformation {
            fields: get_fields_information(tuple_variant_info.iter()),
            fields_type: FieldType::Unnamed,
        },
    };
    let mut subschema = JsonSchemaBevyType::default();
    registry.update_schema_with_fields_info(&mut subschema, &fields_info);
    schema.properties = [(variant.name().into(), subschema.into())].into();
    schema
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

    /// Builds a schema reference for a given type ID.
    fn build_schema_reference_for_type_id(
        &self,
        type_id: TypeId,
        field_data: Option<&SchemaFieldData>,
    ) -> Option<JsonSchemaBevyType>;

    /// Updates a JSON schema with fields information.
    fn update_schema_with_fields_info(
        &self,
        schema: &mut JsonSchemaBevyType,
        info: &FieldsInformation,
    ) {
        match &info.fields_type {
            FieldType::Named => {
                schema.additional_properties = Some(JsonSchemaVariant::BoolValue(false));
                schema.schema_type = Some(SchemaTypeVariant::Single(SchemaType::Object));
                let schema_fields: Vec<(Cow<'static, str>, JsonSchemaBevyType)> = info
                    .fields
                    .iter()
                    .map(|field| {
                        (
                            field.to_name(),
                            self.build_schema_reference_for_type_id(field.type_id, Some(field))
                                .unwrap_or_default(),
                        )
                    })
                    .collect();
                schema.properties = schema_fields
                    .into_iter()
                    .map(|(name, schema)| (name.clone(), schema.into()))
                    .collect();
                schema.required = info
                    .fields
                    .iter()
                    .map(|field| field.name.clone().unwrap_or_default())
                    .collect();
            }
            FieldType::Unnamed if info.fields.len() == 1 => {
                let new_schema = self
                    .build_schema_reference_for_type_id(
                        info.fields[0].type_id,
                        Some(&info.fields[0]),
                    )
                    .unwrap_or_default();
                *schema = new_schema;
                schema.kind = Some(SchemaKind::Tuple);
            }
            FieldType::Unnamed | FieldType::ForceUnnamed => {
                schema.min_items = Some(info.fields.len() as u64);
                schema.max_items = Some(info.fields.len() as u64);
                schema.prefix_items = info
                    .fields
                    .iter()
                    .map(|field| {
                        let mut schema = self
                            .build_schema_reference_for_type_id(field.type_id, Some(field))
                            .unwrap_or_default();
                        if schema.description.is_none() && field.name.is_some() {
                            schema.description = field.name.clone();
                        }
                        JsonSchemaVariant::Schema(Box::new(schema))
                    })
                    .collect();
            }
        }
    }
}

impl TypeDefinitionBuilder for TypeRegistry {
    fn build_schema_for_type_id(
        &self,
        type_id: TypeId,
        metadata: &SchemaTypesMetadata,
    ) -> Option<(Option<TypeReferenceId>, JsonSchemaBevyType)> {
        let type_reg = self.get(type_id)?;
        let internal = InternalSchemaType::from_type_registration(type_reg, self);

        let mut id: Option<TypeReferenceId> = Some(type_reg.type_info().type_path().into());
        if let Some(custom_schema) = &type_reg.data::<super::ReflectJsonSchema>() {
            return Some((id, custom_schema.0.clone()));
        }
        let range: MinMaxValues = type_id.into();
        let type_path_table = type_reg.type_info().type_path_table();
        let (type_path, short_path, crate_name, module_path) = (
            type_path_table.path().into(),
            type_path_table.short_path().into(),
            type_path_table.crate_name().map(Into::into),
            type_path_table.module_path().map(Into::into),
        );
        #[cfg(feature = "documentation")]
        let description = type_reg
            .type_info()
            .docs()
            .map(|docs| docs.trim().replace("\n", "").into());
        #[cfg(not(feature = "documentation"))]
        let description = None;

        let mut schema = JsonSchemaBevyType {
            description,
            type_path,
            short_path,
            crate_name,
            module_path,
            kind: Some(SchemaKind::from_type_reg(type_reg)),
            minimum: range.min.get_inclusive(),
            maximum: range.max.get_inclusive(),
            exclusive_minimum: range.min.get_exclusive(),
            exclusive_maximum: range.max.get_exclusive(),
            schema_type: (&internal).into(),
            reflect_type_data: metadata.get_registered_reflect_types(type_reg),
            ..default()
        };
        schema.schema_type = (&internal).into();
        match internal {
            InternalSchemaType::PrimitiveType {
                type_id: _,
                primitive: _,
                field_data,
            } => {
                return self
                    .build_schema_reference_for_type_id(type_id, field_data.as_ref())
                    .map(|schema| (None, schema));
            }
            InternalSchemaType::Map { key, value } => {
                id = None;
                schema.value_type = self
                    .build_schema_reference_for_type_id(value, None)
                    .map(Box::new);
                if let Some(p) = try_get_regex_for_type(key) {
                    schema.pattern_properties =
                        [(p, schema.value_type.clone().unwrap_or_default())].into();
                    schema.additional_properties = Some(JsonSchemaVariant::BoolValue(false));
                } else {
                    schema.additional_properties =
                        schema.value_type.clone().map(JsonSchemaVariant::Schema);
                }
                schema.key_type = self
                    .build_schema_reference_for_type_id(key, None)
                    .map(Box::new);
            }
            InternalSchemaType::Regular(_) => {}
            InternalSchemaType::EnumHolder(variants) => {
                schema.one_of = variants
                    .iter()
                    .map(|variant| {
                        let schema = variant_to_definition(variant, self);
                        Box::new(schema)
                    })
                    .collect();
            }
            InternalSchemaType::FieldsHolder(fields) => {
                self.update_schema_with_fields_info(&mut schema, &fields);
            }
            InternalSchemaType::Array {
                element_ty,
                min_size,
                max_size,
            } => {
                id = None;
                let items_schema = self
                    .build_schema_reference_for_type_id(element_ty, None)
                    .unwrap_or_default();
                schema.items = Some(items_schema.into());
                schema.min_items = min_size;
                schema.max_items = max_size;
            }
            InternalSchemaType::Optional { generic } => {
                id = None;
                let optional_schema = self
                    .build_schema_reference_for_type_id(generic.type_id(), None)
                    .unwrap_or_default();

                schema.ref_type = None;
                schema.schema_type = None;
                schema.one_of = vec![
                    Box::new(JsonSchemaBevyType {
                        schema_type: Some(SchemaTypeVariant::Single(SchemaType::Null)),
                        ..Default::default()
                    }),
                    Box::new(optional_schema),
                ];
            }
        }
        Some((id, schema))
    }

    fn get_type_dependencies(&self, type_id: TypeId) -> HashSet<TypeId> {
        let Some(type_reg) = self.get(type_id) else {
            return HashSet::new();
        };
        let internal_schema_type = InternalSchemaType::from_type_registration(type_reg, self);

        internal_schema_type.get_dependencies(self)
    }

    fn build_schema_for_type_id_with_definitions(
        &self,
        type_id: TypeId,
        metadata: &SchemaTypesMetadata,
    ) -> Option<JsonSchemaBevyType> {
        let (_, mut schema) = self.build_schema_for_type_id(type_id, metadata)?;
        let dependencies = self.get_type_dependencies(type_id);
        schema.schema = Some(super::json_schema::SchemaMarker.into());
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

    fn build_schema_reference_for_type_id(
        &self,
        type_id: TypeId,
        field_data: Option<&SchemaFieldData>,
    ) -> Option<JsonSchemaBevyType> {
        let type_reg = self.get(type_id)?;
        let description = field_data.and_then(SchemaFieldData::to_description);

        let ref_type = Some(TypeReferencePath::definition(
            type_reg.type_info().type_path(),
        ));

        let mut schema = JsonSchemaBevyType {
            description,
            kind: Some(SchemaKind::from_type_reg(type_reg)),
            ref_type,
            type_path: type_reg.type_info().type_path().into(),
            schema_type: None,
            ..default()
        };
        let internal = InternalSchemaType::from_type_registration(type_reg, self);
        schema.schema_type = (&internal).into();
        match internal {
            InternalSchemaType::PrimitiveType {
                type_id,
                primitive: _,
                field_data: p_field_data,
            } => {
                let mut range: MinMaxValues = type_id.into();
                if let Some(data) = BASE_TYPES_INFO.get(&type_id) {
                    schema.not = data.not.map(|s| {
                        Box::new(JsonSchemaBevyType {
                            const_value: Some(s.into()),
                            ..default()
                        })
                    });
                    schema.kind = Some(data.schema_kind);
                }
                if let Some(field_range) = field_data
                    .as_ref()
                    .and_then(|d| d.attributes.get_range_by_id(type_id))
                {
                    range = range.with(field_range);
                }
                if let Some(field_range) = p_field_data
                    .as_ref()
                    .and_then(|d| d.attributes.get_range_by_id(type_id))
                {
                    range = range.with(field_range);
                }

                schema.minimum = range.min.get_inclusive();
                schema.maximum = range.max.get_inclusive();
                schema.exclusive_minimum = range.min.get_exclusive();
                schema.exclusive_maximum = range.max.get_exclusive();
                schema.ref_type = None;
                return Some(schema);
            }
            InternalSchemaType::Array {
                element_ty,
                min_size,
                max_size,
            } => {
                schema.kind = Some(SchemaKind::Array);
                schema.ref_type = None;
                schema.schema_type = Some(SchemaTypeVariant::Single(SchemaType::Array));
                let items_schema = self.build_schema_reference_for_type_id(element_ty, None);
                schema.items = items_schema.map(|i| JsonSchemaVariant::Schema(Box::new(i)));
                schema.min_items = min_size;
                schema.max_items = max_size;
            }
            InternalSchemaType::Map { key, value } => {
                schema.ref_type = None;
                schema.schema_type = Some(SchemaTypeVariant::Single(SchemaType::Object));
                schema.kind = Some(SchemaKind::Map);
                schema.key_type = self
                    .build_schema_reference_for_type_id(key, None)
                    .map(Box::new);
                schema.value_type = self
                    .build_schema_reference_for_type_id(value, None)
                    .map(Box::new);

                if let Some(p) = try_get_regex_for_type(key) {
                    schema.pattern_properties =
                        [(p, schema.value_type.clone().unwrap_or_default())].into();
                    schema.additional_properties = Some(JsonSchemaVariant::BoolValue(false));
                } else {
                    schema.additional_properties =
                        schema.value_type.clone().map(JsonSchemaVariant::Schema);
                }
            }
            InternalSchemaType::Optional { generic } => {
                let schema_optional = self
                    .build_schema_reference_for_type_id(generic.ty().id(), None)
                    .unwrap_or_default();
                schema.ref_type = None;
                schema.one_of = vec![
                    Box::new(JsonSchemaBevyType {
                        schema_type: SchemaType::Null.into(),
                        ..Default::default()
                    }),
                    schema_optional.into(),
                ];
            }
            _ => {}
        }
        if schema.ref_type.is_some() {
            schema.schema_type = None;
        }

        Some(schema)
    }
}

#[cfg(test)]
pub(super) mod tests {
    use bevy_ecs::{
        component::Component,
        entity::{Entity, EntityHashMap},
        name::Name,
        reflect::AppTypeRegistry,
    };
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
            .unwrap_or_else(|errors| {
                panic!(
                    "Failed to build schema validator because of errors: {:?}, schema: {}",
                    errors,
                    serde_json::to_string_pretty(&schema).unwrap_or_default()
                )
            });
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
            if let Err(error) = schema_validator.validate(value) {
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
                schema_validator.validate(value).is_err(),
                "Validation should fail for invalid value: {}, schema: {}",
                value,
                serde_json::to_string_pretty(&schema_value).unwrap_or_default()
            );
        }
    }

    #[test]
    fn custom_range_test() {
        #[derive(Reflect, Default, Deserialize, Serialize)]
        pub struct StructTest {
            /// Test documentation
            #[reflect(@10..=13_i32)]
            value: i32,
        }
        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<StructTest>();
        }
        let type_registry = atr.read();
        let (_, schema) = type_registry
            .build_schema_for_type_id(TypeId::of::<StructTest>(), &Default::default())
            .expect("");
        let JsonSchemaVariant::Schema(field_schema) = &schema.properties["value"] else {
            panic!("Should be a schema");
        };
        let range: MinMaxValues = (&**field_schema).into();
        assert_eq!(range.min, Some(BoundValue::Inclusive(10.into())));
        assert_eq!(range.max, Some(BoundValue::Inclusive(13.into())));
        assert_eq!(
            field_schema.schema_type,
            Some(SchemaTypeVariant::Single(SchemaType::Integer))
        );
        #[cfg(feature = "documentation")]
        assert_eq!(field_schema.description, Some("Test documentation".into()));
    }

    #[test]
    fn custom_range_test_usize() {
        #[derive(Reflect, Default, Deserialize, Serialize)]
        pub struct StructTest {
            /// Test documentation
            #[reflect(@..13_usize)]
            some_value: usize,
        }
        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<StructTest>();
        }
        let type_registry = atr.read();
        let (_, schema) = type_registry
            .build_schema_for_type_id(TypeId::of::<StructTest>(), &Default::default())
            .expect("");
        validate::<StructTest>(
            schema,
            &[StructTest { some_value: 5 }],
            &[
                serde_json::json!({"some_value": 5}),
                serde_json::json!({"some_value": 1}),
            ],
            &[
                serde_json::json!({"some_value": 1111111}),
                serde_json::json!({"some_value": -5555}),
                serde_json::json!({"some_value": 1,"ss": 2}),
            ],
        );
    }

    #[cfg(feature = "bevy_math")]
    #[test]
    fn other_ss_test() {
        #[derive(Reflect, Default, Deserialize, Serialize)]
        struct Foo {
            /// Test doc
            a: u16,
        }
        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<bevy_math::Vec3>();
            register.register::<Foo>();
            register.register_type_data::<bevy_math::Vec3, ReflectJsonSchemaForceAsArray>();
        }
        let type_registry = atr.read();
        let (_, schema) = type_registry
            .build_schema_for_type_id(TypeId::of::<Foo>(), &Default::default())
            .expect("");
        validate::<Foo>(
            schema,
            &[Foo { a: 5 }, Foo { a: 1111 }],
            &[serde_json::json!({"a": 5}), serde_json::json!({"a": 1})],
            &[
                serde_json::json!({"a": 1111111}),
                serde_json::json!({"ab": -5555}),
                serde_json::json!({"a": 5555,"b": 5555}),
            ],
        );
        let (_, schema) = type_registry
            .build_schema_for_type_id(TypeId::of::<bevy_math::Vec3>(), &Default::default())
            .expect("");

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
        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<TupleTest>();
        }
        let types = atr.read();
        let schema = types
            .build_schema_for_type_id_with_definitions(
                TypeId::of::<TupleTest>(),
                &Default::default(),
            )
            .expect("");
        let range: MinMaxValues = (&schema).into();
        assert_eq!(range.min, Some(BoundValue::Inclusive(0.into())));
        assert_eq!(range.max, Some(BoundValue::Exclusive(13.into())));
        assert_eq!(
            schema.schema_type,
            Some(SchemaTypeVariant::Single(SchemaType::Integer))
        );
        #[cfg(feature = "documentation")]
        assert_eq!(schema.description, Some("Test documentation".into()));
        validate::<TupleTest>(
            schema,
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
        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<EnumTest>();
        }
        let types = atr.read();
        let schema = types
            .build_schema_for_type_id_with_definitions(
                TypeId::of::<EnumTest>(),
                &Default::default(),
            )
            .expect("");
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
        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<ArrayComponent>();
        }
        let types = atr.read();
        let schema = types
            .build_schema_for_type_id_with_definitions(
                TypeId::of::<ArrayComponent>(),
                &Default::default(),
            )
            .expect("");
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
    fn reflect_entity_hashmap() {
        #[derive(Reflect, Default, Deserialize, Serialize)]
        struct S {
            map: EntityHashMap<String>,
        }
        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<S>();
        }
        let types = atr.read();
        let schema = types
            .build_schema_for_type_id_with_definitions(TypeId::of::<S>(), &Default::default())
            .expect("");
        validate::<S>(
            schema,
            &[S {
                map: [
                    (Entity::from_bits(5), "Sd".to_string()),
                    (Entity::from_bits(15), "Sas".to_string()),
                    (Entity::from_bits(55), "Sa".to_string()),
                ]
                .into(),
            }],
            &[
                serde_json::json!({"map": {"5": "dd"}}),
                serde_json::json!({"map": {"5": "dasdas"}}),
            ],
            &[
                serde_json::json!({"map": {"5.5": 10}}),
                serde_json::json!({"map": {"s": 10}}),
            ],
        );
    }

    #[test]
    fn reflect_struct_with_hashmap() {
        #[derive(Reflect, Default, Deserialize, Serialize)]
        pub struct HashMapStruct {
            pub map: HashMap<i32, Option<i32>>,
        }
        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<HashMapStruct>();
        }
        let types = atr.read();
        let schema = types
            .build_schema_for_type_id_with_definitions(
                TypeId::of::<HashMapStruct>(),
                &Default::default(),
            )
            .expect("");
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
    fn name_field_test() {
        #[derive(Reflect, Deserialize, Serialize)]
        pub struct StructWithNameField {
            pub name: Name,
            pub entity: Entity,
        }
        impl Default for StructWithNameField {
            fn default() -> Self {
                Self {
                    name: Name::new(""),
                    entity: Entity::from_bits(1),
                }
            }
        }
        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<StructWithNameField>();
        }
        let types = atr.read();
        let schema = types
            .build_schema_for_type_id_with_definitions(
                TypeId::of::<StructWithNameField>(),
                &Default::default(),
            )
            .expect("");
        validate::<StructWithNameField>(
            schema,
            &[StructWithNameField {
                name: Name::new("test"),
                entity: Entity::from_bits(11),
            }],
            &[
                serde_json::json!({"name": "value", "entity": 1}),
                serde_json::json!({"name": "other", "entity": u32::MAX - 1}),
            ],
            &[
                serde_json::json!({"name": "other", "entity": u32::MAX}),
                serde_json::json!({"name": "other", "entity": 0}),
                serde_json::json!({"name1": "value"}),
                serde_json::json!({"name": serde_json::Value::Null}),
                serde_json::json!({}),
                serde_json::json!(serde_json::Value::Null),
            ],
        );
    }

    #[test]
    fn test_out_optional_tuple() {
        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<Option<(i8, Option<i16>)>>();
        }
        let types = atr.read();

        let schema = types
            .build_schema_for_type_id_with_definitions(
                TypeId::of::<Option<(i8, Option<i16>)>>(),
                &Default::default(),
            )
            .expect("");
        validate::<Option<(i8, Option<i16>)>>(schema, &[None], &[], &[]);
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

        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<TupleStruct>();
        }
        let types = atr.read();
        let schema = types
            .build_schema_for_type_id_with_definitions(
                TypeId::of::<TupleStruct>(),
                &Default::default(),
            )
            .expect("");
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
        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<TupleStruct>();
        }
        let types = atr.read();
        let schema = types
            .build_schema_for_type_id_with_definitions(
                TypeId::of::<TupleStruct>(),
                &Default::default(),
            )
            .expect("");

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
        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<TupleStruct>();
        }
        let types = atr.read();
        let schema = types
            .build_schema_for_type_id_with_definitions(
                TypeId::of::<TupleStruct>(),
                &Default::default(),
            )
            .expect("");
        let range: MinMaxValues = (&schema).into();
        assert!(!range.in_range(51));
        assert!(range.in_range(15));
        assert!(range.in_range(50));
        assert!(!range.in_range(51));

        validate::<TupleStruct>(
            schema,
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
