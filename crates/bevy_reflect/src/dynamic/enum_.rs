use core::fmt::{Debug, Formatter};

use bevy_reflect_derive::impl_type_path;

use crate::{
    dynamic::{DynamicStruct, DynamicTuple, DynamicValue, DynamicValueRef},
    enums::{Enum, EnumInfo, VariantType},
    ApplyError, DynamicTypePath, ReflectCloneError, TypeInfo,
};

use alloc::{format, string::String};
use derive_more::derive::From;

/// A dynamic representation of an enum variant.
#[derive(Debug, Default, From)]
pub enum DynamicVariant {
    /// A unit variant.
    #[default]
    Unit,
    /// A tuple variant.
    Tuple(DynamicTuple),
    /// A struct variant.
    Struct(DynamicStruct),
}

impl DynamicVariant {
    /// Attempts to clone this value's data.
    ///
    /// May fail if an opaque value cannot be cloned.
    pub fn dynamic_clone(&self) -> Result<Self, ReflectCloneError> {
        Ok(match self {
            Self::Unit => Self::Unit,
            Self::Tuple(value) => Self::Tuple(value.dynamic_clone()?),
            Self::Struct(value) => Self::Struct(value.dynamic_clone()?),
        })
    }
}

/// # Panics
///
/// `Clone` cannot fail, so deep-cloning a variant whose fields contain a non-cloneable
/// opaque value panics. This is unavoidable: the field-by-field deep clone relies on
/// `Reflect::reflect_clone`, which is fallible to avoid exactly this problem.
impl Clone for DynamicVariant {
    fn clone(&self) -> Self {
        self.dynamic_clone().unwrap()
    }
}

impl From<()> for DynamicVariant {
    fn from(_: ()) -> Self {
        Self::Unit
    }
}

/// A dynamic representation of an enum.
///
/// This allows for enums to be configured at runtime.
///
/// # Example
///
/// ```
/// # use bevy_reflect::{dynamic::{DynamicEnum, DynamicVariant}, Reflect, Reflect};
///
/// // The original enum value
/// let mut value: Option<usize> = Some(123);
///
/// // Create a DynamicEnum to represent the new value
/// let mut dyn_enum = DynamicEnum::new(
///   "None",
///   DynamicVariant::Unit
/// );
///
/// // Apply the DynamicEnum as a patch to the original value
/// value.apply(dyn_enum.as_partial_reflect());
///
/// // Tada!
/// assert_eq!(None, value);
/// ```
#[derive(Default)]
pub struct DynamicEnum {
    pub(crate) represented_type: Option<&'static TypeInfo>,
    variant_name: String,
    variant_index: usize,
    variant: DynamicVariant,
}

impl DynamicEnum {
    /// Create a new [`DynamicEnum`] to represent an enum at runtime.
    ///
    /// # Arguments
    ///
    /// * `variant_name`: The name of the variant to set
    /// * `variant`: The variant data
    pub fn new<I: Into<String>, V: Into<DynamicVariant>>(variant_name: I, variant: V) -> Self {
        Self {
            represented_type: None,
            variant_index: 0,
            variant_name: variant_name.into(),
            variant: variant.into(),
        }
    }

    /// Create a new [`DynamicEnum`] with a variant index to represent an enum at runtime.
    ///
    /// # Arguments
    ///
    /// * `variant_index`: The index of the variant to set
    /// * `variant_name`: The name of the variant to set
    /// * `variant`: The variant data
    pub fn new_with_index<I: Into<String>, V: Into<DynamicVariant>>(
        variant_index: usize,
        variant_name: I,
        variant: V,
    ) -> Self {
        Self {
            represented_type: None,
            variant_index,
            variant_name: variant_name.into(),
            variant: variant.into(),
        }
    }

    /// Sets the [type] to be represented by this `DynamicEnum`.
    ///
    /// # Panics
    ///
    /// Panics if the given [type] is not a [`TypeInfo::Enum`].
    ///
    /// [type]: TypeInfo
    pub fn set_represented_type(&mut self, represented_type: Option<&'static TypeInfo>) {
        if let Some(represented_type) = represented_type {
            assert!(
                matches!(represented_type, TypeInfo::Enum(_)),
                "expected TypeInfo::Enum but received: {represented_type:?}",
            );
        }

        self.represented_type = represented_type;
    }

    /// Set the current enum variant represented by this struct.
    pub fn set_variant<I: Into<String>, V: Into<DynamicVariant>>(&mut self, name: I, variant: V) {
        self.variant_name = name.into();
        self.variant = variant.into();
    }

    /// Set the current enum variant represented by this struct along with its variant index.
    pub fn set_variant_with_index<I: Into<String>, V: Into<DynamicVariant>>(
        &mut self,
        variant_index: usize,
        variant_name: I,
        variant: V,
    ) {
        self.variant_index = variant_index;
        self.variant_name = variant_name.into();
        self.variant = variant.into();
    }

    /// Get a reference to the [`DynamicVariant`] contained in `self`.
    pub fn variant(&self) -> &DynamicVariant {
        &self.variant
    }

    /// Get a mutable reference to the [`DynamicVariant`] contained in `self`.
    ///
    /// Using the mut reference to switch to a different variant will ___not___ update the
    /// internal tracking of the variant name and index.
    ///
    /// If you want to switch variants, prefer one of the setters:
    /// [`DynamicEnum::set_variant`] or [`DynamicEnum::set_variant_with_index`].
    pub fn variant_mut(&mut self) -> &mut DynamicVariant {
        &mut self.variant
    }

    /// Create a [`DynamicEnum`] from an existing one.
    ///
    /// This is functionally the same as [`DynamicEnum::from_ref`] except it takes an owned value.
    #[deprecated(
        since = "0.20.0",
        note = "Use `DynamicEnum::try_from` instead, which returns a Result."
    )]
    pub fn from<TEnum: Enum>(value: TEnum) -> Self {
        Self::try_from(value).unwrap()
    }

    /// Create a [`DynamicEnum`] from an existing one.
    ///
    /// This is functionally the same as [`DynamicEnum::try_from_ref`] except it takes an owned value.
    ///
    /// Returns an error if any field of the active variant cannot be converted via
    /// [`Reflect::to_dynamic`].
    pub fn try_from<TEnum: Enum>(value: TEnum) -> Result<Self, ReflectCloneError> {
        Self::try_from_ref(&value)
    }

    /// Create a [`DynamicEnum`] from an existing one.
    ///
    /// This is functionally the same as [`DynamicEnum::from`] except it takes a reference.
    #[deprecated(
        since = "0.20.0",
        note = "Use `DynamicEnum::try_from_ref` instead, which returns a Result."
    )]
    pub fn from_ref<TEnum: Enum + ?Sized>(value: &TEnum) -> Self {
        Self::try_from_ref(value).unwrap()
    }

    /// Create a [`DynamicEnum`] from an existing one.
    ///
    /// This is functionally the same as [`DynamicEnum::try_from`] except it takes a reference.
    ///
    /// Returns an error if any field of the active variant cannot be converted via
    /// [`Reflect::to_dynamic`].
    pub fn try_from_ref<TEnum: Enum + ?Sized>(value: &TEnum) -> Result<Self, ReflectCloneError> {
        let type_info = value.reflect_type_info();
        let mut dyn_enum = match value.variant_type() {
            VariantType::Unit => DynamicEnum::new_with_index(
                value.variant_index(),
                value.variant_name(),
                DynamicVariant::Unit,
            ),
            VariantType::Tuple => {
                let mut data = DynamicTuple::default();
                for field in value.iter_fields() {
                    data.insert(DynamicValue::try_from_reflect(field.value())?);
                }
                DynamicEnum::new_with_index(
                    value.variant_index(),
                    value.variant_name(),
                    DynamicVariant::Tuple(data),
                )
            }
            VariantType::Struct => {
                let mut data = DynamicStruct::default();
                for field in value.iter_fields() {
                    let name = field.name().unwrap();
                    data.insert(name, DynamicValue::try_from_reflect(field.value())?);
                }
                DynamicEnum::new_with_index(
                    value.variant_index(),
                    value.variant_name(),
                    DynamicVariant::Struct(data),
                )
            }
        };

        dyn_enum.set_represented_type(Some(type_info));
        Ok(dyn_enum)
    }

    /// Apply the values in `self` to the provided `reflect` using reflection.
    ///
    /// If this function fails, the `reflect` may be partially mutated.
    pub fn try_apply(self, reflect: &mut dyn Enum) -> Result<(), ApplyError> {
        if self.variant_name != reflect.variant_name() {
            // New variant -> perform a switch.
            return reflect.set_from_dynamic(self);
        }

        if self.variant_type() != reflect.variant_type() {
            return Err(ApplyError::WrongEnumVariantType {
                from_type: self.variant_type(),
                to_type: reflect.variant_type(),
            });
        }

        // Same variant -> just update fields
        match self.variant {
            DynamicVariant::Unit => {}
            DynamicVariant::Struct(value) => {
                for (name, value) in value.field_names.into_iter().zip(value.fields.into_iter()) {
                    let Some(field) = reflect.field_mut(&name) else {
                        // TODO: Should this be an error?
                        continue;
                    };
                    value.try_apply(field)?;
                }
            }
            DynamicVariant::Tuple(value) => {
                for (index, value) in value.fields.into_iter().enumerate() {
                    let Some(field) = reflect.field_at_mut(index) else {
                        // TODO: Should this be an error?
                        continue;
                    };
                    value.try_apply(field)?;
                }
            }
        }
        Ok(())
    }

    /// Attempts to clone this value's data.
    ///
    /// May fail if an opaque value cannot be cloned.
    pub fn dynamic_clone(&self) -> Result<Self, ReflectCloneError> {
        Ok(Self {
            represented_type: self.represented_type,
            variant_name: self.variant_name.clone(),
            variant_index: self.variant_index,
            variant: self.variant.dynamic_clone()?,
        })
    }

    /// Gets the value of a field from the current variant by the field's name.
    ///
    /// Returns [`None`] if there is no field with such a name, or if the enum is not a struct
    /// variant.
    pub fn field(&self, name: &str) -> Option<&DynamicValue> {
        if let DynamicVariant::Struct(data) = &self.variant {
            data.field(name)
        } else {
            None
        }
    }

    /// Gets the value of a field from the current variant by the field's index.
    ///
    /// Returns [`None`] if there is no field with such a name, or if the enum is a unit variant.
    pub fn field_at(&self, index: usize) -> Option<&DynamicValue> {
        match &self.variant {
            DynamicVariant::Tuple(data) => data.field(index),
            DynamicVariant::Struct(data) => data.field_at(index),
            DynamicVariant::Unit => None,
        }
    }

    /// Gets the value mutably of a field from the current variant by the field's name.
    ///
    /// Returns [`None`] if there is no field with such a name, or if the enum is not a struct
    /// variant.
    pub fn field_mut(&mut self, name: &str) -> Option<&mut DynamicValue> {
        if let DynamicVariant::Struct(data) = &mut self.variant {
            data.field_mut(name)
        } else {
            None
        }
    }

    /// Gets the value mutably of a field from the current variant by the field's index.
    ///
    /// Returns [`None`] if there is no field with such a name, or if the enum is a unit variant.
    pub fn field_at_mut(&mut self, index: usize) -> Option<&mut DynamicValue> {
        match &mut self.variant {
            DynamicVariant::Tuple(data) => data.field_mut(index),
            DynamicVariant::Struct(data) => data.field_at_mut(index),
            DynamicVariant::Unit => None,
        }
    }

    /// Gets the index of a field by its name in the current variant.
    ///
    /// Returns [`None`] if there is no field with such a name, or if the enum is not a struct
    /// variant.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        if let DynamicVariant::Struct(data) = &self.variant {
            data.index_of_name(name)
        } else {
            None
        }
    }

    /// Gets the name of a field by its index in the current variant.
    ///
    /// Returns [`None`] if the field index is out of range, or if the enum is not a struct variant.
    pub fn name_at(&self, index: usize) -> Option<&str> {
        if let DynamicVariant::Struct(data) = &self.variant {
            data.name_at(index)
        } else {
            None
        }
    }

    /// Iterates through the fields if the enum.
    pub fn iter_fields(&self) -> impl Iterator<Item = DynamicVariantField<'_>> + ExactSizeIterator {
        match &self.variant {
            DynamicVariant::Unit => DynamicEnumFieldIter::Unit,
            DynamicVariant::Struct(value) => DynamicEnumFieldIter::Struct(
                value
                    .iter_fields()
                    .map(|(name, value)| DynamicVariantField::Struct(name, value.as_ref())),
            ),
            DynamicVariant::Tuple(value) => DynamicEnumFieldIter::Tuple(
                value
                    .iter_fields()
                    .map(|value| DynamicVariantField::Tuple(value.as_ref())),
            ),
        }
    }

    /// Returns the number of fields in the enum.
    pub fn field_len(&self) -> usize {
        match &self.variant {
            DynamicVariant::Unit => 0,
            DynamicVariant::Tuple(data) => data.field_len(),
            DynamicVariant::Struct(data) => data.field_len(),
        }
    }

    /// Returns the name of the current variant of the enum.
    pub fn variant_name(&self) -> &str {
        &self.variant_name
    }

    /// Returns the index of the current variant of the enum.
    pub fn variant_index(&self) -> usize {
        self.variant_index
    }

    /// Returns the type of the current variant.
    pub fn variant_type(&self) -> VariantType {
        match &self.variant {
            DynamicVariant::Unit => VariantType::Unit,
            DynamicVariant::Tuple(..) => VariantType::Tuple,
            DynamicVariant::Struct(..) => VariantType::Struct,
        }
    }

    /// Returns the type path of the current variant.
    pub fn variant_path(&self) -> String {
        format!("{}::{}", self.reflect_type_path(), self.variant_name())
    }

    /// Gets the type info of the represented enum, if this dynamic value represents a known type.
    pub fn reflect_enum_info(&self) -> Option<&'static EnumInfo> {
        self.represented_type?.as_enum().ok()
    }
}

impl Debug for DynamicEnum {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "DynamicEnum(")?;
        match &self.variant {
            DynamicVariant::Unit => {
                f.write_str(self.variant_name())?;
            }
            DynamicVariant::Tuple(value) => {
                let mut debug = f.debug_tuple(&self.variant_name);
                for field in value.fields.iter() {
                    debug.field(field as &dyn Debug);
                }
                debug.finish()?;
            }
            DynamicVariant::Struct(value) => {
                let mut debug = f.debug_struct(&self.variant_name);
                for (field, name) in value.fields.iter().zip(value.field_names.iter()) {
                    debug.field(name, field as &dyn Debug);
                }
                debug.finish()?;
            }
        }
        write!(f, ")")
    }
}

impl_type_path!((in bevy_reflect::dynamic) DynamicEnum);

/// A field in the current [`DynamicEnum`] variant.
pub enum DynamicVariantField<'a> {
    /// The name and value of a field in a struct variant.
    Struct(&'a str, DynamicValueRef<'a>),
    /// The value of a field in a tuple variant.
    Tuple(DynamicValueRef<'a>),
}

impl<'a> DynamicVariantField<'a> {
    /// Returns the name of a struct variant field, or [`None`] for a tuple variant field.
    pub fn name(&self) -> Option<&'a str> {
        if let Self::Struct(name, ..) = self {
            Some(*name)
        } else {
            None
        }
    }

    /// Gets a reference to the value of this field.
    pub fn value(&self) -> DynamicValueRef<'a> {
        match *self {
            Self::Struct(_, value) | Self::Tuple(value) => value,
        }
    }
}

/// An enum of iterators for different enum variant types.
enum DynamicEnumFieldIter<StructIter, TupleIter> {
    /// The enum being iterated is a unit type.
    Unit,
    /// The enum being iterated is a struct variant.
    Struct(StructIter),
    /// The enum being iterated is a tuple struct variant.
    Tuple(TupleIter),
}

impl<'a, T, StructIter, TupleIter> Iterator for DynamicEnumFieldIter<StructIter, TupleIter>
where
    StructIter: Iterator<Item = T>,
    TupleIter: Iterator<Item = T>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Unit => None,
            Self::Struct(iter) => iter.next(),
            Self::Tuple(iter) => iter.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Unit => (0, Some(0)),
            Self::Struct(iter) => iter.size_hint(),
            Self::Tuple(iter) => iter.size_hint(),
        }
    }
}

impl<'a, T, StructIter, TupleIter> ExactSizeIterator for DynamicEnumFieldIter<StructIter, TupleIter>
where
    StructIter: Iterator<Item = T> + ExactSizeIterator,
    TupleIter: Iterator<Item = T> + ExactSizeIterator,
{
    fn len(&self) -> usize {
        match self {
            Self::Unit => 0,
            Self::Struct(iter) => iter.len(),
            Self::Tuple(iter) => iter.len(),
        }
    }
}
