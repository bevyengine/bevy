use crate::{NamedField, UnnamedField};
use bevy_utils::HashMap;
use std::slice::Iter;

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

    /// The metadata of the underlying variant.
    pub fn meta(&self) -> &VariantMeta {
        match self {
            Self::Struct(info) => info.meta(),
            Self::Tuple(info) => info.meta(),
            Self::Unit(info) => info.meta(),
        }
    }
}

/// Type info for struct variants.
#[derive(Clone, Debug)]
pub struct StructVariantInfo {
    name: &'static str,
    fields: Box<[NamedField]>,
    field_names: Box<[&'static str]>,
    field_indices: HashMap<&'static str, usize>,
    meta: VariantMeta,
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
            meta: VariantMeta::new(),
        }
    }

    /// Add metadata for this variant.
    pub fn with_meta(self, meta: VariantMeta) -> Self {
        Self { meta, ..self }
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

    /// The metadata of the variant.
    pub fn meta(&self) -> &VariantMeta {
        &self.meta
    }
}

/// Type info for tuple variants.
#[derive(Clone, Debug)]
pub struct TupleVariantInfo {
    name: &'static str,
    fields: Box<[UnnamedField]>,
    meta: VariantMeta,
}

impl TupleVariantInfo {
    /// Create a new [`TupleVariantInfo`].
    pub fn new(name: &'static str, fields: &[UnnamedField]) -> Self {
        Self {
            name,
            fields: fields.to_vec().into_boxed_slice(),
            meta: VariantMeta::new(),
        }
    }

    /// Add metadata for this variant.
    pub fn with_meta(self, meta: VariantMeta) -> Self {
        Self { meta, ..self }
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

    /// The metadata of the variant.
    pub fn meta(&self) -> &VariantMeta {
        &self.meta
    }
}

/// Type info for unit variants.
#[derive(Clone, Debug)]
pub struct UnitVariantInfo {
    name: &'static str,
    meta: VariantMeta,
}

impl UnitVariantInfo {
    /// Create a new [`UnitVariantInfo`].
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            meta: VariantMeta::new(),
        }
    }

    /// Add metadata for this variant.
    pub fn with_meta(self, meta: VariantMeta) -> Self {
        Self { meta, ..self }
    }

    /// The name of this variant.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The metadata of the variant.
    pub fn meta(&self) -> &VariantMeta {
        &self.meta
    }
}

#[derive(Clone, Debug)]
pub struct VariantMeta {
    /// The docstring of this variant, if any.
    #[cfg(feature = "documentation")]
    pub docs: Option<&'static str>,
}

impl VariantMeta {
    pub const fn new() -> Self {
        Self {
            #[cfg(feature = "documentation")]
            docs: None,
        }
    }
}

impl Default for VariantMeta {
    fn default() -> Self {
        Self::new()
    }
}
