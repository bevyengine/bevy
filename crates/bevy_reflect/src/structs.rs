//! Traits and types used to power [struct-like] operations via reflection.
//!
//! [struct-like]: https://doc.rust-lang.org/book/ch05-01-defining-structs.html
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
/// use bevy_reflect::{PartialReflect, Reflect, structs::Struct};
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
    /// Gets a reference to the value of the field named `name` as a `&dyn
    /// PartialReflect`.
    fn field(&self, name: &str) -> Option<&dyn PartialReflect>;

    /// Gets a mutable reference to the value of the field named `name` as a
    /// `&mut dyn PartialReflect`.
    fn field_mut(&mut self, name: &str) -> Option<&mut dyn PartialReflect>;

    /// Gets a reference to the value of the field with index `index` as a
    /// `&dyn PartialReflect`.
    fn field_at(&self, index: usize) -> Option<&dyn PartialReflect>;

    /// Gets a mutable reference to the value of the field with index `index`
    /// as a `&mut dyn PartialReflect`.
    fn field_at_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect>;

    /// Gets the name of the field with index `index`.
    fn name_at(&self, index: usize) -> Option<&str>;

    /// Gets the name of the field, if it exists.
    fn name_of(&self, field: &dyn PartialReflect) -> Option<&str>;

    /// Gets the index of the field
    fn index_of(&self, value: &dyn PartialReflect) -> Option<usize>;

    /// Gets the index of the field with the given name.
    fn index_of_name(&self, name: &str) -> Option<usize>;

    /// Returns the number of fields in the struct.
    fn field_len(&self) -> usize;

    /// Returns an iterator over the values of the reflectable fields for this struct.
    fn iter_fields(&self) -> FieldIter<'_>;

    /// Creates a new [`DynamicStruct`] from this struct.
    fn to_dynamic_struct(&self) -> DynamicStruct {
        let mut dynamic_struct = DynamicStruct::default();
        dynamic_struct.set_represented_type(self.get_represented_type_info());
        for (name, value) in self.iter_fields() {
            dynamic_struct.insert_boxed(name, value.to_dynamic());
        }
        dynamic_struct
    }

    /// Will return `None` if [`TypeInfo`] is not available.
    fn get_represented_struct_info(&self) -> Option<&'static StructInfo> {
        self.get_represented_type_info()?.as_struct().ok()
    }
}

impl<'a> IntoIterator for &'a dyn Struct {
    type Item = (&'a str, &'a dyn PartialReflect);
    type IntoIter = FieldIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_fields()
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
    #[cfg(feature = "reflect_documentation")]
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
            #[cfg(feature = "reflect_documentation")]
            docs: None,
        }
    }

    /// Sets the docstring for this struct.
    #[cfg(feature = "reflect_documentation")]
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
    #[cfg(feature = "reflect_documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }

    impl_custom_attribute_methods!(self.custom_attributes, "struct");

    impl_generic_info_methods!(generics);
}

/// An iterator over the names and fields of a struct.
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
    type Item = (&'a str, &'a dyn PartialReflect);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(name) = self.struct_val.name_at(self.index)
            && let Some(field) = self.struct_val.field_at(self.index)
        {
            self.index += 1;
            Some((name, field))
        } else {
            None
        }
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
/// use bevy_reflect::{structs::GetField, Reflect};
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
    /// Gets a reference to the value of the field named `name`, downcast to
    /// `T`.
    fn get_field<T: Reflect>(&self, name: &str) -> Option<&T>;

    /// Gets a mutable reference to the value of the field named `name`,
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
        field: Box<dyn PartialReflect>,
    ) {
        let name: Cow<str> = name.into();
        if let Some(index) = self.field_indices.get(&name) {
            self.fields[*index] = field;
        } else {
            self.fields.push(field);
            self.field_indices
                .insert(Cow::Owned(name.clone().into_owned()), self.fields.len() - 1);
            self.field_names.push(Cow::Owned(name.into_owned()));
        }
    }

    /// Inserts a field named `name` with the typed value `value` into the struct.
    ///
    /// If the field already exists, it is overwritten.
    pub fn insert<'a, T: PartialReflect>(&mut self, name: impl Into<Cow<'a, str>>, field: T) {
        self.insert_boxed(name, Box::new(field));
    }

    /// Removes a field at `index`.
    pub fn remove_at(
        &mut self,
        index: usize,
    ) -> Option<(Cow<'static, str>, Box<dyn PartialReflect>)> {
        let mut i: usize = 0;
        #[expect(
            clippy::incompatible_msrv,
            reason = "MSRV is 1.85 and `extract_if` is Stable in 1.87"
        )]
        let mut extract = self.field_names.extract_if(0..self.field_names.len(), |n| {
            let mut result = false;
            if i == index {
                self.field_indices
                    .remove(n)
                    .expect("Invalid name for `field_indices.remove(name)`");
                result = true;
            } else if i > index {
                *self
                    .field_indices
                    .get_mut(n)
                    .expect("Invalid name for `field_indices.get_mut(name)`") -= 1;
            }
            i += 1;
            result
        });

        let name = extract
            .nth(0)
            .expect("Invalid index for `extract.nth(index)`");
        extract.for_each(drop); // Fully evaluate the rest of the iterator, so we don't short-circuit the extract.

        Some((name, self.fields.remove(index)))
    }

    /// Removes the first field that satisfies the given predicate, `f`.
    pub fn remove_if<F>(&mut self, mut f: F) -> Option<(Cow<'static, str>, Box<dyn PartialReflect>)>
    where
        F: FnMut((&str, &dyn PartialReflect)) -> bool,
    {
        if let Some(index) = self
            .field_names
            .iter()
            .zip(self.fields.iter())
            .position(|(name, field)| f((name.as_ref(), field.as_ref())))
        {
            self.remove_at(index)
        } else {
            None
        }
    }

    /// Removes a field by `name`.
    pub fn remove_by_name(
        &mut self,
        name: &str,
    ) -> Option<(Cow<'static, str>, Box<dyn PartialReflect>)> {
        if let Some(index) = self.index_of_name(name) {
            self.remove_at(index)
        } else {
            None
        }
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
        if let Some(index) = self.index_of_name(name) {
            Some(self.fields[index].as_mut())
        } else {
            None
        }
    }

    #[inline]
    fn field_at(&self, index: usize) -> Option<&dyn PartialReflect> {
        self.fields.get(index).map(AsRef::as_ref)
    }

    #[inline]
    fn field_at_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        self.fields.get_mut(index).map(AsMut::as_mut)
    }

    #[inline]
    fn name_at(&self, index: usize) -> Option<&str> {
        self.field_names.get(index).map(AsRef::as_ref)
    }

    #[inline]
    fn name_of(&self, field: &dyn PartialReflect) -> Option<&str> {
        if let Some(index) = self.index_of(field) {
            self.name_at(index)
        } else {
            None
        }
    }

    // Gets the index of the field.
    #[inline]
    fn index_of(&self, field: &dyn PartialReflect) -> Option<usize> {
        self.fields.iter().position(|v| core::ptr::eq(&**v, field))
    }

    /// Gets the index of the field with the given name.
    #[inline]
    fn index_of_name(&self, name: &str) -> Option<usize> {
        self.field_indices.get(name).copied()
    }

    #[inline]
    fn field_len(&self) -> usize {
        self.fields.len()
    }

    #[inline]
    fn iter_fields(&self) -> FieldIter<'_> {
        FieldIter::new(self)
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

        for (name, value) in struct_value {
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

    fn reflect_partial_cmp(&self, value: &dyn PartialReflect) -> Option<::core::cmp::Ordering> {
        struct_partial_cmp(self, value)
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
    type Item = (Cow<'static, str>, Box<dyn PartialReflect>);
    type IntoIter = core::iter::Zip<
        alloc::vec::IntoIter<Cow<'static, str>>,
        alloc::vec::IntoIter<Box<dyn PartialReflect>>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.field_names.into_iter().zip(self.fields)
    }
}

impl<'a> IntoIterator for &'a DynamicStruct {
    type Item = (&'a str, &'a dyn PartialReflect);
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
#[inline(never)]
pub fn struct_partial_eq(a: &dyn Struct, b: &dyn PartialReflect) -> Option<bool> {
    let ReflectRef::Struct(struct_value) = b.reflect_ref() else {
        return Some(false);
    };

    if a.field_len() != struct_value.field_len() {
        return Some(false);
    }

    for (name, value) in struct_value {
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

/// Lexicographically compares two [`Struct`] values and returns their ordering.
///
/// Returns [`None`] if the comparison couldn't be performed (e.g., kinds mismatch
/// or an element comparison returns `None`).
#[inline(never)]
pub fn struct_partial_cmp(a: &dyn Struct, b: &dyn PartialReflect) -> Option<::core::cmp::Ordering> {
    let ReflectRef::Struct(struct_value) = b.reflect_ref() else {
        return None;
    };

    if a.field_len() != struct_value.field_len() {
        return None;
    }

    // Delegate detailed field-name-aware comparison to shared helper
    partial_cmp_by_field_names(
        a.field_len(),
        |i| a.name_at(i),
        |i| a.field_at(i),
        |i| struct_value.name_at(i),
        |i| struct_value.field_at(i),
        |name| struct_value.field(name),
    )
}

/// Compare two sets of named fields. `field_len` should be equal.
///
/// Tries best to:
/// 1. when used on concrete types of actually same type, should be compatible
///    with derived `PartialOrd` implementations.
/// 2. compatible with `reflect_partial_eq`: when `reflect_partial_eq(a, b) = Some(true)`,
///    then `partial_cmp(a, b) = Some(Ordering::Equal)`.
/// 3. when used on dynamic types, provide a consistent ordering:
///    see example `crate::tests:reflect_partial_cmp_struct_named_field_reorder`
pub(crate) fn partial_cmp_by_field_names<'a, NA, FA, NB, FB, FBY>(
    field_len: usize,
    name_at_a: NA,
    field_at_a: FA,
    name_at_b: NB,
    field_at_b_index: FB,
    field_b_by_name: FBY,
) -> Option<::core::cmp::Ordering>
where
    NA: Fn(usize) -> Option<&'a str>,
    FA: Fn(usize) -> Option<&'a dyn PartialReflect>,
    NB: Fn(usize) -> Option<&'a str>,
    FB: Fn(usize) -> Option<&'a dyn PartialReflect>,
    FBY: Fn(&str) -> Option<&'a dyn PartialReflect>,
{
    use ::core::cmp::Ordering;

    let mut same_field_order = true;
    for i in 0..field_len {
        if name_at_a(i) != name_at_b(i) {
            same_field_order = false;
            break;
        }
    }

    if same_field_order {
        for i in 0..field_len {
            let a_val = field_at_a(i).unwrap();
            let b_val = field_at_b_index(i).unwrap();
            match a_val.reflect_partial_cmp(b_val) {
                None => return None,
                Some(Ordering::Equal) => continue,
                Some(ord) => return Some(ord),
            }
        }
        return Some(Ordering::Equal);
    }

    let mut all_less_equal = true;
    let mut all_greater_equal = true;
    let mut all_equal = true;

    for i in 0..field_len {
        let field_name = name_at_a(i).unwrap();
        let a_val = field_at_a(i).unwrap();
        let b_val = field_b_by_name(field_name)?;
        match a_val.reflect_partial_cmp(b_val) {
            None => return None,
            Some(::core::cmp::Ordering::Less) => {
                all_greater_equal = false;
                all_equal = false;
            }
            Some(::core::cmp::Ordering::Greater) => {
                all_less_equal = false;
                all_equal = false;
            }
            Some(::core::cmp::Ordering::Equal) => {}
        }
    }

    if all_equal {
        Some(::core::cmp::Ordering::Equal)
    } else if all_less_equal {
        Some(::core::cmp::Ordering::Less)
    } else if all_greater_equal {
        Some(::core::cmp::Ordering::Greater)
    } else {
        None
    }
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
    use crate::{structs::*, *};
    use alloc::borrow::ToOwned;

    #[derive(Reflect, Default)]
    struct MyStruct {
        a: (),
        b: (),
        c: (),
    }

    #[test]
    fn dynamic_struct_remove_at() {
        let mut my_struct = MyStruct::default().to_dynamic_struct();

        assert_eq!(my_struct.field_len(), 3);

        let field_2 = my_struct
            .remove_at(1)
            .expect("Invalid index for `my_struct.remove_at(index)`");

        assert_eq!(my_struct.field_len(), 2);
        assert_eq!(field_2.0, "b");

        let field_3 = my_struct
            .remove_at(0)
            .expect("Invalid index for `my_struct.remove_at(index)`");

        assert_eq!(my_struct.field_len(), 1);
        assert_eq!(field_3.0, "a");

        let field_1 = my_struct
            .remove_at(0)
            .expect("Invalid index for `my_struct.remove_at(index)`");

        assert_eq!(my_struct.field_len(), 0);
        assert_eq!(field_1.0, "c");
    }

    #[test]
    fn dynamic_struct_remove_by_name() {
        let mut my_struct = MyStruct::default().to_dynamic_struct();

        assert_eq!(my_struct.field_len(), 3);

        let field_3 = my_struct
            .remove_by_name("b")
            .expect("Invalid name for `my_struct.remove_by_name(name)`");

        assert_eq!(my_struct.field_len(), 2);
        assert_eq!(field_3.0, "b");

        let field_2 = my_struct
            .remove_by_name("c")
            .expect("Invalid name for `my_struct.remove_by_name(name)`");

        assert_eq!(my_struct.field_len(), 1);
        assert_eq!(field_2.0, "c");

        let field_1 = my_struct
            .remove_by_name("a")
            .expect("Invalid name for `my_struct.remove_by_name(name)`");

        assert_eq!(my_struct.field_len(), 0);
        assert_eq!(field_1.0, "a");
    }

    #[test]
    fn dynamic_struct_remove_if() {
        let mut my_struct = MyStruct::default().to_dynamic_struct();

        assert_eq!(my_struct.field_len(), 3);

        let field_3_name = my_struct
            .name_of(
                my_struct
                    .field_at(2)
                    .expect("Invalid index for `my_struct.field_at(index)`"),
            )
            .expect("Invalid field for `my_struct.name_of(field)")
            .to_owned();
        let field_3 = my_struct
            .remove_if(|(name, _field)| name == field_3_name)
            .expect("No valid name/field found for `my_struct.remove_with(|(name, field)|{})");

        assert_eq!(my_struct.field_len(), 2);
        assert_eq!(field_3.0, "c");
    }

    #[test]
    fn dynamic_struct_remove_combo() {
        let mut my_struct = MyStruct::default().to_dynamic_struct();

        assert_eq!(my_struct.field_len(), 3);

        let field_2 = my_struct
            .remove_at(
                my_struct
                    .index_of(
                        my_struct
                            .field("b")
                            .expect("Invalid name for `my_struct.field(name)`"),
                    )
                    .expect("Invalid field for `my_struct.index_of(field)`"),
            )
            .expect("Invalid index for `my_struct.remove_at(index)`");

        assert_eq!(my_struct.field_len(), 2);
        assert_eq!(field_2.0, "b");

        let field_3_name = my_struct
            .name_of(
                my_struct
                    .field_at(1)
                    .expect("Invalid index for `my_struct.field_at(index)`"),
            )
            .expect("Invalid field for `my_struct.name_of(field)`")
            .to_owned();
        let field_3 = my_struct
            .remove_by_name(field_3_name.as_ref())
            .expect("Invalid name for `my_struct.remove_by_name(name)`");

        assert_eq!(my_struct.field_len(), 1);
        assert_eq!(field_3.0, "c");

        let field_1_name = my_struct
            .name_at(0)
            .expect("Invalid name for `my_struct.name_at(name)`")
            .to_owned();
        let field_1 = my_struct
            .remove_if(|(name, _field)| name == field_1_name)
            .expect("No valid name/field found for `my_struct.remove_with(|(name, field)|{})`");

        assert_eq!(my_struct.field_len(), 0);
        assert_eq!(field_1.0, "a");
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
