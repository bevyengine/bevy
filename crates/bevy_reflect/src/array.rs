//! Traits and types used to power [array-like] operations via reflection.
//!
//! [array-like]: https://doc.rust-lang.org/book/ch03-02-data-types.html#the-array-type
use crate::dynamic::{DynamicArray, DynamicValue, DynamicValueRef};
use crate::generics::impl_generic_info_methods;
use crate::{
    ty::impl_type_methods, utility::reflect_hasher, Generics, MaybeTyped, Reflect, ReflectRef,
    Type, TypeInfo, TypePath,
};
use alloc::{boxed::Box, vec::Vec};
use core::cmp::Ordering;
use core::{
    any::Any,
    fmt::{Debug, Formatter},
    hash::{Hash, Hasher},
};

/// A trait used to power [array-like] operations via [reflection].
///
/// This corresponds to true Rust arrays like `[T; N]`,
/// but also to any fixed-size linear sequence types.
/// It is expected that implementors of this trait uphold this contract
/// and maintain a fixed size as returned by the [`Array::len`] method.
///
/// Due to the [type-erasing] nature of the reflection API as a whole,
/// this trait does not make any guarantees that the implementor's elements
/// are homogeneous (i.e. all the same type).
///
/// This trait has a blanket implementation over Rust arrays of up to 32 items.
/// This implementation can technically contain more than 32,
/// but the blanket [`GetTypeRegistration`] is only implemented up to the 32
/// item limit due to a [limitation] on [`Deserialize`].
///
/// # Example
///
/// ```
/// use bevy_reflect::{Reflect, array::Array};
///
/// let foo: &dyn Array = &[123_u32, 456_u32, 789_u32];
/// assert_eq!(foo.len(), 3);
///
/// let field: &dyn Reflect = foo.get(0).unwrap();
/// assert_eq!(field.downcast_ref::<u32>(), Some(&123));
/// ```
///
/// [array-like]: https://doc.rust-lang.org/book/ch03-02-data-types.html#the-array-type
/// [reflection]: crate
/// [`List`]: crate::list::List
/// [type-erasing]: https://doc.rust-lang.org/book/ch17-02-trait-objects.html
/// [`GetTypeRegistration`]: crate::GetTypeRegistration
/// [limitation]: https://github.com/serde-rs/serde/issues/1937
/// [`Deserialize`]: ::serde::Deserialize
// Prevents unexpectedly importing this trait when trying to call, for example, `array::map`
#[rust_analyzer::completions(ignore_flyimport_methods)]
pub trait Array: Reflect {
    /// Returns a reference to the element at `index`, or `None` if out of bounds.
    fn get(&self, index: usize) -> Option<&dyn Reflect>;

    /// Returns a mutable reference to the element at `index`, or `None` if out of bounds.
    fn get_mut(&mut self, index: usize) -> Option<&mut dyn Reflect>;

    /// Swaps two elements in place.
    ///
    /// # Panics
    /// Panics if `a` or `b` are out of bounds.
    fn swap(&mut self, a: usize, b: usize);

    /// Returns the number of elements in the array.
    fn len(&self) -> usize;

    /// Returns `true` if the collection contains no elements.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over the array.
    fn iter(&self) -> ArrayIter<'_>;

    /// Drain the elements of this array to get a vector of owned values.
    fn drain(self: Box<Self>) -> Vec<Box<dyn Reflect>>;

    /// Returns the [`ArrayInfo`] for this type.
    fn reflect_array_info(&self) -> &'static ArrayInfo {
        self.reflect_type_info().as_array().unwrap()
    }
}

/// A container for compile-time array info.
#[derive(Clone, Debug)]
pub struct ArrayInfo {
    ty: Type,
    generics: Generics,
    item_info: fn() -> Option<&'static TypeInfo>,
    item_ty: Type,
    capacity: usize,
    #[cfg(feature = "reflect_documentation")]
    docs: Option<&'static str>,
}

impl ArrayInfo {
    /// Create a new [`ArrayInfo`].
    ///
    /// # Arguments
    ///
    /// * `capacity`: The maximum capacity of the underlying array.
    pub fn new<TArray: Array + TypePath, TItem: Reflect + MaybeTyped + TypePath>(
        capacity: usize,
    ) -> Self {
        Self {
            ty: Type::of::<TArray>(),
            generics: Generics::new(),
            item_info: TItem::maybe_type_info,
            item_ty: Type::of::<TItem>(),
            capacity,
            #[cfg(feature = "reflect_documentation")]
            docs: None,
        }
    }

    /// Sets the docstring for this array.
    #[cfg(feature = "reflect_documentation")]
    pub fn with_docs(self, docs: Option<&'static str>) -> Self {
        Self { docs, ..self }
    }

    /// The compile-time capacity of the array.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    impl_type_methods!(ty);

    /// The [`TypeInfo`] of the array item.
    ///
    /// Returns `None` if the array item does not contain static type information,
    /// such as for dynamic types.
    pub fn item_info(&self) -> Option<&'static TypeInfo> {
        (self.item_info)()
    }

    /// The [type] of the array item.
    ///
    /// [type]: Type
    pub fn item_ty(&self) -> Type {
        self.item_ty
    }

    /// The docstring of this array, if any.
    #[cfg(feature = "reflect_documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }

    impl_generic_info_methods!(generics);
}

/// An iterator over an [`Array`].
pub struct ArrayIter<'a> {
    array: &'a dyn Array,
    index: usize,
}

impl ArrayIter<'_> {
    /// Creates a new [`ArrayIter`].
    #[inline]
    pub const fn new(array: &dyn Array) -> ArrayIter<'_> {
        ArrayIter { array, index: 0 }
    }
}

impl<'a> Iterator for ArrayIter<'a> {
    type Item = &'a dyn Reflect;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let value = self.array.get(self.index);
        self.index += value.is_some() as usize;
        value
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.array.len().saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for ArrayIter<'a> {}

/// Returns the `u64` hash of the given [array](Array).
#[inline]
pub fn array_hash<A: Array + ?Sized>(array: &A) -> Option<u64> {
    let mut hasher = reflect_hasher();
    Any::type_id(array).hash(&mut hasher);
    array.len().hash(&mut hasher);
    for value in array.iter() {
        hasher.write_u64(value.reflect_hash()?);
    }
    Some(hasher.finish())
}

/// Returns the `u64` hash of the given [`DynamicArray`].
#[inline]
pub fn dynamic_array_hash(array: &DynamicArray) -> Option<u64> {
    let mut hasher = reflect_hasher();
    // TODO: This type_id is **not** correct.
    Any::type_id(array).hash(&mut hasher);
    array.len().hash(&mut hasher);
    for value in array.iter() {
        hasher.write_u64(value.as_ref().dynamic_hash()?);
    }
    Some(hasher.finish())
}

pub(crate) trait ArrayLike {
    fn len(&self) -> usize;

    fn get(&self, index: usize) -> Option<DynamicValueRef<'_>>;

    fn iter(&self) -> impl Iterator<Item = DynamicValueRef<'_>>;
}

impl ArrayLike for DynamicArray {
    fn len(&self) -> usize {
        self.len()
    }

    fn get(&self, index: usize) -> Option<DynamicValueRef<'_>> {
        self.get(index).map(DynamicValue::as_ref)
    }

    fn iter(&self) -> impl Iterator<Item = DynamicValueRef<'_>> {
        self.iter().map(DynamicValue::as_ref)
    }
}

impl ArrayLike for dyn Array {
    fn len(&self) -> usize {
        self.len()
    }

    fn get(&self, index: usize) -> Option<DynamicValueRef<'_>> {
        self.get(index).map(Into::into)
    }

    fn iter(&self) -> impl Iterator<Item = DynamicValueRef<'_>> {
        self.iter().map(|value| DynamicValueRef::from(value))
    }
}

/// Compares two [arrays](Array) (one concrete and one reflected) to see if they
/// are equal.
///
/// Returns [`None`] if the comparison couldn't even be performed.
#[inline(never)]
pub fn array_partial_eq(a: &dyn Array, b: &dyn Reflect) -> Option<bool> {
    let ReflectRef::Array(b) = b.reflect_ref() else {
        return None;
    };
    array_like_partial_eq(a, b)
}

/// Same as above, but for any two [`ListLike`] types.
pub(crate) fn array_like_partial_eq<L: ArrayLike + ?Sized, R: ArrayLike + ?Sized>(
    a: &L,
    b: &R,
) -> Option<bool> {
    if a.len() != b.len() {
        return Some(false);
    }
    for (a, b) in a.iter().zip(b.iter()) {
        let eq_result = a.dynamic_partial_eq(b);
        if let failed @ (Some(false) | None) = eq_result {
            return failed;
        }
    }
    Some(true)
}

/// Lexicographically compares two [arrays](Array) and returns their ordering.
///
/// Returns [`None`] if the comparison couldn't be performed (e.g., kinds mismatch
/// or an element comparison returns `None`).
#[inline(never)]
pub fn array_partial_cmp(a: &dyn Array, b: &dyn Reflect) -> Option<Ordering> {
    let ReflectRef::Array(b) = b.reflect_ref() else {
        return None;
    };
    array_like_partial_cmp(a, b)
}

/// Same as above, but for any two [`ArrayLike`] types.
pub(crate) fn array_like_partial_cmp<L: ArrayLike + ?Sized, R: ArrayLike + ?Sized>(
    a: &L,
    b: &R,
) -> Option<Ordering> {
    let min_len = core::cmp::min(a.len(), b.len());

    for (a, b) in a.iter().zip(b.iter()).take(min_len) {
        match a.dynamic_partial_cmp(b) {
            None => return None,
            Some(Ordering::Equal) => continue,
            Some(ord) => return Some(ord),
        }
    }

    // If all compared elements were equal, order by length
    Some(a.len().cmp(&b.len()))
}

/// The default debug formatter for [`Array`] types.
///
/// # Example
/// ```
/// use bevy_reflect::Reflect;
///
/// let my_array: &dyn Reflect = &[1, 2, 3];
/// println!("{:#?}", my_array);
///
/// // Output:
///
/// // [
/// //   1,
/// //   2,
/// //   3,
/// // ]
/// ```
#[inline]
pub fn array_debug(dyn_array: &dyn Array, f: &mut Formatter<'_>) -> core::fmt::Result {
    let mut debug = f.debug_list();
    for item in dyn_array.iter() {
        debug.entry(&item as &dyn Debug);
    }
    debug.finish()
}
#[cfg(test)]
mod tests {
    use crate::Reflect;
    use alloc::boxed::Box;

    #[test]
    fn next_index_increment() {
        const SIZE: usize = if cfg!(debug_assertions) {
            4
        } else {
            // If compiled in release mode, verify we dont overflow
            usize::MAX
        };

        let b = Box::new([(); SIZE]).into_reflect();

        let array = b.reflect_ref().as_array().unwrap();

        let mut iter = array.iter();
        iter.index = SIZE - 1;
        assert!(iter.next().is_some());

        // When None we should no longer increase index
        assert!(iter.next().is_none());
        assert!(iter.index == SIZE);
        assert!(iter.next().is_none());
        assert!(iter.index == SIZE);
    }

    #[test]
    fn test_swap() {
        let mut b = Box::new([0usize, 1usize]).into_reflect();
        let array = b.reflect_mut().as_array().unwrap();
        array.swap(0, 1);
        assert_eq!(array.get(0).unwrap().downcast_ref(), Some(&1usize));
        assert_eq!(array.get(1).unwrap().downcast_ref(), Some(&0usize));
    }

    #[test]
    #[should_panic]
    fn test_swap_oob() {
        let mut b = Box::new([(); 2]).into_reflect();
        let array = b.reflect_mut().as_array().unwrap();
        array.swap(0, 10);
    }
}
