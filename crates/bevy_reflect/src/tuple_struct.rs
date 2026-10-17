//! Traits and types used to power [tuple-struct-like] operations via reflection.
//!
//! [tuple-struct-like]: https://doc.rust-lang.org/book/ch05-01-defining-structs.html#using-tuple-structs-without-named-fields-to-create-different-types
use crate::dynamic::{DynamicTupleStruct, DynamicValue, DynamicValueRef};
use crate::generics::impl_generic_info_methods;
use crate::{
    attributes::{impl_custom_attribute_methods, CustomAttributes},
    ty::impl_type_methods,
    Generics, PartialReflect, Reflect, ReflectRef, Type, TypeInfo, TypePath, UnnamedField,
};
use alloc::boxed::Box;
use core::cmp::Ordering;
use core::{
    fmt::{Debug, Formatter},
    slice::Iter,
};

/// A trait used to power [tuple struct-like] operations via [reflection].
///
/// This trait uses the [`Reflect`] trait to allow implementors to have their fields
/// be dynamically addressed by index.
///
/// When using [`#[derive(Reflect)]`](derive@crate::Reflect) on a tuple struct,
/// this trait will be automatically implemented.
///
/// # Example
///
/// ```
/// use bevy_reflect::{PartialReflect, Reflect, tuple_struct::TupleStruct};
///
/// #[derive(Reflect)]
/// struct Foo(u32);
///
/// let foo = Foo(123);
///
/// assert_eq!(foo.field_len(), 1);
///
/// let field: &dyn PartialReflect = foo.field(0).unwrap();
/// assert_eq!(field.try_downcast_ref::<u32>(), Some(&123));
/// ```
///
/// [tuple struct-like]: https://doc.rust-lang.org/book/ch05-01-defining-structs.html#using-tuple-structs-without-named-fields-to-create-different-types
/// [reflection]: crate
pub trait TupleStruct: PartialReflect {
    /// Returns a reference to the value of the field with index `index` as a
    /// `&dyn Reflect`.
    fn field(&self, index: usize) -> Option<&dyn PartialReflect>;

    /// Returns a mutable reference to the value of the field with index `index`
    /// as a `&mut dyn Reflect`.
    fn field_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect>;

    /// Returns the number of fields in the tuple struct.
    fn field_len(&self) -> usize;

    /// Returns an iterator over the values of the tuple struct's fields.
    fn iter_fields(&self) -> TupleStructFieldIter<'_>;

    /// Will return `None` if [`TypeInfo`] is not available.
    fn get_represented_tuple_struct_info(&self) -> Option<&'static TupleStructInfo> {
        self.get_represented_type_info()?.as_tuple_struct().ok()
    }
}

/// A container for compile-time tuple struct info.
#[derive(Clone, Debug)]
pub struct TupleStructInfo {
    ty: Type,
    generics: Generics,
    fields: Box<[UnnamedField]>,
    custom_attributes: CustomAttributes,
    #[cfg(feature = "reflect_documentation")]
    docs: Option<&'static str>,
}

impl TupleStructInfo {
    /// Create a new [`TupleStructInfo`].
    ///
    /// # Arguments
    ///
    /// * `fields`: The fields of this struct in the order they are defined
    pub fn new<T: Reflect + TypePath>(fields: &[UnnamedField]) -> Self {
        Self::from_erased(fields, Type::of::<T>())
    }

    // Inlining is disabled because this function is called many times by cold
    // functions inside generated code.
    #[inline(never)]
    fn from_erased(fields: &[UnnamedField], ty: Type) -> Self {
        Self {
            ty,
            generics: Generics::new(),
            fields: fields.to_vec().into_boxed_slice(),
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

    /// Get the field at the given index.
    pub fn field_at(&self, index: usize) -> Option<&UnnamedField> {
        self.fields.get(index)
    }

    /// Iterate over the fields of this struct.
    pub fn iter(&self) -> Iter<'_, UnnamedField> {
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

/// An iterator over the field values of a tuple struct.
pub struct TupleStructFieldIter<'a> {
    pub(crate) tuple_struct: &'a dyn TupleStruct,
    pub(crate) index: usize,
}

impl<'a> TupleStructFieldIter<'a> {
    /// Creates a new [`TupleStructFieldIter`].
    pub fn new(value: &'a dyn TupleStruct) -> Self {
        TupleStructFieldIter {
            tuple_struct: value,
            index: 0,
        }
    }
}

impl<'a> Iterator for TupleStructFieldIter<'a> {
    type Item = &'a dyn PartialReflect;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.tuple_struct.field(self.index);
        self.index += value.is_some() as usize;
        value
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.tuple_struct.field_len().saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for TupleStructFieldIter<'a> {}

/// A convenience trait which combines fetching and downcasting of tuple
/// struct fields.
///
/// # Example
///
/// ```
/// use bevy_reflect::{tuple_struct::GetTupleStructField, Reflect};
///
/// #[derive(Reflect)]
/// struct Foo(String);
///
/// # fn main() {
/// let mut foo = Foo("Hello, world!".to_string());
///
/// foo.get_field_mut::<String>(0).unwrap().truncate(5);
/// assert_eq!(foo.get_field::<String>(0), Some(&"Hello".to_string()));
/// # }
/// ```
pub trait GetTupleStructField {
    /// Returns a reference to the value of the field with index `index`,
    /// downcast to `T`.
    fn get_field<T: Reflect>(&self, index: usize) -> Option<&T>;

    /// Returns a mutable reference to the value of the field with index
    /// `index`, downcast to `T`.
    fn get_field_mut<T: Reflect>(&mut self, index: usize) -> Option<&mut T>;
}

impl<S: TupleStruct> GetTupleStructField for S {
    fn get_field<T: Reflect>(&self, index: usize) -> Option<&T> {
        self.field(index)
            .and_then(|value| value.try_downcast_ref::<T>())
    }

    fn get_field_mut<T: Reflect>(&mut self, index: usize) -> Option<&mut T> {
        self.field_mut(index)
            .and_then(|value| value.try_downcast_mut::<T>())
    }
}

impl GetTupleStructField for dyn TupleStruct {
    fn get_field<T: Reflect>(&self, index: usize) -> Option<&T> {
        self.field(index)
            .and_then(|value| value.try_downcast_ref::<T>())
    }

    fn get_field_mut<T: Reflect>(&mut self, index: usize) -> Option<&mut T> {
        self.field_mut(index)
            .and_then(|value| value.try_downcast_mut::<T>())
    }
}

pub(crate) trait TupleStructLike {
    fn field(&self, index: usize) -> Option<DynamicValueRef<'_>>;

    fn field_len(&self) -> usize;

    fn iter_fields(&self) -> impl Iterator<Item = DynamicValueRef<'_>>;
}

impl TupleStructLike for DynamicTupleStruct {
    fn field(&self, index: usize) -> Option<DynamicValueRef<'_>> {
        self.field(index).map(DynamicValue::as_ref)
    }

    fn field_len(&self) -> usize {
        self.field_len()
    }

    fn iter_fields(&self) -> impl Iterator<Item = DynamicValueRef<'_>> {
        self.iter_fields().map(DynamicValue::as_ref)
    }
}

impl TupleStructLike for dyn TupleStruct {
    fn field(&self, index: usize) -> Option<DynamicValueRef<'_>> {
        // TODO: Don't unwrap here.
        self.field(index)
            .map(|value| DynamicValueRef::from(value.try_as_reflect().unwrap()))
    }

    fn field_len(&self) -> usize {
        self.field_len()
    }

    fn iter_fields(&self) -> impl Iterator<Item = DynamicValueRef<'_>> {
        self.iter_fields()
            .map(|value| DynamicValueRef::from(value.try_as_reflect().unwrap()))
    }
}

/// Compares a [`TupleStruct`] with a [`PartialReflect`] value.
///
/// Returns true if and only if all of the following are true:
/// - `b` is a tuple struct;
/// - `b` has the same number of fields as `a`;
/// - [`PartialReflect::reflect_partial_eq`] returns `Some(true)` for pairwise fields of `a` and `b`.
///
/// Returns [`None`] if the comparison couldn't even be performed.
#[inline(never)]
pub fn tuple_struct_partial_eq(a: &dyn TupleStruct, b: &dyn PartialReflect) -> Option<bool> {
    let ReflectRef::TupleStruct(b) = b.reflect_ref() else {
        return Some(false);
    };

    tuple_struct_like_partial_eq(a, b)
}

/// Same as above, but comparing two [`TupleStructLike`] values.
pub(crate) fn tuple_struct_like_partial_eq<
    L: TupleStructLike + ?Sized,
    R: TupleStructLike + ?Sized,
>(
    a: &L,
    b: &R,
) -> Option<bool> {
    if a.field_len() != b.field_len() {
        return Some(false);
    }

    for (i, value) in b.iter_fields().enumerate() {
        if let Some(field_value) = a.field(i) {
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
/// Lexicographically compares two [`TupleStruct`] values and returns their ordering.
///
/// Returns [`None`] if the comparison couldn't be performed (e.g., kinds mismatch
/// or an element comparison returns `None`).
#[inline(never)]
pub fn tuple_struct_partial_cmp(a: &dyn TupleStruct, b: &dyn PartialReflect) -> Option<Ordering> {
    let ReflectRef::TupleStruct(b) = b.reflect_ref() else {
        return None;
    };
    tuple_struct_like_partial_cmp(a, b)
}

/// Same as above, but for any two [`TupleStructLike`] types.
pub(crate) fn tuple_struct_like_partial_cmp<
    L: TupleStructLike + ?Sized,
    R: TupleStructLike + ?Sized,
>(
    a: &L,
    b: &R,
) -> Option<Ordering> {
    if a.field_len() != b.field_len() {
        return None;
    }

    for (i, value) in b.iter_fields().enumerate() {
        if let Some(field_value) = a.field(i) {
            match field_value.dynamic_partial_cmp(value) {
                None => return None,
                Some(Ordering::Equal) => continue,
                Some(ord) => return Some(ord),
            }
        }
        return None;
    }

    Some(Ordering::Equal)
}

/// The default debug formatter for [`TupleStruct`] types.
///
/// # Example
/// ```
/// use bevy_reflect::Reflect;
/// #[derive(Reflect)]
/// struct MyTupleStruct(usize);
///
/// let my_tuple_struct: &dyn Reflect = &MyTupleStruct(123);
/// println!("{:#?}", my_tuple_struct);
///
/// // Output:
///
/// // MyTupleStruct (
/// //   123,
/// // )
/// ```
#[inline]
pub fn tuple_struct_debug(
    dyn_tuple_struct: &dyn TupleStruct,
    f: &mut Formatter<'_>,
) -> core::fmt::Result {
    let mut debug = f.debug_tuple(
        dyn_tuple_struct
            .get_represented_type_info()
            .map(TypeInfo::type_path)
            .unwrap_or("_"),
    );
    for field in dyn_tuple_struct.iter_fields() {
        debug.field(&field as &dyn Debug);
    }
    debug.finish()
}

#[cfg(test)]
mod tests {
    use super::TupleStruct;
    use crate::Reflect;
    #[derive(Reflect)]
    struct Ts(u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8);
    #[test]
    fn next_index_increment() {
        let mut iter = Ts(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11).iter_fields();
        let size = iter.len();
        iter.index = size - 1;
        let prev_index = iter.index;
        assert!(iter.next().is_some());
        assert_eq!(prev_index, iter.index - 1);

        // When None we should no longer increase index
        assert!(iter.next().is_none());
        assert_eq!(size, iter.index);
        assert!(iter.next().is_none());
        assert_eq!(size, iter.index);
    }
}
