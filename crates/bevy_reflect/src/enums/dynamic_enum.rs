use crate::utility::NonGenericTypeInfoCell;
use crate::{
    enum_hash, enum_partial_eq, DynamicInfo, DynamicStruct, DynamicTuple, Enum, Reflect,
    ReflectMut, ReflectRef, Struct, Tuple, TypeInfo, Typed, VariantFieldIter, VariantType,
};
use std::any::Any;

/// A dynamic representation of an enum variant.
pub enum DynamicVariant {
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

impl Default for DynamicVariant {
    fn default() -> Self {
        DynamicVariant::Unit
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
///   Reflect::type_name(&value),
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
#[derive(Default)]
pub struct DynamicEnum {
    name: String,
    variant_name: String,
    variant: DynamicVariant,
}

impl DynamicEnum {
    /// Create a new [`DynamicEnum`] to represent an enum at runtime.
    ///
    /// # Arguments
    ///
    /// * `name`: The type name of the enum
    /// * `variant_name`: The name of the variant to set
    /// * `variant`: The variant data
    ///
    pub fn new<I: Into<String>>(name: I, variant_name: I, variant: DynamicVariant) -> Self {
        Self {
            name: name.into(),
            variant_name: variant_name.into(),
            variant,
        }
    }

    /// Returns the type name of the enum.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the type name of the enum.
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// Set the current enum variant represented by this struct.
    pub fn set_variant<I: Into<String>>(&mut self, name: I, variant: DynamicVariant) {
        self.variant_name = name.into();
        self.variant = variant;
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
        match value.variant_type() {
            VariantType::Unit => DynamicEnum::new(
                value.type_name(),
                value.variant_name(),
                DynamicVariant::Unit,
            ),
            VariantType::Tuple => {
                let mut data = DynamicTuple::default();
                for field in value.iter_fields() {
                    data.insert_boxed(field.clone_value());
                }
                DynamicEnum::new(
                    value.type_name(),
                    value.variant_name(),
                    DynamicVariant::Tuple(data),
                )
            }
            VariantType::Struct => {
                let mut data = DynamicStruct::default();
                for (index, field) in value.iter_fields().enumerate() {
                    let name = value.name_at(index).unwrap();
                    data.insert_boxed(name, field.clone_value());
                }
                DynamicEnum::new(
                    value.type_name(),
                    value.variant_name(),
                    DynamicVariant::Struct(data),
                )
            }
        }
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

    fn variant_type(&self) -> VariantType {
        match &self.variant {
            DynamicVariant::Unit => VariantType::Unit,
            DynamicVariant::Tuple(..) => VariantType::Tuple,
            DynamicVariant::Struct(..) => VariantType::Struct,
        }
    }

    fn clone_dynamic(&self) -> DynamicEnum {
        Self {
            name: self.name.clone(),
            variant_name: self.variant_name.clone(),
            variant: self.variant.clone(),
        }
    }
}

impl Reflect for DynamicEnum {
    #[inline]
    fn type_name(&self) -> &str {
        &self.name
    }

    #[inline]
    fn get_type_info(&self) -> &'static TypeInfo {
        <Self as Typed>::type_info()
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
    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    #[inline]
    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }

    #[inline]
    fn apply(&mut self, value: &dyn Reflect) {
        if let ReflectRef::Enum(enum_value) = value.reflect_ref() {
            for (i, value) in enum_value.iter_fields().enumerate() {
                let name = enum_value.name_at(i).unwrap();
                if let Some(v) = self.field_mut(name) {
                    v.apply(value);
                }
            }
        } else {
            panic!("Attempted to apply non-enum type to enum type.");
        }
    }

    #[inline]
    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
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
}

impl Typed for DynamicEnum {
    fn type_info() -> &'static TypeInfo {
        static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
        CELL.get_or_set(|| TypeInfo::Dynamic(DynamicInfo::new::<Self>()))
    }
}
