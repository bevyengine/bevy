use bevy_reflect_derive::impl_type_path;

use crate::{
    self as bevy_reflect, enum_debug, enum_hash, enum_partial_eq, ApplyError, DynamicStruct,
    DynamicTuple, Enum, Reflect, ReflectKind, ReflectMut, ReflectOwned, ReflectRef, Struct, Tuple,
    TypeInfo, VariantFieldIter, VariantType,
};
use std::any::Any;
use std::fmt::Formatter;

/// A dynamic representation of an enum variant.
#[derive(Debug, Default)]
pub enum DynamicVariant {
    #[default]
    Unit,
    Tuple(DynamicTuple),
    Struct(DynamicStruct),
}

impl Clone for DynamicVariant {
    fn clone(&self) -> Self {
        match self {
            DynamicVariant::Unit => DynamicVariant::Unit,
            DynamicVariant::Tuple(data) => DynamicVariant::Tuple(data.clone_dynamic()),
            DynamicVariant::Struct(data) => DynamicVariant::Struct(data.clone_dynamic()),
        }
    }
}

impl From<DynamicTuple> for DynamicVariant {
    fn from(dyn_tuple: DynamicTuple) -> Self {
        Self::Tuple(dyn_tuple)
    }
}

impl From<DynamicStruct> for DynamicVariant {
    fn from(dyn_struct: DynamicStruct) -> Self {
        Self::Struct(dyn_struct)
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
/// # use bevy_reflect::{DynamicEnum, DynamicVariant, Reflect};
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
/// value.apply(&dyn_enum);
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
    ///
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
    ///
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
                "expected TypeInfo::Enum but received: {:?}",
                represented_type
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

    /// Create a [`DynamicEnum`] from an existing one.
    ///
    /// This is functionally the same as [`DynamicEnum::from_ref`] except it takes an owned value.
    pub fn from<TEnum: Enum>(value: TEnum) -> Self {
        Self::from_ref(&value)
    }

    /// Create a [`DynamicEnum`] from an existing one.
    ///
    /// This is functionally the same as [`DynamicEnum::from`] except it takes a reference.
    pub fn from_ref<TEnum: Enum>(value: &TEnum) -> Self {
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
                    data.insert_boxed(field.value().clone_value());
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
                    data.insert_boxed(name, field.value().clone_value());
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
    fn field(&self, name: &str) -> Option<&dyn Reflect> {
        if let DynamicVariant::Struct(data) = &self.variant {
            data.field(name)
        } else {
            None
        }
    }

    fn field_at(&self, index: usize) -> Option<&dyn Reflect> {
        if let DynamicVariant::Tuple(data) = &self.variant {
            data.field(index)
        } else {
            None
        }
    }

    fn field_mut(&mut self, name: &str) -> Option<&mut dyn Reflect> {
        if let DynamicVariant::Struct(data) = &mut self.variant {
            data.field_mut(name)
        } else {
            None
        }
    }

    fn field_at_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        if let DynamicVariant::Tuple(data) = &mut self.variant {
            data.field_mut(index)
        } else {
            None
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

    fn iter_fields(&self) -> VariantFieldIter {
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

    fn clone_dynamic(&self) -> DynamicEnum {
        Self {
            represented_type: self.represented_type,
            variant_index: self.variant_index,
            variant_name: self.variant_name.clone(),
            variant: self.variant.clone(),
        }
    }
}

impl Reflect for DynamicEnum {
    #[inline]
    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        self.represented_type
    }

    #[inline]
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    #[inline]
    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
        self
    }

    #[inline]
    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    #[inline]
    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }

    #[inline]
    fn try_apply(&mut self, value: &dyn Reflect) -> Result<(), ApplyError> {
        if let ReflectRef::Enum(value) = value.reflect_ref() {
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
                            dyn_tuple.insert_boxed(field.value().clone_value());
                        }
                        DynamicVariant::Tuple(dyn_tuple)
                    }
                    VariantType::Struct => {
                        let mut dyn_struct = DynamicStruct::default();
                        for field in value.iter_fields() {
                            dyn_struct
                                .insert_boxed(field.name().unwrap(), field.value().clone_value());
                        }
                        DynamicVariant::Struct(dyn_struct)
                    }
                };
                self.set_variant(value.variant_name(), dyn_variant);
            }
        } else {
            return Err(ApplyError::MismatchedKinds {
                from_kind: value.reflect_kind(),
                to_kind: ReflectKind::Enum,
            });
        }
        Ok(())
    }

    #[inline]
    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
    }

    #[inline]
    fn reflect_kind(&self) -> ReflectKind {
        ReflectKind::Enum
    }

    #[inline]
    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::Enum(self)
    }

    #[inline]
    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::Enum(self)
    }

    #[inline]
    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Enum(self)
    }

    #[inline]
    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(self.clone_dynamic())
    }

    #[inline]
    fn reflect_hash(&self) -> Option<u64> {
        enum_hash(self)
    }

    #[inline]
    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        enum_partial_eq(self, value)
    }

    #[inline]
    fn debug(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "DynamicEnum(")?;
        enum_debug(self, f)?;
        write!(f, ")")
    }
}

impl_type_path!((in bevy_reflect) DynamicEnum);
