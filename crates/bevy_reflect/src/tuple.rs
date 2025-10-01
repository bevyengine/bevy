use bevy_reflect_derive::impl_type_path;
use variadics_please::all_tuples;

use crate::generics::impl_generic_info_methods;
use crate::{
    type_info::impl_type_methods, utility::GenericTypePathCell, ApplyError, FromReflect, Generics,
    GetTypeRegistration, MaybeTyped, PartialReflect, Reflect, ReflectCloneError, ReflectKind,
    ReflectMut, ReflectOwned, ReflectRef, Type, TypeInfo, TypePath, TypeRegistration, TypeRegistry,
    Typed, UnnamedField,
};
use alloc::{boxed::Box, vec, vec::Vec};
use core::{
    any::Any,
    fmt::{Debug, Formatter},
    slice::Iter,
};

/// A trait used to power [tuple-like] operations via [reflection].
///
/// This trait uses the [`Reflect`] trait to allow implementors to have their fields
/// be dynamically addressed by index.
///
/// This trait is automatically implemented for arbitrary tuples of up to 12
/// elements, provided that each element implements [`Reflect`].
///
/// # Example
///
/// ```
/// use bevy_reflect::{PartialReflect, Tuple};
///
/// let foo = (123_u32, true);
/// assert_eq!(foo.field_len(), 2);
///
/// let field: &dyn PartialReflect = foo.field(0).unwrap();
/// assert_eq!(field.try_downcast_ref::<u32>(), Some(&123));
/// ```
///
/// [tuple-like]: https://doc.rust-lang.org/book/ch03-02-data-types.html#the-tuple-type
/// [reflection]: crate
pub trait Tuple: PartialReflect {
    /// Returns a reference to the value of the field with index `index` as a
    /// `&dyn Reflect`.
    fn field(&self, index: usize) -> Option<&dyn PartialReflect>;

    /// Returns a mutable reference to the value of the field with index `index`
    /// as a `&mut dyn Reflect`.
    fn field_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect>;

    /// Returns the number of fields in the tuple.
    fn field_len(&self) -> usize;

    /// Returns an iterator over the values of the tuple's fields.
    fn iter_fields(&self) -> TupleFieldIter<'_>;

    /// Drain the fields of this tuple to get a vector of owned values.
    fn drain(self: Box<Self>) -> Vec<Box<dyn PartialReflect>>;

    /// Creates a new [`DynamicTuple`] from this tuple.
    fn to_dynamic_tuple(&self) -> DynamicTuple {
        DynamicTuple {
            represented_type: self.get_represented_type_info(),
            fields: self.iter_fields().map(PartialReflect::to_dynamic).collect(),
        }
    }

    /// Will return `None` if [`TypeInfo`] is not available.
    fn get_represented_tuple_info(&self) -> Option<&'static TupleInfo> {
        self.get_represented_type_info()?.as_tuple().ok()
    }
}

/// An iterator over the field values of a tuple.
pub struct TupleFieldIter<'a> {
    pub(crate) tuple: &'a dyn Tuple,
    pub(crate) index: usize,
}

impl<'a> TupleFieldIter<'a> {
    /// Creates a new [`TupleFieldIter`].
    pub fn new(value: &'a dyn Tuple) -> Self {
        TupleFieldIter {
            tuple: value,
            index: 0,
        }
    }
}

impl<'a> Iterator for TupleFieldIter<'a> {
    type Item = &'a dyn PartialReflect;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.tuple.field(self.index);
        self.index += value.is_some() as usize;
        value
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.tuple.field_len();
        (size, Some(size))
    }
}

impl<'a> ExactSizeIterator for TupleFieldIter<'a> {}

/// A convenience trait which combines fetching and downcasting of tuple
/// fields.
///
/// # Example
///
/// ```
/// use bevy_reflect::GetTupleField;
///
/// # fn main() {
/// let foo = ("blue".to_string(), 42_i32);
///
/// assert_eq!(foo.get_field::<String>(0), Some(&"blue".to_string()));
/// assert_eq!(foo.get_field::<i32>(1), Some(&42));
/// # }
/// ```
pub trait GetTupleField {
    /// Returns a reference to the value of the field with index `index`,
    /// downcast to `T`.
    fn get_field<T: Reflect>(&self, index: usize) -> Option<&T>;

    /// Returns a mutable reference to the value of the field with index
    /// `index`, downcast to `T`.
    fn get_field_mut<T: Reflect>(&mut self, index: usize) -> Option<&mut T>;
}

impl<S: Tuple> GetTupleField for S {
    fn get_field<T: Reflect>(&self, index: usize) -> Option<&T> {
        self.field(index)
            .and_then(|value| value.try_downcast_ref::<T>())
    }

    fn get_field_mut<T: Reflect>(&mut self, index: usize) -> Option<&mut T> {
        self.field_mut(index)
            .and_then(|value| value.try_downcast_mut::<T>())
    }
}

impl GetTupleField for dyn Tuple {
    fn get_field<T: Reflect>(&self, index: usize) -> Option<&T> {
        self.field(index)
            .and_then(|value| value.try_downcast_ref::<T>())
    }

    fn get_field_mut<T: Reflect>(&mut self, index: usize) -> Option<&mut T> {
        self.field_mut(index)
            .and_then(|value| value.try_downcast_mut::<T>())
    }
}

/// A container for compile-time tuple info.
#[derive(Clone, Debug)]
pub struct TupleInfo {
    ty: Type,
    generics: Generics,
    fields: Box<[UnnamedField]>,
    #[cfg(feature = "documentation")]
    docs: Option<&'static str>,
}

impl TupleInfo {
    /// Create a new [`TupleInfo`].
    ///
    /// # Arguments
    ///
    /// * `fields`: The fields of this tuple in the order they are defined
    pub fn new<T: Reflect + TypePath>(fields: &[UnnamedField]) -> Self {
        Self {
            ty: Type::of::<T>(),
            generics: Generics::new(),
            fields: fields.to_vec().into_boxed_slice(),
            #[cfg(feature = "documentation")]
            docs: None,
        }
    }

    /// Sets the docstring for this tuple.
    #[cfg(feature = "documentation")]
    pub fn with_docs(self, docs: Option<&'static str>) -> Self {
        Self { docs, ..self }
    }

    /// Get the field at the given index.
    pub fn field_at(&self, index: usize) -> Option<&UnnamedField> {
        self.fields.get(index)
    }

    /// Iterate over the fields of this tuple.
    pub fn iter(&self) -> Iter<'_, UnnamedField> {
        self.fields.iter()
    }

    /// The total number of fields in this tuple.
    pub fn field_len(&self) -> usize {
        self.fields.len()
    }

    impl_type_methods!(ty);

    /// The docstring of this tuple, if any.
    #[cfg(feature = "documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }

    impl_generic_info_methods!(generics);
}

/// A tuple which allows fields to be added at runtime.
#[derive(Default, Debug)]
pub struct DynamicTuple {
    represented_type: Option<&'static TypeInfo>,
    fields: Vec<Box<dyn PartialReflect>>,
}

impl DynamicTuple {
    /// Sets the [type] to be represented by this `DynamicTuple`.
    ///
    /// # Panics
    ///
    /// Panics if the given [type] is not a [`TypeInfo::Tuple`].
    ///
    /// [type]: TypeInfo
    pub fn set_represented_type(&mut self, represented_type: Option<&'static TypeInfo>) {
        if let Some(represented_type) = represented_type {
            assert!(
                matches!(represented_type, TypeInfo::Tuple(_)),
                "expected TypeInfo::Tuple but received: {represented_type:?}"
            );
        }
        self.represented_type = represented_type;
    }

    /// Appends an element with value `value` to the tuple.
    pub fn insert_boxed(&mut self, value: Box<dyn PartialReflect>) {
        self.represented_type = None;
        self.fields.push(value);
    }

    /// Appends a typed element with value `value` to the tuple.
    pub fn insert<T: PartialReflect>(&mut self, value: T) {
        self.represented_type = None;
        self.insert_boxed(Box::new(value));
    }
}

impl Tuple for DynamicTuple {
    #[inline]
    fn field(&self, index: usize) -> Option<&dyn PartialReflect> {
        self.fields.get(index).map(|field| &**field)
    }

    #[inline]
    fn field_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        self.fields.get_mut(index).map(|field| &mut **field)
    }

    #[inline]
    fn field_len(&self) -> usize {
        self.fields.len()
    }

    #[inline]
    fn iter_fields(&self) -> TupleFieldIter<'_> {
        TupleFieldIter {
            tuple: self,
            index: 0,
        }
    }

    #[inline]
    fn drain(self: Box<Self>) -> Vec<Box<dyn PartialReflect>> {
        self.fields
    }
}

impl PartialReflect for DynamicTuple {
    #[inline]
    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        self.represented_type
    }

    #[inline]
    fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
        self
    }

    fn as_partial_reflect(&self) -> &dyn PartialReflect {
        self
    }

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

    fn apply(&mut self, value: &dyn PartialReflect) {
        tuple_apply(self, value);
    }

    #[inline]
    fn reflect_kind(&self) -> ReflectKind {
        ReflectKind::Tuple
    }

    #[inline]
    fn reflect_ref(&self) -> ReflectRef<'_> {
        ReflectRef::Tuple(self)
    }

    #[inline]
    fn reflect_mut(&mut self) -> ReflectMut<'_> {
        ReflectMut::Tuple(self)
    }

    #[inline]
    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Tuple(self)
    }

    fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
        tuple_try_apply(self, value)
    }

    fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
        tuple_partial_eq(self, value)
    }

    fn debug(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "DynamicTuple(")?;
        tuple_debug(self, f)?;
        write!(f, ")")
    }

    #[inline]
    fn is_dynamic(&self) -> bool {
        true
    }
}

impl_type_path!((in bevy_reflect) DynamicTuple);

impl FromIterator<Box<dyn PartialReflect>> for DynamicTuple {
    fn from_iter<I: IntoIterator<Item = Box<dyn PartialReflect>>>(fields: I) -> Self {
        Self {
            represented_type: None,
            fields: fields.into_iter().collect(),
        }
    }
}

impl IntoIterator for DynamicTuple {
    type Item = Box<dyn PartialReflect>;
    type IntoIter = vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.fields.into_iter()
    }
}

impl<'a> IntoIterator for &'a DynamicTuple {
    type Item = &'a dyn PartialReflect;
    type IntoIter = TupleFieldIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_fields()
    }
}

/// Applies the elements of `b` to the corresponding elements of `a`.
///
/// # Panics
///
/// This function panics if `b` is not a tuple.
#[inline]
pub fn tuple_apply<T: Tuple>(a: &mut T, b: &dyn PartialReflect) {
    if let Err(err) = tuple_try_apply(a, b) {
        panic!("{err}");
    }
}

/// Tries to apply the elements of `b` to the corresponding elements of `a` and
/// returns a Result.
///
/// # Errors
///
/// This function returns an [`ApplyError::MismatchedKinds`] if `b` is not a tuple or if
/// applying elements to each other fails.
#[inline]
pub fn tuple_try_apply<T: Tuple>(a: &mut T, b: &dyn PartialReflect) -> Result<(), ApplyError> {
    let tuple = b.reflect_ref().as_tuple()?;

    for (i, value) in tuple.iter_fields().enumerate() {
        if let Some(v) = a.field_mut(i) {
            v.try_apply(value)?;
        }
    }

    Ok(())
}

/// Compares a [`Tuple`] with a [`PartialReflect`] value.
///
/// Returns true if and only if all of the following are true:
/// - `b` is a tuple;
/// - `b` has the same number of elements as `a`;
/// - [`PartialReflect::reflect_partial_eq`] returns `Some(true)` for pairwise elements of `a` and `b`.
///
/// Returns [`None`] if the comparison couldn't even be performed.
#[inline]
pub fn tuple_partial_eq<T: Tuple + ?Sized>(a: &T, b: &dyn PartialReflect) -> Option<bool> {
    let ReflectRef::Tuple(b) = b.reflect_ref() else {
        return Some(false);
    };

    if a.field_len() != b.field_len() {
        return Some(false);
    }

    for (a_field, b_field) in a.iter_fields().zip(b.iter_fields()) {
        let eq_result = a_field.reflect_partial_eq(b_field);
        if let failed @ (Some(false) | None) = eq_result {
            return failed;
        }
    }

    Some(true)
}

/// The default debug formatter for [`Tuple`] types.
///
/// # Example
/// ```
/// use bevy_reflect::Reflect;
///
/// let my_tuple: &dyn Reflect = &(1, 2, 3);
/// println!("{:#?}", my_tuple);
///
/// // Output:
///
/// // (
/// //   1,
/// //   2,
/// //   3,
/// // )
/// ```
#[inline]
pub fn tuple_debug(dyn_tuple: &dyn Tuple, f: &mut Formatter<'_>) -> core::fmt::Result {
    let mut debug = f.debug_tuple("");
    for field in dyn_tuple.iter_fields() {
        debug.field(&field as &dyn Debug);
    }
    debug.finish()
}

macro_rules! impl_reflect_tuple {
    {$($index:tt : $name:tt),*} => {
        impl<$($name: Reflect + MaybeTyped + TypePath + GetTypeRegistration),*> Tuple for ($($name,)*) {
            #[inline]
            fn field(&self, index: usize) -> Option<&dyn PartialReflect> {
                match index {
                    $($index => Some(&self.$index as &dyn PartialReflect),)*
                    _ => None,
                }
            }

            #[inline]
            fn field_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
                match index {
                    $($index => Some(&mut self.$index as &mut dyn PartialReflect),)*
                    _ => None,
                }
            }

            #[inline]
            fn field_len(&self) -> usize {
                let indices: &[usize] = &[$($index as usize),*];
                indices.len()
            }

            #[inline]
            fn iter_fields(&self) -> TupleFieldIter<'_> {
                TupleFieldIter {
                    tuple: self,
                    index: 0,
                }
            }

            #[inline]
            fn drain(self: Box<Self>) -> Vec<Box<dyn PartialReflect>> {
                vec![
                    $(Box::new(self.$index),)*
                ]
            }
        }

        impl<$($name: Reflect + MaybeTyped + TypePath + GetTypeRegistration),*> PartialReflect for ($($name,)*) {
            fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
                Some(<Self as Typed>::type_info())
            }

            #[inline]
            fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
                self
            }

            fn as_partial_reflect(&self) -> &dyn PartialReflect {
                self
            }

            fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
                self
            }

            fn try_into_reflect(self: Box<Self>) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
                Ok(self)
            }

            fn try_as_reflect(&self) -> Option<&dyn Reflect> {
                Some(self)
            }

            fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> {
                Some(self)
            }

            fn reflect_kind(&self) -> ReflectKind {
                ReflectKind::Tuple
            }

            fn reflect_ref(&self) -> ReflectRef <'_> {
                ReflectRef::Tuple(self)
            }

            fn reflect_mut(&mut self) -> ReflectMut <'_> {
                ReflectMut::Tuple(self)
            }

            fn reflect_owned(self: Box<Self>) -> ReflectOwned {
                ReflectOwned::Tuple(self)
            }

            fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
                crate::tuple_partial_eq(self, value)
            }

            fn apply(&mut self, value: &dyn PartialReflect) {
                crate::tuple_apply(self, value);
            }

            fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
                crate::tuple_try_apply(self, value)
            }

            fn reflect_clone(&self) -> Result<Box<dyn Reflect>, ReflectCloneError> {
                Ok(Box::new((
                    $(
                        self.$index.reflect_clone()?
                            .take::<$name>()
                            .expect("`Reflect::reflect_clone` should return the same type"),
                    )*
                )))
            }
        }

        impl<$($name: Reflect + MaybeTyped + TypePath + GetTypeRegistration),*> Reflect for ($($name,)*) {
            fn into_any(self: Box<Self>) -> Box<dyn Any> {
                self
            }

            fn as_any(&self) -> &dyn Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }

            fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
                self
            }

            fn as_reflect(&self) -> &dyn Reflect {
                self
            }

            fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
                self
            }

            fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
                *self = value.take()?;
                Ok(())
            }
        }

        impl <$($name: Reflect + MaybeTyped + TypePath + GetTypeRegistration),*> Typed for ($($name,)*) {
            fn type_info() -> &'static TypeInfo {
                static CELL: $crate::utility::GenericTypeInfoCell = $crate::utility::GenericTypeInfoCell::new();
                CELL.get_or_insert::<Self, _>(|| {
                    let fields = [
                        $(UnnamedField::new::<$name>($index),)*
                    ];
                    let info = TupleInfo::new::<Self>(&fields);
                    TypeInfo::Tuple(info)
                })
            }
        }

        impl<$($name: Reflect + MaybeTyped + TypePath + GetTypeRegistration),*> GetTypeRegistration for ($($name,)*) {
            fn get_type_registration() -> TypeRegistration {
                TypeRegistration::of::<($($name,)*)>()
            }

            fn register_type_dependencies(_registry: &mut TypeRegistry) {
                $(_registry.register::<$name>();)*
            }
        }

        impl<$($name: FromReflect + MaybeTyped + TypePath + GetTypeRegistration),*> FromReflect for ($($name,)*)
        {
            fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
                let _ref_tuple = reflect.reflect_ref().as_tuple().ok()?;

                Some(
                    (
                        $(
                            <$name as FromReflect>::from_reflect(_ref_tuple.field($index)?)?,
                        )*
                    )
                )
            }
        }
    }
}

impl_reflect_tuple! {}

impl_reflect_tuple! {0: A}

impl_reflect_tuple! {0: A, 1: B}

impl_reflect_tuple! {0: A, 1: B, 2: C}

impl_reflect_tuple! {0: A, 1: B, 2: C, 3: D}

impl_reflect_tuple! {0: A, 1: B, 2: C, 3: D, 4: E}

impl_reflect_tuple! {0: A, 1: B, 2: C, 3: D, 4: E, 5: F}

impl_reflect_tuple! {0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G}

impl_reflect_tuple! {0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H}

impl_reflect_tuple! {0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I}

impl_reflect_tuple! {0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J}

impl_reflect_tuple! {0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K}

impl_reflect_tuple! {0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K, 11: L}

macro_rules! impl_type_path_tuple {
    ($(#[$meta:meta])*) => {
        $(#[$meta])*
        impl TypePath for () {
            fn type_path() -> &'static str {
                "()"
            }

            fn short_type_path() -> &'static str {
                "()"
            }
        }
    };

    ($(#[$meta:meta])* $param:ident) => {
        $(#[$meta])*
        impl <$param: TypePath> TypePath for ($param,) {
            fn type_path() -> &'static str {
                use $crate::__macro_exports::alloc_utils::ToOwned;
                static CELL: GenericTypePathCell = GenericTypePathCell::new();
                CELL.get_or_insert::<Self, _>(|| {
                    "(".to_owned() + $param::type_path() + ",)"
                })
            }

            fn short_type_path() -> &'static str {
                use $crate::__macro_exports::alloc_utils::ToOwned;
                static CELL: GenericTypePathCell = GenericTypePathCell::new();
                CELL.get_or_insert::<Self, _>(|| {
                    "(".to_owned() + $param::short_type_path() + ",)"
                })
            }
        }
    };

    ($(#[$meta:meta])* $last:ident $(,$param:ident)*) => {
        $(#[$meta])*
        impl <$($param: TypePath,)* $last: TypePath> TypePath for ($($param,)* $last) {
            fn type_path() -> &'static str {
                use $crate::__macro_exports::alloc_utils::ToOwned;
                static CELL: GenericTypePathCell = GenericTypePathCell::new();
                CELL.get_or_insert::<Self, _>(|| {
                    "(".to_owned() $(+ $param::type_path() + ", ")* + $last::type_path() + ")"
                })
            }

            fn short_type_path() -> &'static str {
                use $crate::__macro_exports::alloc_utils::ToOwned;
                static CELL: GenericTypePathCell = GenericTypePathCell::new();
                CELL.get_or_insert::<Self, _>(|| {
                    "(".to_owned() $(+ $param::short_type_path() + ", ")* + $last::short_type_path() + ")"
                })
            }
        }
    };
}

all_tuples!(
    #[doc(fake_variadic)]
    impl_type_path_tuple,
    0,
    12,
    P
);

#[cfg(feature = "functions")]
const _: () = {
    macro_rules! impl_get_ownership_tuple {
    ($(#[$meta:meta])* $($name: ident),*) => {
        $(#[$meta])*
        $crate::func::args::impl_get_ownership!(($($name,)*); <$($name),*>);
    };
}

    all_tuples!(
        #[doc(fake_variadic)]
        impl_get_ownership_tuple,
        0,
        12,
        P
    );

    macro_rules! impl_from_arg_tuple {
    ($(#[$meta:meta])* $($name: ident),*) => {
        $(#[$meta])*
        $crate::func::args::impl_from_arg!(($($name,)*); <$($name: FromReflect + MaybeTyped + TypePath + GetTypeRegistration),*>);
    };
}

    all_tuples!(
        #[doc(fake_variadic)]
        impl_from_arg_tuple,
        0,
        12,
        P
    );

    macro_rules! impl_into_return_tuple {
    ($(#[$meta:meta])* $($name: ident),+) => {
        $(#[$meta])*
        $crate::func::impl_into_return!(($($name,)*); <$($name: FromReflect + MaybeTyped + TypePath + GetTypeRegistration),*>);
    };
}

    // The unit type (i.e. `()`) is special-cased, so we skip implementing it here.
    all_tuples!(
        #[doc(fake_variadic)]
        impl_into_return_tuple,
        1,
        12,
        P
    );
};

#[cfg(test)]
mod tests {
    use super::Tuple;

    #[test]
    fn next_index_increment() {
        let mut iter = (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11).iter_fields();
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
