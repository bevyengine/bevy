use bevy_reflect_derive::impl_type_path;

use crate::{
    enum_debug, enum_hash, enum_partial_eq, ApplyError, DynamicStruct, DynamicTuple, Enum,
    PartialReflect, Reflect, ReflectKind, ReflectMut, ReflectOwned, ReflectRef, Struct, Tuple,
    TypeInfo, VariantFieldIter, VariantType,
};

use alloc::{boxed::Box, string::String};
use core::fmt::Formatter;
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

impl Clone for DynamicVariant {
    fn clone(&self) -> Self {
        match self {
            DynamicVariant::Unit => DynamicVariant::Unit,
            DynamicVariant::Tuple(data) => DynamicVariant::Tuple(data.to_dynamic_tuple()),
            DynamicVariant::Struct(data) => DynamicVariant::Struct(data.to_dynamic_struct()),
        }
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
/// # use bevy_reflect::{DynamicEnum, DynamicVariant, Reflect, PartialReflect};
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
#[derive(Default, Debug)]
pub struct DynamicEnum {
    represented_type: Option<&'static TypeInfo>,
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
    pub fn from<TEnum: Enum>(value: TEnum) -> Self {
        Self::from_ref(&value)
    }

    /// Create a [`DynamicEnum`] from an existing one.
    ///
    /// This is functionally the same as [`DynamicEnum::from`] except it takes a reference.
    pub fn from_ref<TEnum: Enum + ?Sized>(value: &TEnum) -> Self {
        let type_info = value.get_represented_type_info();
        let mut dyn_enum = match value.variant_type() {
            VariantType::Unit => DynamicEnum::new_with_index(
                value.variant_index(),
                value.variant_name(),
                DynamicVariant::Unit,
            ),
            VariantType::Tuple => {
                let mut data = DynamicTuple::default();
                for field in value.iter_fields() {
                    data.insert_boxed(field.value().to_dynamic());
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
                    data.insert_boxed(name, field.value().to_dynamic());
                }
                DynamicEnum::new_with_index(
                    value.variant_index(),
                    value.variant_name(),
                    DynamicVariant::Struct(data),
                )
            }
        };

        dyn_enum.set_represented_type(type_info);
        dyn_enum
    }
}

impl Enum for DynamicEnum {
    fn field(&self, name: &str) -> Option<&dyn PartialReflect> {
        if let DynamicVariant::Struct(data) = &self.variant {
            data.field(name)
        } else {
            None
        }
    }

    fn field_at(&self, index: usize) -> Option<&dyn PartialReflect> {
        match &self.variant {
            DynamicVariant::Tuple(data) => data.field(index),
            DynamicVariant::Struct(data) => data.field_at(index),
            DynamicVariant::Unit => None,
        }
    }

    fn field_mut(&mut self, name: &str) -> Option<&mut dyn PartialReflect> {
        if let DynamicVariant::Struct(data) = &mut self.variant {
            data.field_mut(name)
        } else {
            None
        }
    }

    fn field_at_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        match &mut self.variant {
            DynamicVariant::Tuple(data) => data.field_mut(index),
            DynamicVariant::Struct(data) => data.field_at_mut(index),
            DynamicVariant::Unit => None,
        }
    }

    fn index_of(&self, name: &str) -> Option<usize> {
        if let DynamicVariant::Struct(data) = &self.variant {
            data.index_of(name)
        } else {
            None
        }
    }

    fn name_at(&self, index: usize) -> Option<&str> {
        if let DynamicVariant::Struct(data) = &self.variant {
            data.name_at(index)
        } else {
            None
        }
    }

    fn iter_fields(&self) -> VariantFieldIter<'_> {
        VariantFieldIter::new(self)
    }

    fn field_len(&self) -> usize {
        match &self.variant {
            DynamicVariant::Unit => 0,
            DynamicVariant::Tuple(data) => data.field_len(),
            DynamicVariant::Struct(data) => data.field_len(),
        }
    }

    fn variant_name(&self) -> &str {
        &self.variant_name
    }

    fn variant_index(&self) -> usize {
        self.variant_index
    }

    fn variant_type(&self) -> VariantType {
        match &self.variant {
            DynamicVariant::Unit => VariantType::Unit,
            DynamicVariant::Tuple(..) => VariantType::Tuple,
            DynamicVariant::Struct(..) => VariantType::Struct,
        }
    }
}

impl PartialReflect for DynamicEnum {
    #[inline]
    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        self.represented_type
    }

    #[inline]
    fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
        self
    }

    #[inline]
    fn as_partial_reflect(&self) -> &dyn PartialReflect {
        self
    }

    #[inline]
    fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        self
    }

    fn try_into_reflect(self: Box<Self>) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
        Err(self)
    }

    fn try_as_reflect(&self) -> Option<&dyn Reflect> {
        None
    }

    fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> {
        None
    }

    #[inline]
    fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
        let value = value.reflect_ref().as_enum()?;

        if Enum::variant_name(self) == value.variant_name() {
            // Same variant -> just update fields
            match value.variant_type() {
                VariantType::Struct => {
                    for field in value.iter_fields() {
                        let name = field.name().unwrap();
                        if let Some(v) = Enum::field_mut(self, name) {
                            v.try_apply(field.value())?;
                        }
                    }
                }
                VariantType::Tuple => {
                    for (index, field) in value.iter_fields().enumerate() {
                        if let Some(v) = Enum::field_at_mut(self, index) {
                            v.try_apply(field.value())?;
                        }
                    }
                }
                _ => {}
            }
        } else {
            // New variant -> perform a switch
            let dyn_variant = match value.variant_type() {
                VariantType::Unit => DynamicVariant::Unit,
                VariantType::Tuple => {
                    let mut dyn_tuple = DynamicTuple::default();
                    for field in value.iter_fields() {
                        dyn_tuple.insert_boxed(field.value().to_dynamic());
                    }
                    DynamicVariant::Tuple(dyn_tuple)
                }
                VariantType::Struct => {
                    let mut dyn_struct = DynamicStruct::default();
                    for field in value.iter_fields() {
                        dyn_struct.insert_boxed(field.name().unwrap(), field.value().to_dynamic());
                    }
                    DynamicVariant::Struct(dyn_struct)
                }
            };
            self.set_variant(value.variant_name(), dyn_variant);
        }

        Ok(())
    }

    #[inline]
    fn reflect_kind(&self) -> ReflectKind {
        ReflectKind::Enum
    }

    #[inline]
    fn reflect_ref(&self) -> ReflectRef<'_> {
        ReflectRef::Enum(self)
    }

    #[inline]
    fn reflect_mut(&mut self) -> ReflectMut<'_> {
        ReflectMut::Enum(self)
    }

    #[inline]
    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Enum(self)
    }

    #[inline]
    fn reflect_hash(&self) -> Option<u64> {
        enum_hash(self)
    }

    #[inline]
    fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
        enum_partial_eq(self, value)
    }

    #[inline]
    fn debug(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "DynamicEnum(")?;
        enum_debug(self, f)?;
        write!(f, ")")
    }

    #[inline]
    fn is_dynamic(&self) -> bool {
        true
    }
}

impl_type_path!((in bevy_reflect) DynamicEnum);
