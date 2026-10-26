//! Traits and types used to power [list-like] operations via reflection.
//!
//! [list-like]: https://doc.rust-lang.org/book/ch08-01-vectors.html
use alloc::{boxed::Box, vec::Vec};
use core::{
    any::Any,
    cmp::Ordering,
    fmt::{Debug, Formatter},
    hash::{Hash, Hasher},
};

use crate::{
    dynamic::{DynamicList, DynamicValue, DynamicValueRef},
    generics::impl_generic_info_methods,
};
use crate::{
    ty::impl_type_methods, utility::reflect_hasher, FromDynamic, Generics, MaybeTyped, Reflect,
    ReflectRef, Type, TypeInfo, TypePath,
};

/// A trait used to power [list-like] operations via [reflection].
///
/// This corresponds to types, like [`Vec`], which contain an ordered sequence
/// of elements that implement [`Reflect`].
///
/// Unlike the [`Array`](crate::array::Array) trait, implementors of this trait are not expected to
/// maintain a constant length.
/// Methods like [insertion](List::insert) and [removal](List::remove) explicitly allow for their
/// internal size to change.
///
/// [`push`](List::push) and [`pop`](List::pop) have default implementations,
/// however it will generally be more performant to implement them manually
/// as the default implementation uses a very naive approach to find the correct position.
///
/// This trait expects its elements to be ordered linearly from front to back.
/// The _front_ element starts at index 0 with the _back_ element ending at the largest index.
/// This contract above should be upheld by any manual implementors.
///
/// Due to the [type-erasing] nature of the reflection API as a whole,
/// this trait does not make any guarantees that the implementor's elements
/// are homogeneous (i.e. all the same type).
///
/// # Example
///
/// ```
/// use bevy_reflect::{Reflect, list::List};
///
/// let foo: &mut dyn List = &mut vec![123_u32, 456_u32, 789_u32];
/// assert_eq!(foo.len(), 3);
///
/// let last_field: Box<dyn Reflect> = foo.pop().unwrap();
/// assert_eq!(last_field.downcast_ref::<u32>(), Some(&789));
/// ```
///
/// [list-like]: https://doc.rust-lang.org/book/ch08-01-vectors.html
/// [reflection]: crate
/// [type-erasing]: https://doc.rust-lang.org/book/ch17-02-trait-objects.html
// Prevents unexpectedly importing this trait when trying to call, for example, `Vec::iter`
#[rust_analyzer::completions(ignore_flyimport_methods)]
pub trait List: Reflect {
    /// Returns a reference to the element at `index`, or `None` if out of bounds.
    fn get(&self, index: usize) -> Option<&dyn Reflect>;

    /// Returns a mutable reference to the element at `index`, or `None` if out of bounds.
    fn get_mut(&mut self, index: usize) -> Option<&mut dyn Reflect>;

    /// Constructs a value for this list from a dynamic value representation.
    fn value_from_dynamic(&self, dynamic: DynamicValue) -> Option<Box<dyn Reflect>>;

    /// Inserts an element at position `index` within the list,
    /// shifting all elements after it towards the back of the list.
    ///
    /// # Panics
    /// Panics if `index > len`.
    fn insert(&mut self, index: usize, element: Box<dyn Reflect>);

    /// Removes and returns the element at position `index` within the list,
    /// shifting all elements before it towards the front of the list.
    ///
    /// # Panics
    /// Panics if `index` is out of bounds.
    fn remove(&mut self, index: usize) -> Box<dyn Reflect>;

    /// Swaps two elements in place.
    ///
    /// # Panics
    /// Panics if `a` or `b` are out of bounds.
    fn swap(&mut self, a: usize, b: usize);

    /// Appends an element to the _back_ of the list.
    fn push(&mut self, value: Box<dyn Reflect>) {
        self.insert(self.len(), value);
    }

    /// Removes the _back_ element from the list and returns it, or [`None`] if it is empty.
    fn pop(&mut self) -> Option<Box<dyn Reflect>> {
        if self.is_empty() {
            None
        } else {
            Some(self.remove(self.len() - 1))
        }
    }

    /// Returns the number of elements in the list.
    fn len(&self) -> usize;

    /// Returns `true` if the collection contains no elements.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over the list.
    fn iter(&self) -> ListIter<'_>;

    /// Drain the elements of this list to get a vector of owned values.
    ///
    /// After calling this function, `self` will be empty. The order of items in the returned
    /// [`Vec`] will match the order of items in `self`.
    fn drain(&mut self) -> Vec<Box<dyn Reflect>>;

    /// Returns the [`ListInfo`] for this type.
    fn reflect_list_info(&self) -> &'static ListInfo {
        self.reflect_type_info().as_list().unwrap()
    }
}

/// A container for compile-time list info.
#[derive(Clone, Debug)]
pub struct ListInfo {
    ty: Type,
    generics: Generics,
    item_info: fn() -> Option<&'static TypeInfo>,
    item_ty: Type,
    #[cfg(feature = "reflect_documentation")]
    docs: Option<&'static str>,
}

impl ListInfo {
    /// Create a new [`ListInfo`].
    pub fn new<TList: List + TypePath, TItem: FromDynamic + MaybeTyped + TypePath>() -> Self {
        Self {
            ty: Type::of::<TList>(),
            generics: Generics::new(),
            item_info: TItem::maybe_type_info,
            item_ty: Type::of::<TItem>(),
            #[cfg(feature = "reflect_documentation")]
            docs: None,
        }
    }

    /// Sets the docstring for this list.
    #[cfg(feature = "reflect_documentation")]
    pub fn with_docs(self, docs: Option<&'static str>) -> Self {
        Self { docs, ..self }
    }

    impl_type_methods!(ty);

    /// The [`TypeInfo`] of the list item.
    ///
    /// Returns `None` if the list item does not contain static type information,
    /// such as for dynamic types.
    pub fn item_info(&self) -> Option<&'static TypeInfo> {
        (self.item_info)()
    }

    /// The [type] of the list item.
    ///
    /// [type]: Type
    pub fn item_ty(&self) -> Type {
        self.item_ty
    }

    /// The docstring of this list, if any.
    #[cfg(feature = "reflect_documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }

    impl_generic_info_methods!(generics);
}

/// An iterator over an [`List`].
pub struct ListIter<'a> {
    list: &'a dyn List,
    index: usize,
}

impl ListIter<'_> {
    /// Creates a new [`ListIter`].
    #[inline]
    pub const fn new(list: &dyn List) -> ListIter<'_> {
        ListIter { list, index: 0 }
    }
}

impl<'a> Iterator for ListIter<'a> {
    type Item = &'a dyn Reflect;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let value = self.list.get(self.index);
        self.index += value.is_some() as usize;
        value
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.list.len().saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for ListIter<'_> {}

/// Returns the `u64` hash of the given [list](List).
#[inline]
pub fn list_hash<L: List>(list: &L) -> Option<u64> {
    let mut hasher = reflect_hasher();
    Any::type_id(list).hash(&mut hasher);
    list.len().hash(&mut hasher);
    for value in list.iter() {
        hasher.write_u64(value.reflect_hash()?);
    }
    Some(hasher.finish())
}

/// Returns the `u64` hash of the given [`DynamicList`].
#[inline]
pub fn dynamic_list_hash(list: &DynamicList) -> Option<u64> {
    let mut hasher = reflect_hasher();
    // TODO: This type_id is **not** correct.
    Any::type_id(list).hash(&mut hasher);
    list.len().hash(&mut hasher);
    for value in list.iter() {
        hasher.write_u64(value.as_ref().dynamic_hash()?);
    }
    Some(hasher.finish())
}

pub(crate) trait ListLike {
    fn len(&self) -> usize;

    fn get(&self, index: usize) -> Option<DynamicValueRef<'_>>;

    fn iter(&self) -> impl Iterator<Item = DynamicValueRef<'_>>;
}

impl ListLike for DynamicList {
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

impl ListLike for dyn List {
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

/// Compares a [`List`] with a [`Reflect`] value.
///
/// Returns true if and only if all of the following are true:
/// - `b` is a list;
/// - `b` is the same length as `a`;
/// - [`Reflect::reflect_partial_eq`] returns `Some(true)` for pairwise elements of `a` and `b`.
///
/// Returns [`None`] if the comparison couldn't even be performed.
#[inline(never)]
pub fn list_partial_eq(a: &dyn List, b: &dyn Reflect) -> Option<bool> {
    let ReflectRef::List(b) = b.reflect_ref() else {
        return Some(false);
    };
    list_like_partial_eq(a, b)
}

/// Same as above, but for any two [`ListLike`] types.
pub(crate) fn list_like_partial_eq<L: ListLike + ?Sized, R: ListLike + ?Sized>(
    a: &L,
    b: &R,
) -> Option<bool> {
    if a.len() != b.len() {
        return Some(false);
    }

    for (a_value, b_value) in a.iter().zip(b.iter()) {
        let eq_result = a_value.dynamic_partial_eq(b_value);
        if let failed @ (Some(false) | None) = eq_result {
            return failed;
        }
    }

    Some(true)
}

/// Lexicographically compares two [List] values and returns their ordering.
///
/// Returns [`None`] if the comparison couldn't be performed (e.g., kinds mismatch
/// or an element comparison returns `None`).
#[inline(never)]
pub fn list_partial_cmp(a: &dyn List, b: &dyn Reflect) -> Option<Ordering> {
    let ReflectRef::List(b) = b.reflect_ref() else {
        return None;
    };
    list_like_partial_cmp(a, b)
}

/// Same as above, but for any two [`ListLike`] types.
pub(crate) fn list_like_partial_cmp<L: ListLike + ?Sized, R: ListLike + ?Sized>(
    a: &L,
    b: &R,
) -> Option<Ordering> {
    let min_len = core::cmp::min(a.len(), b.len());

    for (a_value, b_value) in a.iter().zip(b.iter()).take(min_len) {
        match a_value.dynamic_partial_cmp(b_value) {
            None => return None,
            Some(Ordering::Equal) => continue,
            Some(ord) => return Some(ord),
        }
    }

    Some(a.len().cmp(&b.len()))
}

/// The default debug formatter for [`List`] types.
///
/// # Example
/// ```
/// use bevy_reflect::Reflect;
///
/// let my_list: &dyn Reflect = &vec![1, 2, 3];
/// println!("{:#?}", my_list);
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
pub fn list_debug(dyn_list: &dyn List, f: &mut Formatter<'_>) -> core::fmt::Result {
    let mut debug = f.debug_list();
    for item in dyn_list.iter() {
        debug.entry(&item as &dyn Debug);
    }
    debug.finish()
}

#[cfg(test)]
mod tests {
    use crate::Reflect;
    use alloc::{boxed::Box, vec};

    #[test]
    fn next_index_increment() {
        const SIZE: usize = if cfg!(debug_assertions) {
            4
        } else {
            // If compiled in release mode, verify we dont overflow
            usize::MAX
        };
        let b = Box::new(vec![(); SIZE]).into_reflect();

        let list = b.reflect_ref().as_list().unwrap();

        let mut iter = list.iter();
        iter.index = SIZE - 1;
        assert!(iter.next().is_some());

        // When None we should no longer increase index
        assert!(iter.next().is_none());
        assert!(iter.index == SIZE);
        assert!(iter.next().is_none());
        assert!(iter.index == SIZE);
    }
}
