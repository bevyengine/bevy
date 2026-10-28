//! Traits and types used to power [struct-like] operations via reflection.
//!
//! [struct-like]: https://doc.rust-lang.org/book/ch05-01-defining-structs.html
use crate::dynamic::{DynamicStruct, DynamicValue, DynamicValueRef};
use crate::generics::impl_generic_info_methods;
use crate::TypeInfo;
use crate::{
    attributes::{impl_custom_attribute_methods, CustomAttributes},
    ty::impl_type_methods,
    Generics, NamedField, Reflect, ReflectRef, Type, TypePath,
};
use alloc::boxed::Box;
use bevy_platform::collections::HashMap;
use core::cmp::Ordering;
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
/// use bevy_reflect::{Reflect, structs::Struct};
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
/// let field: &dyn Reflect = foo.field("bar").unwrap();
/// assert_eq!(field.downcast_ref::<u32>(), Some(&123));
/// ```
///
/// [struct-like]: https://doc.rust-lang.org/book/ch05-01-defining-structs.html
/// [reflection]: crate
/// [unit structs]: https://doc.rust-lang.org/book/ch05-01-defining-structs.html#unit-like-structs-without-any-fields
pub trait Struct: Reflect {
    /// Gets a reference to the value of the field named `name` as a `&dyn
    /// Reflect`.
    fn field(&self, name: &str) -> Option<&dyn Reflect>;

    /// Gets a mutable reference to the value of the field named `name` as a
    /// `&mut dyn Reflect`.
    fn field_mut(&mut self, name: &str) -> Option<&mut dyn Reflect>;

    /// Gets a reference to the value of the field with index `index` as a
    /// `&dyn Reflect`.
    fn field_at(&self, index: usize) -> Option<&dyn Reflect>;

    /// Gets a mutable reference to the value of the field with index `index`
    /// as a `&mut dyn Reflect`.
    fn field_at_mut(&mut self, index: usize) -> Option<&mut dyn Reflect>;

    /// Gets the name of the field with index `index`.
    fn name_at(&self, index: usize) -> Option<&str>;

    /// Gets the index of the field with the given name.
    fn index_of_name(&self, name: &str) -> Option<usize>;

    /// Returns the number of fields in the struct.
    fn field_len(&self) -> usize;

    /// Returns an iterator over the values of the reflectable fields for this struct.
    fn iter_fields(&self) -> FieldIter<'_>;

    /// Returns the [`StructInfo`] for this type.
    fn reflect_struct_info(&self) -> &'static StructInfo {
        self.reflect_type_info().as_struct().unwrap()
    }
}

impl<'a> IntoIterator for &'a dyn Struct {
    type Item = (&'a str, &'a dyn Reflect);
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
    custom_attributes: CustomAttributes,
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
        Self::from_erased(fields, Type::of::<T>())
    }

    // Inlining is disabled because this function is called many times by cold
    // functions inside generated code.
    #[inline(never)]
    fn from_erased(fields: &[NamedField], ty: Type) -> Self {
        let field_indices = fields
            .iter()
            .enumerate()
            .map(|(index, field)| (field.name(), index))
            .collect::<HashMap<_, _>>();

        let field_names = fields.iter().map(NamedField::name).collect();

        Self {
            ty,
            generics: Generics::new(),
            fields: fields.to_vec().into_boxed_slice(),
            field_names,
            field_indices,
            custom_attributes: CustomAttributes::default(),
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
            custom_attributes,
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
    type Item = (&'a str, &'a dyn Reflect);

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
        let remaining = self.struct_val.field_len().saturating_sub(self.index);
        (remaining, Some(remaining))
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

pub(crate) trait StructLike {
    fn field(&self, name: &str) -> Option<DynamicValueRef<'_>>;

    fn field_at(&self, index: usize) -> Option<DynamicValueRef<'_>>;

    fn name_at(&self, index: usize) -> Option<&str>;

    fn field_len(&self) -> usize;

    fn iter_fields(&self) -> impl Iterator<Item = (&str, DynamicValueRef<'_>)>;

    fn get_represented_type_info(&self) -> Option<&'static TypeInfo>;
}

impl StructLike for DynamicStruct {
    fn field(&self, name: &str) -> Option<DynamicValueRef<'_>> {
        self.field(name).map(DynamicValue::as_ref)
    }

    fn field_at(&self, index: usize) -> Option<DynamicValueRef<'_>> {
        self.field_at(index).map(DynamicValue::as_ref)
    }

    fn name_at(&self, index: usize) -> Option<&str> {
        self.name_at(index)
    }

    fn field_len(&self) -> usize {
        self.field_len()
    }

    fn iter_fields(&self) -> impl Iterator<Item = (&str, DynamicValueRef<'_>)> {
        self.iter_fields()
            .map(|(name, value)| (name, value.as_ref()))
    }

    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        self.get_represented_type_info()
    }
}

impl StructLike for dyn Struct {
    fn field(&self, name: &str) -> Option<DynamicValueRef<'_>> {
        self.field(name).map(|value| DynamicValueRef::from(value))
    }

    fn field_at(&self, index: usize) -> Option<DynamicValueRef<'_>> {
        self.field_at(index)
            .map(|value| DynamicValueRef::from(value))
    }

    fn name_at(&self, index: usize) -> Option<&str> {
        self.name_at(index)
    }

    fn field_len(&self) -> usize {
        self.field_len()
    }

    fn iter_fields(&self) -> impl Iterator<Item = (&str, DynamicValueRef<'_>)> {
        self.iter_fields()
            .map(|(name, value)| (name, DynamicValueRef::from(value)))
    }

    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        Some(self.reflect_type_info())
    }
}

/// Compares a [`Struct`] with a [`Reflect`] value.
///
/// Returns true if and only if all of the following are true:
/// - `b` is a struct;
/// - For each field in `a`, `b` contains a field with the same name and
///   [`Reflect::reflect_partial_eq`] returns `Some(true)` for the two field
///   values.
///
/// Returns [`None`] if the comparison couldn't even be performed.
#[inline(never)]
pub fn struct_partial_eq(a: &dyn Struct, b: &dyn Reflect) -> Option<bool> {
    let ReflectRef::Struct(b) = b.reflect_ref() else {
        return Some(false);
    };

    struct_like_partial_eq(a, b)
}

/// Same as above, but comparing two [`StructLike`] values.
pub(crate) fn struct_like_partial_eq<L: StructLike + ?Sized, R: StructLike + ?Sized>(
    a: &L,
    b: &R,
) -> Option<bool> {
    if a.field_len() != b.field_len() {
        return Some(false);
    }

    for (name, value) in b.iter_fields() {
        if let Some(field_value) = a.field(name) {
            let eq_result = field_value.dynamic_partial_eq(value);
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
pub fn struct_partial_cmp(a: &dyn Struct, b: &dyn Reflect) -> Option<Ordering> {
    let ReflectRef::Struct(b) = b.reflect_ref() else {
        return None;
    };

    struct_like_partial_cmp(a, b)
}

pub(crate) fn struct_like_partial_cmp<L: StructLike + ?Sized, R: StructLike + ?Sized>(
    a: &L,
    b: &R,
) -> Option<Ordering> {
    if a.field_len() != b.field_len() {
        return None;
    }

    // Delegate detailed field-name-aware comparison to shared helper
    partial_cmp_by_field_names(
        a.field_len(),
        |i| a.name_at(i),
        |i| a.field_at(i),
        |i| b.name_at(i),
        |i| b.field_at(i),
        |name| b.field(name),
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
    FA: Fn(usize) -> Option<DynamicValueRef<'a>>,
    NB: Fn(usize) -> Option<&'a str>,
    FB: Fn(usize) -> Option<DynamicValueRef<'a>>,
    FBY: Fn(&str) -> Option<DynamicValueRef<'a>>,
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
            match a_val.dynamic_partial_cmp(b_val) {
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
        match a_val.dynamic_partial_cmp(b_val) {
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
    let mut debug = f.debug_struct(dyn_struct.reflect_type_info().type_path());
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
