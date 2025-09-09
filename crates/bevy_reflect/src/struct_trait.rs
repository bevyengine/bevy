use crate::generics::impl_generic_info_methods;
use crate::{
    attributes::{impl_custom_attribute_methods, CustomAttributes},
    type_info::impl_type_methods,
    ApplyError, Generics, NamedField, PartialReflect, Reflect, ReflectKind, ReflectMut,
    ReflectOwned, ReflectRef, Type, TypeInfo, TypePath,
};
use alloc::{borrow::Cow, boxed::Box, vec::Vec};
use bevy_platform::collections::HashMap;
use bevy_platform::sync::Arc;
use bevy_reflect_derive::impl_type_path;
use core::{
    fmt::{Debug, Formatter},
    slice::Iter,
};

/// A trait used to power [struct-like] operations via [reflection].
///
/// This trait uses the [`Reflect`] trait to allow implementors to have their fields
/// be dynamically addressed by both name and index.
///
/// When using [`#[derive(Reflect)]`](derive@crate::Reflect) on a standard struct,
/// this trait will be automatically implemented.
/// This goes for [unit structs] as well.
///
/// # Example
///
/// ```
/// use bevy_reflect::{PartialReflect, Reflect, Struct};
///
/// #[derive(Reflect)]
/// struct Foo {
///     bar: u32,
/// }
///
/// let foo = Foo { bar: 123 };
///
/// assert_eq!(foo.field_len(), 1);
/// assert_eq!(foo.name_at(0), Some("bar"));
///
/// let field: &dyn PartialReflect = foo.field("bar").unwrap();
/// assert_eq!(field.try_downcast_ref::<u32>(), Some(&123));
/// ```
///
/// [struct-like]: https://doc.rust-lang.org/book/ch05-01-defining-structs.html
/// [reflection]: crate
/// [unit structs]: https://doc.rust-lang.org/book/ch05-01-defining-structs.html#unit-like-structs-without-any-fields
pub trait Struct: PartialReflect {
    /// Returns a reference to the value of the field named `name` as a `&dyn
    /// PartialReflect`.
    fn field(&self, name: &str) -> Option<&dyn PartialReflect>;

    /// Returns a mutable reference to the value of the field named `name` as a
    /// `&mut dyn PartialReflect`.
    fn field_mut(&mut self, name: &str) -> Option<&mut dyn PartialReflect>;

    /// Returns a reference to the value of the field with index `index` as a
    /// `&dyn PartialReflect`.
    fn field_at(&self, index: usize) -> Option<&dyn PartialReflect>;

    /// Returns a mutable reference to the value of the field with index `index`
    /// as a `&mut dyn PartialReflect`.
    fn field_at_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect>;

    /// Returns the name of the field with index `index`.
    fn name_at(&self, index: usize) -> Option<&str>;

    /// Returns the number of fields in the struct.
    fn field_len(&self) -> usize;

    /// Returns an iterator over the values of the reflectable fields for this struct.
    fn iter_fields(&self) -> FieldIter<'_>;

    /// Creates a new [`DynamicStruct`] from this struct.
    fn to_dynamic_struct(&self) -> DynamicStruct {
        let mut dynamic_struct = DynamicStruct::default();
        dynamic_struct.set_represented_type(self.get_represented_type_info());
        for (i, value) in self.iter_fields().enumerate() {
            dynamic_struct.insert_boxed(self.name_at(i).unwrap(), value.to_dynamic());
        }
        dynamic_struct
    }

    /// Will return `None` if [`TypeInfo`] is not available.
    fn get_represented_struct_info(&self) -> Option<&'static StructInfo> {
        self.get_represented_type_info()?.as_struct().ok()
    }
}

/// A container for compile-time named struct info.
#[derive(Clone, Debug)]
pub struct StructInfo {
    ty: Type,
    generics: Generics,
    fields: Box<[NamedField]>,
    field_names: Box<[&'static str]>,
    field_indices: HashMap<&'static str, usize>,
    custom_attributes: Arc<CustomAttributes>,
    #[cfg(feature = "documentation")]
    docs: Option<&'static str>,
}

impl StructInfo {
    /// Create a new [`StructInfo`].
    ///
    /// # Arguments
    ///
    /// * `fields`: The fields of this struct in the order they are defined
    pub fn new<T: Reflect + TypePath>(fields: &[NamedField]) -> Self {
        let field_indices = fields
            .iter()
            .enumerate()
            .map(|(index, field)| (field.name(), index))
            .collect::<HashMap<_, _>>();

        let field_names = fields.iter().map(NamedField::name).collect();

        Self {
            ty: Type::of::<T>(),
            generics: Generics::new(),
            fields: fields.to_vec().into_boxed_slice(),
            field_names,
            field_indices,
            custom_attributes: Arc::new(CustomAttributes::default()),
            #[cfg(feature = "documentation")]
            docs: None,
        }
    }

    /// Sets the docstring for this struct.
    #[cfg(feature = "documentation")]
    pub fn with_docs(self, docs: Option<&'static str>) -> Self {
        Self { docs, ..self }
    }

    /// Sets the custom attributes for this struct.
    pub fn with_custom_attributes(self, custom_attributes: CustomAttributes) -> Self {
        Self {
            custom_attributes: Arc::new(custom_attributes),
            ..self
        }
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

    /// Iterate over the fields of this struct.
    pub fn iter(&self) -> Iter<'_, NamedField> {
        self.fields.iter()
    }

    /// The total number of fields in this struct.
    pub fn field_len(&self) -> usize {
        self.fields.len()
    }

    impl_type_methods!(ty);

    /// The docstring of this struct, if any.
    #[cfg(feature = "documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }

    impl_custom_attribute_methods!(self.custom_attributes, "struct");

    impl_generic_info_methods!(generics);
}

/// An iterator over the field values of a struct.
pub struct FieldIter<'a> {
    pub(crate) struct_val: &'a dyn Struct,
    pub(crate) index: usize,
}

impl<'a> FieldIter<'a> {
    /// Creates a new [`FieldIter`].
    pub fn new(value: &'a dyn Struct) -> Self {
        FieldIter {
            struct_val: value,
            index: 0,
        }
    }
}

impl<'a> Iterator for FieldIter<'a> {
    type Item = &'a dyn PartialReflect;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.struct_val.field_at(self.index);
        self.index += value.is_some() as usize;
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
        self.field(name)
            .and_then(|value| value.try_downcast_ref::<T>())
    }

    fn get_field_mut<T: Reflect>(&mut self, name: &str) -> Option<&mut T> {
        self.field_mut(name)
            .and_then(|value| value.try_downcast_mut::<T>())
    }
}

impl GetField for dyn Struct {
    fn get_field<T: Reflect>(&self, name: &str) -> Option<&T> {
        self.field(name)
            .and_then(|value| value.try_downcast_ref::<T>())
    }

    fn get_field_mut<T: Reflect>(&mut self, name: &str) -> Option<&mut T> {
        self.field_mut(name)
            .and_then(|value| value.try_downcast_mut::<T>())
    }
}

/// A struct type which allows fields to be added at runtime.
#[derive(Default)]
pub struct DynamicStruct {
    represented_type: Option<&'static TypeInfo>,
    fields: Vec<Box<dyn PartialReflect>>,
    field_names: Vec<Cow<'static, str>>,
    field_indices: HashMap<Cow<'static, str>, usize>,
}

impl DynamicStruct {
    /// Sets the [type] to be represented by this `DynamicStruct`.
    ///
    /// # Panics
    ///
    /// Panics if the given [type] is not a [`TypeInfo::Struct`].
    ///
    /// [type]: TypeInfo
    pub fn set_represented_type(&mut self, represented_type: Option<&'static TypeInfo>) {
        if let Some(represented_type) = represented_type {
            assert!(
                matches!(represented_type, TypeInfo::Struct(_)),
                "expected TypeInfo::Struct but received: {represented_type:?}"
            );
        }

        self.represented_type = represented_type;
    }

    /// Inserts a field named `name` with value `value` into the struct.
    ///
    /// If the field already exists, it is overwritten.
    pub fn insert_boxed<'a>(
        &mut self,
        name: impl Into<Cow<'a, str>>,
        value: Box<dyn PartialReflect>,
    ) {
        let name: Cow<str> = name.into();
        if let Some(index) = self.field_indices.get(&name) {
            self.fields[*index] = value;
        } else {
            self.fields.push(value);
            self.field_indices
                .insert(Cow::Owned(name.clone().into_owned()), self.fields.len() - 1);
            self.field_names.push(Cow::Owned(name.into_owned()));
        }
    }

    /// Inserts a field named `name` with the typed value `value` into the struct.
    ///
    /// If the field already exists, it is overwritten.
    pub fn insert<'a, T: PartialReflect>(&mut self, name: impl Into<Cow<'a, str>>, value: T) {
        self.insert_boxed(name, Box::new(value));
    }

    /// Gets the index of the field with the given name.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.field_indices.get(name).copied()
    }
}

impl Struct for DynamicStruct {
    #[inline]
    fn field(&self, name: &str) -> Option<&dyn PartialReflect> {
        self.field_indices
            .get(name)
            .map(|index| &*self.fields[*index])
    }

    #[inline]
    fn field_mut(&mut self, name: &str) -> Option<&mut dyn PartialReflect> {
        if let Some(index) = self.field_indices.get(name) {
            Some(&mut *self.fields[*index])
        } else {
            None
        }
    }

    #[inline]
    fn field_at(&self, index: usize) -> Option<&dyn PartialReflect> {
        self.fields.get(index).map(|value| &**value)
    }

    #[inline]
    fn field_at_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        self.fields.get_mut(index).map(|value| &mut **value)
    }

    #[inline]
    fn name_at(&self, index: usize) -> Option<&str> {
        self.field_names.get(index).map(AsRef::as_ref)
    }

    #[inline]
    fn field_len(&self) -> usize {
        self.fields.len()
    }

    #[inline]
    fn iter_fields(&self) -> FieldIter<'_> {
        FieldIter {
            struct_val: self,
            index: 0,
        }
    }
}

impl PartialReflect for DynamicStruct {
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

    fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
        let struct_value = value.reflect_ref().as_struct()?;

        for (i, value) in struct_value.iter_fields().enumerate() {
            let name = struct_value.name_at(i).unwrap();
            if let Some(v) = self.field_mut(name) {
                v.try_apply(value)?;
            }
        }

        Ok(())
    }

    #[inline]
    fn reflect_kind(&self) -> ReflectKind {
        ReflectKind::Struct
    }

    #[inline]
    fn reflect_ref(&self) -> ReflectRef<'_> {
        ReflectRef::Struct(self)
    }

    #[inline]
    fn reflect_mut(&mut self) -> ReflectMut<'_> {
        ReflectMut::Struct(self)
    }

    #[inline]
    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Struct(self)
    }

    fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
        struct_partial_eq(self, value)
    }

    fn debug(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "DynamicStruct(")?;
        struct_debug(self, f)?;
        write!(f, ")")
    }

    #[inline]
    fn is_dynamic(&self) -> bool {
        true
    }
}

impl_type_path!((in bevy_reflect) DynamicStruct);

impl Debug for DynamicStruct {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        self.debug(f)
    }
}

impl<'a, N> FromIterator<(N, Box<dyn PartialReflect>)> for DynamicStruct
where
    N: Into<Cow<'a, str>>,
{
    /// Create a dynamic struct that doesn't represent a type from the
    /// field name, field value pairs.
    fn from_iter<I: IntoIterator<Item = (N, Box<dyn PartialReflect>)>>(fields: I) -> Self {
        let mut dynamic_struct = Self::default();
        for (name, value) in fields.into_iter() {
            dynamic_struct.insert_boxed(name, value);
        }
        dynamic_struct
    }
}

impl IntoIterator for DynamicStruct {
    type Item = Box<dyn PartialReflect>;
    type IntoIter = alloc::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.fields.into_iter()
    }
}

impl<'a> IntoIterator for &'a DynamicStruct {
    type Item = &'a dyn PartialReflect;
    type IntoIter = FieldIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_fields()
    }
}

/// Compares a [`Struct`] with a [`PartialReflect`] value.
///
/// Returns true if and only if all of the following are true:
/// - `b` is a struct;
/// - For each field in `a`, `b` contains a field with the same name and
///   [`PartialReflect::reflect_partial_eq`] returns `Some(true)` for the two field
///   values.
///
/// Returns [`None`] if the comparison couldn't even be performed.
#[inline]
pub fn struct_partial_eq<S: Struct + ?Sized>(a: &S, b: &dyn PartialReflect) -> Option<bool> {
    let ReflectRef::Struct(struct_value) = b.reflect_ref() else {
        return Some(false);
    };

    if a.field_len() != struct_value.field_len() {
        return Some(false);
    }

    for (i, value) in struct_value.iter_fields().enumerate() {
        let name = struct_value.name_at(i).unwrap();
        if let Some(field_value) = a.field(name) {
            let eq_result = field_value.reflect_partial_eq(value);
            if let failed @ (Some(false) | None) = eq_result {
                return failed;
            }
        } else {
            return Some(false);
        }
    }

    Some(true)
}

/// The default debug formatter for [`Struct`] types.
///
/// # Example
/// ```
/// use bevy_reflect::Reflect;
/// #[derive(Reflect)]
/// struct MyStruct {
///   foo: usize
/// }
///
/// let my_struct: &dyn Reflect = &MyStruct { foo: 123 };
/// println!("{:#?}", my_struct);
///
/// // Output:
///
/// // MyStruct {
/// //   foo: 123,
/// // }
/// ```
#[inline]
pub fn struct_debug(dyn_struct: &dyn Struct, f: &mut Formatter<'_>) -> core::fmt::Result {
    let mut debug = f.debug_struct(
        dyn_struct
            .get_represented_type_info()
            .map(TypeInfo::type_path)
            .unwrap_or("_"),
    );
    for field_index in 0..dyn_struct.field_len() {
        let field = dyn_struct.field_at(field_index).unwrap();
        debug.field(
            dyn_struct.name_at(field_index).unwrap(),
            &field as &dyn Debug,
        );
    }
    debug.finish()
}

#[cfg(test)]
mod tests {
    use crate::*;
    #[derive(Reflect, Default)]
    struct MyStruct {
        a: (),
        b: (),
        c: (),
    }
    #[test]
    fn next_index_increment() {
        let my_struct = MyStruct::default();
        let mut iter = my_struct.iter_fields();
        iter.index = iter.len() - 1;
        let prev_index = iter.index;
        assert!(iter.next().is_some());
        assert_eq!(prev_index, iter.index - 1);

        // When None we should no longer increase index
        let prev_index = iter.index;
        assert!(iter.next().is_none());
        assert_eq!(prev_index, iter.index);
        assert!(iter.next().is_none());
        assert_eq!(prev_index, iter.index);
    }
}
