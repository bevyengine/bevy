use bevy_reflect_derive::impl_type_path;

use crate::attributes::{impl_custom_attribute_methods, CustomAttributes};
use crate::{
    self as bevy_reflect, ApplyError, DynamicTuple, Reflect, ReflectKind, ReflectMut, ReflectOwned,
    ReflectRef, Tuple, TypeInfo, TypePath, TypePathTable, UnnamedField,
};
use std::any::{Any, TypeId};
use std::fmt::{Debug, Formatter};
use std::slice::Iter;
use std::sync::Arc;

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
/// use bevy_reflect::{Reflect, TupleStruct};
///
/// #[derive(Reflect)]
/// struct Foo(u32);
///
/// let foo = Foo(123);
///
/// assert_eq!(foo.field_len(), 1);
///
/// let field: &dyn Reflect = foo.field(0).unwrap();
/// assert_eq!(field.downcast_ref::<u32>(), Some(&123));
/// ```
///
/// [tuple struct-like]: https://doc.rust-lang.org/book/ch05-01-defining-structs.html#using-tuple-structs-without-named-fields-to-create-different-types
/// [reflection]: crate
pub trait TupleStruct: Reflect {
    /// Returns a reference to the value of the field with index `index` as a
    /// `&dyn Reflect`.
    fn field(&self, index: usize) -> Option<&dyn Reflect>;

    /// Returns a mutable reference to the value of the field with index `index`
    /// as a `&mut dyn Reflect`.
    fn field_mut(&mut self, index: usize) -> Option<&mut dyn Reflect>;

    /// Returns the number of fields in the tuple struct.
    fn field_len(&self) -> usize;

    /// Returns an iterator over the values of the tuple struct's fields.
    fn iter_fields(&self) -> TupleStructFieldIter;

    /// Clones the struct into a [`DynamicTupleStruct`].
    fn clone_dynamic(&self) -> DynamicTupleStruct;
}

/// A container for compile-time tuple struct info.
#[derive(Clone, Debug)]
pub struct TupleStructInfo {
    type_path: TypePathTable,
    type_id: TypeId,
    fields: Box<[UnnamedField]>,
    custom_attributes: Arc<CustomAttributes>,
    #[cfg(feature = "documentation")]
    docs: Option<&'static str>,
}

impl TupleStructInfo {
    /// Create a new [`TupleStructInfo`].
    ///
    /// # Arguments
    ///
    /// * `fields`: The fields of this struct in the order they are defined
    ///
    pub fn new<T: Reflect + TypePath>(fields: &[UnnamedField]) -> Self {
        Self {
            type_path: TypePathTable::of::<T>(),
            type_id: TypeId::of::<T>(),
            fields: fields.to_vec().into_boxed_slice(),
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

    /// A representation of the type path of the struct.
    ///
    /// Provides dynamic access to all methods on [`TypePath`].
    pub fn type_path_table(&self) -> &TypePathTable {
        &self.type_path
    }

    /// The [stable, full type path] of the struct.
    ///
    /// Use [`type_path_table`] if you need access to the other methods on [`TypePath`].
    ///
    /// [stable, full type path]: TypePath
    /// [`type_path_table`]: Self::type_path_table
    pub fn type_path(&self) -> &'static str {
        self.type_path_table().path()
    }

    /// The [`TypeId`] of the tuple struct.
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Check if the given type matches the tuple struct type.
    pub fn is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }

    /// The docstring of this struct, if any.
    #[cfg(feature = "documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }

    impl_custom_attribute_methods!(self.custom_attributes, "struct");
}

/// An iterator over the field values of a tuple struct.
pub struct TupleStructFieldIter<'a> {
    pub(crate) tuple_struct: &'a dyn TupleStruct,
    pub(crate) index: usize,
}

impl<'a> TupleStructFieldIter<'a> {
    pub fn new(value: &'a dyn TupleStruct) -> Self {
        TupleStructFieldIter {
            tuple_struct: value,
            index: 0,
        }
    }
}

impl<'a> Iterator for TupleStructFieldIter<'a> {
    type Item = &'a dyn Reflect;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.tuple_struct.field(self.index);
        self.index += value.is_some() as usize;
        value
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.tuple_struct.field_len();
        (size, Some(size))
    }
}

impl<'a> ExactSizeIterator for TupleStructFieldIter<'a> {}

/// A convenience trait which combines fetching and downcasting of tuple
/// struct fields.
///
/// # Example
///
/// ```
/// use bevy_reflect::{GetTupleStructField, Reflect};
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
            .and_then(|value| value.downcast_ref::<T>())
    }

    fn get_field_mut<T: Reflect>(&mut self, index: usize) -> Option<&mut T> {
        self.field_mut(index)
            .and_then(|value| value.downcast_mut::<T>())
    }
}

impl GetTupleStructField for dyn TupleStruct {
    fn get_field<T: Reflect>(&self, index: usize) -> Option<&T> {
        self.field(index)
            .and_then(|value| value.downcast_ref::<T>())
    }

    fn get_field_mut<T: Reflect>(&mut self, index: usize) -> Option<&mut T> {
        self.field_mut(index)
            .and_then(|value| value.downcast_mut::<T>())
    }
}

/// A tuple struct which allows fields to be added at runtime.
#[derive(Default)]
pub struct DynamicTupleStruct {
    represented_type: Option<&'static TypeInfo>,
    fields: Vec<Box<dyn Reflect>>,
}

impl DynamicTupleStruct {
    /// Sets the [type] to be represented by this `DynamicTupleStruct`.
    ///
    /// # Panics
    ///
    /// Panics if the given [type] is not a [`TypeInfo::TupleStruct`].
    ///
    /// [type]: TypeInfo
    pub fn set_represented_type(&mut self, represented_type: Option<&'static TypeInfo>) {
        if let Some(represented_type) = represented_type {
            assert!(
                matches!(represented_type, TypeInfo::TupleStruct(_)),
                "expected TypeInfo::TupleStruct but received: {:?}",
                represented_type
            );
        }

        self.represented_type = represented_type;
    }

    /// Appends an element with value `value` to the tuple struct.
    pub fn insert_boxed(&mut self, value: Box<dyn Reflect>) {
        self.fields.push(value);
    }

    /// Appends a typed element with value `value` to the tuple struct.
    pub fn insert<T: Reflect>(&mut self, value: T) {
        self.insert_boxed(Box::new(value));
    }
}

impl TupleStruct for DynamicTupleStruct {
    #[inline]
    fn field(&self, index: usize) -> Option<&dyn Reflect> {
        self.fields.get(index).map(|field| &**field)
    }

    #[inline]
    fn field_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        self.fields.get_mut(index).map(|field| &mut **field)
    }

    #[inline]
    fn field_len(&self) -> usize {
        self.fields.len()
    }

    #[inline]
    fn iter_fields(&self) -> TupleStructFieldIter {
        TupleStructFieldIter {
            tuple_struct: self,
            index: 0,
        }
    }

    fn clone_dynamic(&self) -> DynamicTupleStruct {
        DynamicTupleStruct {
            represented_type: self.represented_type,
            fields: self
                .fields
                .iter()
                .map(|value| value.clone_value())
                .collect(),
        }
    }
}

impl Reflect for DynamicTupleStruct {
    #[inline]
    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        self.represented_type
    }

    #[inline]
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    #[inline]
    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
        self
    }

    #[inline]
    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    #[inline]
    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }

    fn try_apply(&mut self, value: &dyn Reflect) -> Result<(), ApplyError> {
        if let ReflectRef::TupleStruct(tuple_struct) = value.reflect_ref() {
            for (i, value) in tuple_struct.iter_fields().enumerate() {
                if let Some(v) = self.field_mut(i) {
                    v.try_apply(value)?;
                }
            }
        } else {
            return Err(ApplyError::MismatchedKinds {
                from_kind: value.reflect_kind(),
                to_kind: ReflectKind::TupleStruct,
            });
        }
        Ok(())
    }

    #[inline]
    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
    }

    #[inline]
    fn reflect_kind(&self) -> ReflectKind {
        ReflectKind::TupleStruct
    }

    #[inline]
    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::TupleStruct(self)
    }

    #[inline]
    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::TupleStruct(self)
    }

    #[inline]
    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::TupleStruct(self)
    }

    #[inline]
    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(self.clone_dynamic())
    }

    #[inline]
    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        tuple_struct_partial_eq(self, value)
    }

    fn debug(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "DynamicTupleStruct(")?;
        tuple_struct_debug(self, f)?;
        write!(f, ")")
    }

    #[inline]
    fn is_dynamic(&self) -> bool {
        true
    }
}

impl_type_path!((in bevy_reflect) DynamicTupleStruct);

impl Debug for DynamicTupleStruct {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.debug(f)
    }
}

impl From<DynamicTuple> for DynamicTupleStruct {
    fn from(value: DynamicTuple) -> Self {
        Self {
            represented_type: None,
            fields: Box::new(value).drain(),
        }
    }
}

/// Compares a [`TupleStruct`] with a [`Reflect`] value.
///
/// Returns true if and only if all of the following are true:
/// - `b` is a tuple struct;
/// - `b` has the same number of fields as `a`;
/// - [`Reflect::reflect_partial_eq`] returns `Some(true)` for pairwise fields of `a` and `b`.
///
/// Returns [`None`] if the comparison couldn't even be performed.
#[inline]
pub fn tuple_struct_partial_eq<S: TupleStruct>(a: &S, b: &dyn Reflect) -> Option<bool> {
    let ReflectRef::TupleStruct(tuple_struct) = b.reflect_ref() else {
        return Some(false);
    };

    if a.field_len() != tuple_struct.field_len() {
        return Some(false);
    }

    for (i, value) in tuple_struct.iter_fields().enumerate() {
        if let Some(field_value) = a.field(i) {
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
) -> std::fmt::Result {
    let mut debug = f.debug_tuple(
        dyn_tuple_struct
            .get_represented_type_info()
            .map(|s| s.type_path())
            .unwrap_or("_"),
    );
    for field in dyn_tuple_struct.iter_fields() {
        debug.field(&field as &dyn Debug);
    }
    debug.finish()
}

#[cfg(test)]
mod tests {
    use crate as bevy_reflect;
    use crate::*;
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
