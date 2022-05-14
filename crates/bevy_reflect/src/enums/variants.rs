use crate::{NamedField, UnnamedField};
use bevy_utils::HashMap;
use std::borrow::Cow;
use std::slice::Iter;

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
    pub fn name(&self) -> &Cow<'static, str> {
        match self {
            Self::Struct(info) => info.name(),
            Self::Tuple(info) => info.name(),
            Self::Unit(info) => info.name(),
        }
    }
}

/// Type info for struct variants.
#[derive(Clone, Debug)]
pub struct StructVariantInfo {
    name: Cow<'static, str>,
    fields: Box<[NamedField]>,
    field_indices: HashMap<Cow<'static, str>, usize>,
}

impl StructVariantInfo {
    /// The name of this variant.
    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
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
}

/// Type info for tuple variants.
#[derive(Clone, Debug)]
pub struct TupleVariantInfo {
    name: Cow<'static, str>,
    fields: Box<[UnnamedField]>,
}

impl TupleVariantInfo {
    /// The name of this variant.
    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
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
}

/// Type info for unit variants.
#[derive(Clone, Debug)]
pub struct UnitVariantInfo {
    name: Cow<'static, str>,
}

impl UnitVariantInfo {
    /// The name of this variant.
    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
    }
}
