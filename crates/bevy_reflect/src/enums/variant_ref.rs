use crate::{Reflect, Struct, Tuple, TupleStructFieldIter};
use bevy_utils::HashMap;
use std::borrow::Cow;
use std::ops::{Deref, DerefMut};
use std::slice::Iter;

pub enum VariantRef<'a> {
    Unit,
    Tuple(TupleVariantRef<'a>),
    Struct(StructVariantRef<'a>),
}

pub enum VariantMut<'a> {
    Unit,
    Tuple(TupleVariantMut<'a>),
    Struct(StructVariantMut<'a>),
}

pub struct TupleVariantRef<'a> {
    fields: Vec<VariantFieldRef<'a>>,
}

impl<'a> TupleVariantRef<'a> {
    /// Creates a new [`TupleVariantRef`].
    pub fn new(fields: Vec<VariantFieldRef<'a>>) -> Self {
        Self { fields }
    }

    /// Returns a reference to the value of the field at the given index.
    pub fn field(&self, index: usize) -> Option<&dyn Reflect> {
        self.fields.get(index).map(AsRef::as_ref)
    }

    /// Returns an iterator over the values of the variant's fields.
    pub fn iter_fields(&self) -> Iter<'_, VariantFieldRef<'a>> {
        self.fields.iter()
    }

    /// Returns the number of fields in the variant.
    pub fn field_len(&self) -> usize {
        self.fields.len()
    }
}

pub struct TupleVariantMut<'a> {
    fields: Vec<VariantFieldMut<'a>>,
}

impl<'a> TupleVariantMut<'a> {
    /// Creates a new [`TupleVariantMut`].
    pub fn new(fields: Vec<VariantFieldMut<'a>>) -> Self {
        Self { fields }
    }

    /// Returns a reference to the value of the field at the given index.
    pub fn field(&self, index: usize) -> Option<&dyn Reflect> {
        self.fields.get(index).map(AsRef::as_ref)
    }

    /// Returns a mutable reference to the value of the field at the given index.
    pub fn field_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        self.fields.get_mut(index).map(AsMut::as_mut)
    }

    /// Returns an iterator over the values of the variant's fields.
    pub fn iter_fields(&self) -> Iter<'_, VariantFieldMut<'a>> {
        self.fields.iter()
    }

    /// Returns the number of fields in the variant.
    pub fn field_len(&self) -> usize {
        self.fields.len()
    }
}

pub struct StructVariantRef<'a> {
    fields: Vec<VariantFieldRef<'a>>,
    field_indices: HashMap<Cow<'static, str>, usize>,
}

impl<'a> StructVariantRef<'a> {
    /// Creates a new [`StructVariantRef`].
    pub fn new(
        fields: Vec<VariantFieldRef<'a>>,
        field_indices: HashMap<Cow<'static, str>, usize>,
    ) -> Self {
        Self {
            fields,
            field_indices,
        }
    }

    /// Returns a reference to the value of the field with the given name.
    pub fn field(&self, name: &str) -> Option<&dyn Reflect> {
        self.field_indices
            .get(name)
            .map(|index| self.fields[*index].as_ref())
    }

    /// Returns a reference to the value of the field at the given index.
    pub fn field_at(&self, index: usize) -> Option<&dyn Reflect> {
        self.fields.get(index).map(AsRef::as_ref)
    }

    /// Returns the index of the field with the given name.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.field_indices.get(name).copied()
    }

    /// Returns an iterator over the values of the variant's fields.
    pub fn iter_fields(&self) -> Iter<'_, VariantFieldRef<'a>> {
        self.fields.iter()
    }

    /// Returns the number of fields in the variant.
    pub fn field_len(&self) -> usize {
        self.fields.len()
    }
}

pub struct StructVariantMut<'a> {
    fields: Vec<VariantFieldMut<'a>>,
    field_indices: HashMap<Cow<'static, str>, usize>,
}

impl<'a> StructVariantMut<'a> {
    /// Creates a new [`StructVariantMut`].
    pub fn new(
        fields: Vec<VariantFieldMut<'a>>,
        field_indices: HashMap<Cow<'static, str>, usize>,
    ) -> Self {
        Self {
            fields,
            field_indices,
        }
    }

    /// Returns a reference to the value of the field with the given name.
    pub fn field(&self, name: &str) -> Option<&dyn Reflect> {
        self.field_indices
            .get(name)
            .map(|index| self.fields[*index].as_ref())
    }

    /// Returns a reference to the value of the field at the given index.
    pub fn field_at(&self, index: usize) -> Option<&dyn Reflect> {
        self.fields.get(index).map(AsRef::as_ref)
    }

    /// Returns a mutable reference to the value of the field with the given name.
    pub fn field_mut(&mut self, name: &str) -> Option<&mut dyn Reflect> {
        self.field_indices
            .get_mut(name)
            .map(|index| self.fields[*index].as_mut())
    }

    /// Returns a mutable reference to the value of the field at the given index.
    pub fn field_at_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        self.fields.get_mut(index).map(AsMut::as_mut)
    }

    /// Returns the index of the field with the given name.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.field_indices.get(name).copied()
    }

    /// Returns an iterator over the values of the variant's fields.
    pub fn iter_fields(&self) -> Iter<'_, VariantFieldMut<'a>> {
        self.fields.iter()
    }

    /// Returns the number of fields in the variant.
    pub fn field_len(&self) -> usize {
        self.fields.len()
    }
}

/// A wrapper around an immutable reference to a variant's field.
#[derive(Copy, Clone)]
pub struct VariantFieldRef<'a>(&'a dyn Reflect);

impl<'a> VariantFieldRef<'a> {
    pub fn new(field: &'a dyn Reflect) -> Self {
        Self(field)
    }
}

impl<'a> AsRef<dyn Reflect> for VariantFieldRef<'a> {
    fn as_ref(&self) -> &dyn Reflect {
        self.0
    }
}

impl<'a> Deref for VariantFieldRef<'a> {
    type Target = dyn Reflect;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

/// A wrapper around a mutable reference to a variant's field.
pub struct VariantFieldMut<'a>(&'a mut dyn Reflect);

impl<'a> VariantFieldMut<'a> {
    pub fn new(field: &'a mut dyn Reflect) -> Self {
        Self(field)
    }
}

impl<'a> AsRef<dyn Reflect> for VariantFieldMut<'a> {
    fn as_ref(&self) -> &dyn Reflect {
        self.0
    }
}

impl<'a> AsMut<dyn Reflect> for VariantFieldMut<'a> {
    fn as_mut(&mut self) -> &mut dyn Reflect {
        self.0
    }
}

impl<'a> Deref for VariantFieldMut<'a> {
    type Target = dyn Reflect;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a> DerefMut for VariantFieldMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}
