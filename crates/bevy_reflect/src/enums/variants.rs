use crate::{NamedField, UnnamedField};
use bevy_utils::HashMap;
use std::borrow::Cow;
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
    /// Create a new [`StructVariantInfo`].
    pub fn new(name: &str, fields: &[NamedField]) -> Self {
        let field_indices = Self::collect_field_indices(fields);

        Self {
            name: Cow::Owned(name.into()),
            fields: fields.to_vec().into_boxed_slice(),
            field_indices,
        }
    }

    /// Create a new [`StructVariantInfo`] using a static string.
    ///
    /// This helps save an allocation when the string has a static lifetime, such
    /// as when using defined sa a literal.
    pub fn new_static(name: &'static str, fields: &[NamedField]) -> Self {
        let field_indices = Self::collect_field_indices(fields);
        Self {
            name: Cow::Borrowed(name),
            fields: fields.to_vec().into_boxed_slice(),
            field_indices,
        }
    }

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

    fn collect_field_indices(fields: &[NamedField]) -> HashMap<Cow<'static, str>, usize> {
        fields
            .iter()
            .enumerate()
            .map(|(index, field)| {
                let name = field.name().clone();
                (name, index)
            })
            .collect()
    }
}

/// Type info for tuple variants.
#[derive(Clone, Debug)]
pub struct TupleVariantInfo {
    name: Cow<'static, str>,
    fields: Box<[UnnamedField]>,
}

impl TupleVariantInfo {
    /// Create a new [`TupleVariantInfo`].
    pub fn new(name: &str, fields: &[UnnamedField]) -> Self {
        Self {
            name: Cow::Owned(name.into()),
            fields: fields.to_vec().into_boxed_slice(),
        }
    }

    /// Create a new [`TupleVariantInfo`] using a static string.
    ///
    /// This helps save an allocation when the string has a static lifetime, such
    /// as when using defined sa a literal.
    pub fn new_static(name: &'static str, fields: &[UnnamedField]) -> Self {
        Self {
            name: Cow::Borrowed(name),
            fields: fields.to_vec().into_boxed_slice(),
        }
    }

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
    /// Create a new [`UnitVariantInfo`].
    pub fn new(name: &str) -> Self {
        Self {
            name: Cow::Owned(name.into()),
        }
    }

    /// Create a new [`UnitVariantInfo`] using a static string.
    ///
    /// This helps save an allocation when the string has a static lifetime, such
    /// as when using defined sa a literal.
    pub fn new_static(name: &'static str) -> Self {
        Self {
            name: Cow::Borrowed(name),
        }
    }

    /// The name of this variant.
    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
    }
}
