use crate::{serde::Serializable, Reflect, ReflectMut, ReflectRef};
use bevy_utils::HashMap;
use std::{any::Any, borrow::Cow, collections::hash_map::Entry};

/// A reflected Rust regular struct type.
///
/// Implementors of this trait allow their fields to be addressed by name as
/// well as by index.
///
/// This trait is automatically implemented for `struct` types with named fields
/// when using `#[derive(Reflect)]`.
///
/// # Example
///
/// ```
/// use bevy_reflect::{Reflect, Struct};
///
/// #[derive(Reflect)]
/// struct Foo {
///     bar: String,
/// }
///
/// # fn main() {
/// let foo = Foo { bar: "Hello, world!".to_string() };
///
/// assert_eq!(foo.field_len(), 1);
/// assert_eq!(foo.name_at(0), Some("bar"));
///
/// let bar = foo.field("bar").unwrap();
/// assert_eq!(bar.downcast_ref::<String>(), Some(&"Hello, world!".to_string()));
/// # }
/// ```
pub trait Struct: Reflect {
    /// Returns a reference to the value of the field named `name` as a `&dyn
    /// Reflect`.
    fn field(&self, name: &str) -> Option<&dyn Reflect>;

    /// Returns a mutable reference to the value of the field named `name` as a
    /// `&mut dyn Reflect`.
    fn field_mut(&mut self, name: &str) -> Option<&mut dyn Reflect>;

    /// Returns a reference to the value of the field with index `index` as a
    /// `&dyn Reflect`.
    fn field_at(&self, index: usize) -> Option<&dyn Reflect>;

    /// Returns a mutable reference to the value of the field with index `index`
    /// as a `&mut dyn Reflect`.
    fn field_at_mut(&mut self, index: usize) -> Option<&mut dyn Reflect>;

    /// Returns the name of the field with index `index`.
    fn name_at(&self, index: usize) -> Option<&str>;

    /// Returns the number of fields in the struct.
    fn field_len(&self) -> usize;

    /// Returns an iterator over the values of the struct's fields.
    fn iter_fields(&self) -> FieldIter;

    /// Clones the struct into a [`DynamicStruct`].
    fn clone_dynamic(&self) -> DynamicStruct;
}

/// An iterator over the field values of a struct.
pub struct FieldIter<'a> {
    pub(crate) struct_val: &'a dyn Struct,
    pub(crate) index: usize,
}

impl<'a> FieldIter<'a> {
    pub fn new(value: &'a dyn Struct) -> Self {
        FieldIter {
            struct_val: value,
            index: 0,
        }
    }
}

impl<'a> Iterator for FieldIter<'a> {
    type Item = &'a dyn Reflect;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.struct_val.field_at(self.index);
        self.index += 1;
        value
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.struct_val.field_len();
        (size, Some(size))
    }
}

impl<'a> ExactSizeIterator for FieldIter<'a> {}

/// A convenience trait which combines fetching and downcasting of struct
/// fields.
///
/// # Example
///
/// ```
/// use bevy_reflect::{GetField, Reflect};
///
/// #[derive(Reflect)]
/// struct Foo {
///     bar: String,
/// }
///
/// # fn main() {
/// let mut foo = Foo { bar: "Hello, world!".to_string() };
///
/// foo.get_field_mut::<String>("bar").unwrap().truncate(5);
/// assert_eq!(foo.get_field::<String>("bar"), Some(&"Hello".to_string()));
/// # }
/// ```
pub trait GetField {
    /// Returns a reference to the value of the field named `name`, downcast to
    /// `T`.
    fn get_field<T: Reflect>(&self, name: &str) -> Option<&T>;

    /// Returns a mutable reference to the value of the field named `name`,
    /// downcast to `T`.
    fn get_field_mut<T: Reflect>(&mut self, name: &str) -> Option<&mut T>;
}

impl<S: Struct> GetField for S {
    fn get_field<T: Reflect>(&self, name: &str) -> Option<&T> {
        self.field(name).and_then(|value| value.downcast_ref::<T>())
    }

    fn get_field_mut<T: Reflect>(&mut self, name: &str) -> Option<&mut T> {
        self.field_mut(name)
            .and_then(|value| value.downcast_mut::<T>())
    }
}

impl GetField for dyn Struct {
    fn get_field<T: Reflect>(&self, name: &str) -> Option<&T> {
        self.field(name).and_then(|value| value.downcast_ref::<T>())
    }

    fn get_field_mut<T: Reflect>(&mut self, name: &str) -> Option<&mut T> {
        self.field_mut(name)
            .and_then(|value| value.downcast_mut::<T>())
    }
}

/// A struct type which allows fields to be added at runtime.
#[derive(Default)]
pub struct DynamicStruct {
    name: String,
    fields: Vec<Box<dyn Reflect>>,
    field_names: Vec<Cow<'static, str>>,
    field_indices: HashMap<Cow<'static, str>, usize>,
}

impl DynamicStruct {
    /// Returns the name of the struct.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the name of the struct.
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// Inserts a field named `name` with value `value` into the struct.
    ///
    /// If the field already exists, it is overwritten.
    pub fn insert_boxed(&mut self, name: &str, value: Box<dyn Reflect>) {
        let name = Cow::Owned(name.to_string());
        match self.field_indices.entry(name) {
            Entry::Occupied(entry) => {
                self.fields[*entry.get()] = value;
            }
            Entry::Vacant(entry) => {
                self.fields.push(value);
                self.field_names.push(entry.key().clone());
                entry.insert(self.fields.len() - 1);
            }
        }
    }

    /// Inserts a field named `name` with the typed value `value` into the struct.
    ///
    /// If the field already exists, it is overwritten.
    pub fn insert<T: Reflect>(&mut self, name: &str, value: T) {
        if let Some(index) = self.field_indices.get(name) {
            self.fields[*index] = Box::new(value);
        } else {
            self.insert_boxed(name, Box::new(value));
        }
    }
}

impl Struct for DynamicStruct {
    #[inline]
    fn field(&self, name: &str) -> Option<&dyn Reflect> {
        self.field_indices
            .get(name)
            .map(|index| &*self.fields[*index])
    }

    #[inline]
    fn field_mut(&mut self, name: &str) -> Option<&mut dyn Reflect> {
        if let Some(index) = self.field_indices.get(name) {
            Some(&mut *self.fields[*index])
        } else {
            None
        }
    }

    #[inline]
    fn field_at(&self, index: usize) -> Option<&dyn Reflect> {
        self.fields.get(index).map(|value| &**value)
    }

    #[inline]
    fn field_at_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        self.fields.get_mut(index).map(|value| &mut **value)
    }

    #[inline]
    fn name_at(&self, index: usize) -> Option<&str> {
        self.field_names.get(index).map(|name| name.as_ref())
    }

    #[inline]
    fn field_len(&self) -> usize {
        self.fields.len()
    }

    #[inline]
    fn iter_fields(&self) -> FieldIter {
        FieldIter {
            struct_val: self,
            index: 0,
        }
    }

    fn clone_dynamic(&self) -> DynamicStruct {
        DynamicStruct {
            name: self.name.clone(),
            field_names: self.field_names.clone(),
            field_indices: self.field_indices.clone(),
            fields: self
                .fields
                .iter()
                .map(|value| value.clone_value())
                .collect(),
        }
    }
}

// SAFE: any and any_mut both return self
unsafe impl Reflect for DynamicStruct {
    #[inline]
    fn type_name(&self) -> &str {
        &self.name
    }

    #[inline]
    fn any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn any_mut(&mut self) -> &mut dyn Any {
        self
    }

    #[inline]
    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(self.clone_dynamic())
    }

    #[inline]
    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::Struct(self)
    }

    #[inline]
    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::Struct(self)
    }

    fn apply(&mut self, value: &dyn Reflect) {
        if let ReflectRef::Struct(struct_value) = value.reflect_ref() {
            for (i, value) in struct_value.iter_fields().enumerate() {
                let name = struct_value.name_at(i).unwrap();
                if let Some(v) = self.field_mut(name) {
                    v.apply(value)
                }
            }
        } else {
            panic!("Attempted to apply non-struct type to struct type.");
        }
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
    }

    fn reflect_hash(&self) -> Option<u64> {
        None
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        struct_partial_eq(self, value)
    }

    fn serializable(&self) -> Option<Serializable> {
        None
    }
}

#[inline]
pub fn struct_partial_eq<S: Struct>(a: &S, b: &dyn Reflect) -> Option<bool> {
    let struct_value = if let ReflectRef::Struct(struct_value) = b.reflect_ref() {
        struct_value
    } else {
        return Some(false);
    };

    if a.field_len() != struct_value.field_len() {
        return Some(false);
    }

    for (i, value) in struct_value.iter_fields().enumerate() {
        let name = struct_value.name_at(i).unwrap();
        if let Some(field_value) = a.field(name) {
            if let Some(false) | None = field_value.reflect_partial_eq(value) {
                return Some(false);
            }
        } else {
            return Some(false);
        }
    }

    Some(true)
}
