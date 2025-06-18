use crate::{
    attributes::{impl_custom_attribute_methods, CustomAttributes},
    NamedField, UnnamedField,
};
use alloc::boxed::Box;
use bevy_platform::collections::HashMap;
use bevy_platform::sync::Arc;
use core::slice::Iter;
use thiserror::Error;

/// Describes the form of an enum variant.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum VariantType {
    /// Struct enums take the form:
    ///
    /// ```
    /// enum MyEnum {
    ///   A {
    ///     foo: usize
    ///   }
    /// }
    /// ```
    Struct,
    /// Tuple enums take the form:
    ///
    /// ```
    /// enum MyEnum {
    ///   A(usize)
    /// }
    /// ```
    Tuple,
    /// Unit enums take the form:
    ///
    /// ```
    /// enum MyEnum {
    ///   A
    /// }
    /// ```
    Unit,
}

/// A [`VariantInfo`]-specific error.
#[derive(Debug, Error)]
pub enum VariantInfoError {
    /// Caused when a variant was expected to be of a certain [type], but was not.
    ///
    /// [type]: VariantType
    #[error("variant type mismatch: expected {expected:?}, received {received:?}")]
    TypeMismatch {
        /// Expected variant type.
        expected: VariantType,
        /// Received variant type.
        received: VariantType,
    },
}

/// A container for compile-time enum variant info.
#[derive(Clone, Debug)]
pub enum VariantInfo {
    /// Struct enums take the form:
    ///
    /// ```
    /// enum MyEnum {
    ///   A {
    ///     foo: usize
    ///   }
    /// }
    /// ```
    Struct(StructVariantInfo),
    /// Tuple enums take the form:
    ///
    /// ```
    /// enum MyEnum {
    ///   A(usize)
    /// }
    /// ```
    Tuple(TupleVariantInfo),
    /// Unit enums take the form:
    ///
    /// ```
    /// enum MyEnum {
    ///   A
    /// }
    /// ```
    Unit(UnitVariantInfo),
}

impl VariantInfo {
    /// The name of the enum variant.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Struct(info) => info.name(),
            Self::Tuple(info) => info.name(),
            Self::Unit(info) => info.name(),
        }
    }

    /// The docstring of the underlying variant, if any.
    #[cfg(feature = "documentation")]
    pub fn docs(&self) -> Option<&str> {
        match self {
            Self::Struct(info) => info.docs(),
            Self::Tuple(info) => info.docs(),
            Self::Unit(info) => info.docs(),
        }
    }

    /// Returns the [type] of this variant.
    ///
    /// [type]: VariantType
    pub fn variant_type(&self) -> VariantType {
        match self {
            Self::Struct(_) => VariantType::Struct,
            Self::Tuple(_) => VariantType::Tuple,
            Self::Unit(_) => VariantType::Unit,
        }
    }

    impl_custom_attribute_methods!(
        self,
        match self {
            Self::Struct(info) => info.custom_attributes(),
            Self::Tuple(info) => info.custom_attributes(),
            Self::Unit(info) => info.custom_attributes(),
        },
        "variant"
    );
}

macro_rules! impl_cast_method {
    ($name:ident : $kind:ident => $info:ident) => {
        #[doc = concat!("Attempts a cast to [`", stringify!($info), "`].")]
        #[doc = concat!("\n\nReturns an error if `self` is not [`VariantInfo::", stringify!($kind), "`].")]
        pub fn $name(&self) -> Result<&$info, VariantInfoError> {
            match self {
                Self::$kind(info) => Ok(info),
                _ => Err(VariantInfoError::TypeMismatch {
                    expected: VariantType::$kind,
                    received: self.variant_type(),
                }),
            }
        }
    };
}

/// Conversion convenience methods for [`VariantInfo`].
impl VariantInfo {
    impl_cast_method!(as_struct_variant: Struct => StructVariantInfo);
    impl_cast_method!(as_tuple_variant: Tuple => TupleVariantInfo);
    impl_cast_method!(as_unit_variant: Unit => UnitVariantInfo);
}

/// Type info for struct variants.
#[derive(Clone, Debug)]
pub struct StructVariantInfo {
    name: &'static str,
    fields: Box<[NamedField]>,
    field_names: Box<[&'static str]>,
    field_indices: HashMap<&'static str, usize>,
    custom_attributes: Arc<CustomAttributes>,
    #[cfg(feature = "documentation")]
    docs: Option<&'static str>,
}

impl StructVariantInfo {
    /// Create a new [`StructVariantInfo`].
    pub fn new(name: &'static str, fields: &[NamedField]) -> Self {
        let field_indices = Self::collect_field_indices(fields);
        let field_names = fields.iter().map(NamedField::name).collect();
        Self {
            name,
            fields: fields.to_vec().into_boxed_slice(),
            field_names,
            field_indices,
            custom_attributes: Arc::new(CustomAttributes::default()),
            #[cfg(feature = "documentation")]
            docs: None,
        }
    }

    /// Sets the docstring for this variant.
    #[cfg(feature = "documentation")]
    pub fn with_docs(self, docs: Option<&'static str>) -> Self {
        Self { docs, ..self }
    }

    /// Sets the custom attributes for this variant.
    pub fn with_custom_attributes(self, custom_attributes: CustomAttributes) -> Self {
        Self {
            custom_attributes: Arc::new(custom_attributes),
            ..self
        }
    }

    /// The name of this variant.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// A slice containing the names of all fields in order.
    pub fn field_names(&self) -> &[&'static str] {
        &self.field_names
    }

    /// Get the field with the given name.
    pub fn field(&self, name: &str) -> Option<&NamedField> {
        self.field_indices
            .get(name)
            .map(|index| &self.fields[*index])
    }

    /// Get the field at the given index.
    pub fn field_at(&self, index: usize) -> Option<&NamedField> {
        self.fields.get(index)
    }

    /// Get the index of the field with the given name.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.field_indices.get(name).copied()
    }

    /// Iterate over the fields of this variant.
    pub fn iter(&self) -> Iter<'_, NamedField> {
        self.fields.iter()
    }

    /// The total number of fields in this variant.
    pub fn field_len(&self) -> usize {
        self.fields.len()
    }

    fn collect_field_indices(fields: &[NamedField]) -> HashMap<&'static str, usize> {
        fields
            .iter()
            .enumerate()
            .map(|(index, field)| (field.name(), index))
            .collect()
    }

    /// The docstring of this variant, if any.
    #[cfg(feature = "documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }

    impl_custom_attribute_methods!(self.custom_attributes, "variant");
}

/// Type info for tuple variants.
#[derive(Clone, Debug)]
pub struct TupleVariantInfo {
    name: &'static str,
    fields: Box<[UnnamedField]>,
    custom_attributes: Arc<CustomAttributes>,
    #[cfg(feature = "documentation")]
    docs: Option<&'static str>,
}

impl TupleVariantInfo {
    /// Create a new [`TupleVariantInfo`].
    pub fn new(name: &'static str, fields: &[UnnamedField]) -> Self {
        Self {
            name,
            fields: fields.to_vec().into_boxed_slice(),
            custom_attributes: Arc::new(CustomAttributes::default()),
            #[cfg(feature = "documentation")]
            docs: None,
        }
    }

    /// Sets the docstring for this variant.
    #[cfg(feature = "documentation")]
    pub fn with_docs(self, docs: Option<&'static str>) -> Self {
        Self { docs, ..self }
    }

    /// Sets the custom attributes for this variant.
    pub fn with_custom_attributes(self, custom_attributes: CustomAttributes) -> Self {
        Self {
            custom_attributes: Arc::new(custom_attributes),
            ..self
        }
    }

    /// The name of this variant.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Get the field at the given index.
    pub fn field_at(&self, index: usize) -> Option<&UnnamedField> {
        self.fields.get(index)
    }

    /// Iterate over the fields of this variant.
    pub fn iter(&self) -> Iter<'_, UnnamedField> {
        self.fields.iter()
    }

    /// The total number of fields in this variant.
    pub fn field_len(&self) -> usize {
        self.fields.len()
    }

    /// The docstring of this variant, if any.
    #[cfg(feature = "documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }

    impl_custom_attribute_methods!(self.custom_attributes, "variant");
}

/// Type info for unit variants.
#[derive(Clone, Debug)]
pub struct UnitVariantInfo {
    name: &'static str,
    custom_attributes: Arc<CustomAttributes>,
    #[cfg(feature = "documentation")]
    docs: Option<&'static str>,
}

impl UnitVariantInfo {
    /// Create a new [`UnitVariantInfo`].
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            custom_attributes: Arc::new(CustomAttributes::default()),
            #[cfg(feature = "documentation")]
            docs: None,
        }
    }

    /// Sets the docstring for this variant.
    #[cfg(feature = "documentation")]
    pub fn with_docs(self, docs: Option<&'static str>) -> Self {
        Self { docs, ..self }
    }

    /// Sets the custom attributes for this variant.
    pub fn with_custom_attributes(self, custom_attributes: CustomAttributes) -> Self {
        Self {
            custom_attributes: Arc::new(custom_attributes),
            ..self
        }
    }

    /// The name of this variant.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The docstring of this variant, if any.
    #[cfg(feature = "documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }

    impl_custom_attribute_methods!(self.custom_attributes, "variant");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Reflect, Typed};

    #[test]
    fn should_return_error_on_invalid_cast() {
        #[derive(Reflect)]
        enum Foo {
            Bar,
        }

        let info = Foo::type_info().as_enum().unwrap();
        let variant = info.variant_at(0).unwrap();
        assert!(matches!(
            variant.as_tuple_variant(),
            Err(VariantInfoError::TypeMismatch {
                expected: VariantType::Tuple,
                received: VariantType::Unit
            })
        ));
    }
}
