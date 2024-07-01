use crate::attributes::{impl_custom_attribute_methods, CustomAttributes};
use crate::{NamedField, UnnamedField};
use bevy_utils::HashMap;
use std::slice::Iter;
use std::sync::Arc;

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
        let field_names = fields.iter().map(|field| field.name()).collect();
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
