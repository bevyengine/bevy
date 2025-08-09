use alloc::{boxed::Box, vec::Vec};
use core::{
    any::Any,
    fmt::{Debug, Formatter},
    hash::{Hash, Hasher},
};

use bevy_reflect_derive::impl_type_path;

use crate::generics::impl_generic_info_methods;
use crate::{
    type_info::impl_type_methods, utility::reflect_hasher, ApplyError, FromReflect, Generics,
    MaybeTyped, PartialReflect, Reflect, ReflectKind, ReflectMut, ReflectOwned, ReflectRef, Type,
    TypeInfo, TypePath,
};

/// A trait used to power [list-like] operations via [reflection].
///
/// This corresponds to types, like [`Vec`], which contain an ordered sequence
/// of elements that implement [`Reflect`].
///
/// Unlike the [`Array`](crate::Array) trait, implementors of this trait are not expected to
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
/// use bevy_reflect::{PartialReflect, Reflect, List};
///
/// let foo: &mut dyn List = &mut vec![123_u32, 456_u32, 789_u32];
/// assert_eq!(foo.len(), 3);
///
/// let last_field: Box<dyn PartialReflect> = foo.pop().unwrap();
/// assert_eq!(last_field.try_downcast_ref::<u32>(), Some(&789));
/// ```
///
/// [list-like]: https://doc.rust-lang.org/book/ch08-01-vectors.html
/// [reflection]: crate
/// [type-erasing]: https://doc.rust-lang.org/book/ch17-02-trait-objects.html
pub trait List: PartialReflect {
    /// Returns a reference to the element at `index`, or `None` if out of bounds.
    fn get(&self, index: usize) -> Option<&dyn PartialReflect>;

    /// Returns a mutable reference to the element at `index`, or `None` if out of bounds.
    fn get_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect>;

    /// Inserts an element at position `index` within the list,
    /// shifting all elements after it towards the back of the list.
    ///
    /// # Panics
    /// Panics if `index > len`.
    fn insert(&mut self, index: usize, element: Box<dyn PartialReflect>);

    /// Removes and returns the element at position `index` within the list,
    /// shifting all elements before it towards the front of the list.
    ///
    /// # Panics
    /// Panics if `index` is out of bounds.
    fn remove(&mut self, index: usize) -> Box<dyn PartialReflect>;

    /// Appends an element to the _back_ of the list.
    fn push(&mut self, value: Box<dyn PartialReflect>) {
        self.insert(self.len(), value);
    }

    /// Removes the _back_ element from the list and returns it, or [`None`] if it is empty.
    fn pop(&mut self) -> Option<Box<dyn PartialReflect>> {
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
    fn drain(&mut self) -> Vec<Box<dyn PartialReflect>>;

    /// Creates a new [`DynamicList`] from this list.
    fn to_dynamic_list(&self) -> DynamicList {
        DynamicList {
            represented_type: self.get_represented_type_info(),
            values: self.iter().map(PartialReflect::to_dynamic).collect(),
        }
    }

    /// Will return `None` if [`TypeInfo`] is not available.
    fn get_represented_list_info(&self) -> Option<&'static ListInfo> {
        self.get_represented_type_info()?.as_list().ok()
    }
}

/// A container for compile-time list info.
#[derive(Clone, Debug)]
pub struct ListInfo {
    ty: Type,
    generics: Generics,
    item_info: fn() -> Option<&'static TypeInfo>,
    item_ty: Type,
    #[cfg(feature = "documentation")]
    docs: Option<&'static str>,
}

impl ListInfo {
    /// Create a new [`ListInfo`].
    pub fn new<TList: List + TypePath, TItem: FromReflect + MaybeTyped + TypePath>() -> Self {
        Self {
            ty: Type::of::<TList>(),
            generics: Generics::new(),
            item_info: TItem::maybe_type_info,
            item_ty: Type::of::<TItem>(),
            #[cfg(feature = "documentation")]
            docs: None,
        }
    }

    /// Sets the docstring for this list.
    #[cfg(feature = "documentation")]
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
    #[cfg(feature = "documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }

    impl_generic_info_methods!(generics);
}

/// A list of reflected values.
#[derive(Default)]
pub struct DynamicList {
    represented_type: Option<&'static TypeInfo>,
    values: Vec<Box<dyn PartialReflect>>,
}

impl DynamicList {
    /// Sets the [type] to be represented by this `DynamicList`.
    /// # Panics
    ///
    /// Panics if the given [type] is not a [`TypeInfo::List`].
    ///
    /// [type]: TypeInfo
    pub fn set_represented_type(&mut self, represented_type: Option<&'static TypeInfo>) {
        if let Some(represented_type) = represented_type {
            assert!(
                matches!(represented_type, TypeInfo::List(_)),
                "expected TypeInfo::List but received: {represented_type:?}"
            );
        }

        self.represented_type = represented_type;
    }

    /// Appends a typed value to the list.
    pub fn push<T: PartialReflect>(&mut self, value: T) {
        self.values.push(Box::new(value));
    }

    /// Appends a [`Reflect`] trait object to the list.
    pub fn push_box(&mut self, value: Box<dyn PartialReflect>) {
        self.values.push(value);
    }
}

impl List for DynamicList {
    fn get(&self, index: usize) -> Option<&dyn PartialReflect> {
        self.values.get(index).map(|value| &**value)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        self.values.get_mut(index).map(|value| &mut **value)
    }

    fn insert(&mut self, index: usize, element: Box<dyn PartialReflect>) {
        self.values.insert(index, element);
    }

    fn remove(&mut self, index: usize) -> Box<dyn PartialReflect> {
        self.values.remove(index)
    }

    fn push(&mut self, value: Box<dyn PartialReflect>) {
        DynamicList::push_box(self, value);
    }

    fn pop(&mut self) -> Option<Box<dyn PartialReflect>> {
        self.values.pop()
    }

    fn len(&self) -> usize {
        self.values.len()
    }

    fn iter(&self) -> ListIter<'_> {
        ListIter::new(self)
    }

    fn drain(&mut self) -> Vec<Box<dyn PartialReflect>> {
        self.values.drain(..).collect()
    }
}

impl PartialReflect for DynamicList {
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

    fn apply(&mut self, value: &dyn PartialReflect) {
        list_apply(self, value);
    }

    fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
        list_try_apply(self, value)
    }

    #[inline]
    fn reflect_kind(&self) -> ReflectKind {
        ReflectKind::List
    }

    #[inline]
    fn reflect_ref(&self) -> ReflectRef<'_> {
        ReflectRef::List(self)
    }

    #[inline]
    fn reflect_mut(&mut self) -> ReflectMut<'_> {
        ReflectMut::List(self)
    }

    #[inline]
    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::List(self)
    }

    #[inline]
    fn reflect_hash(&self) -> Option<u64> {
        list_hash(self)
    }

    fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
        list_partial_eq(self, value)
    }

    fn debug(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "DynamicList(")?;
        list_debug(self, f)?;
        write!(f, ")")
    }

    #[inline]
    fn is_dynamic(&self) -> bool {
        true
    }
}

impl_type_path!((in bevy_reflect) DynamicList);

impl Debug for DynamicList {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        self.debug(f)
    }
}

impl FromIterator<Box<dyn PartialReflect>> for DynamicList {
    fn from_iter<I: IntoIterator<Item = Box<dyn PartialReflect>>>(values: I) -> Self {
        Self {
            represented_type: None,
            values: values.into_iter().collect(),
        }
    }
}

impl<T: PartialReflect> FromIterator<T> for DynamicList {
    fn from_iter<I: IntoIterator<Item = T>>(values: I) -> Self {
        values
            .into_iter()
            .map(|field| Box::new(field).into_partial_reflect())
            .collect()
    }
}

impl IntoIterator for DynamicList {
    type Item = Box<dyn PartialReflect>;
    type IntoIter = alloc::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

impl<'a> IntoIterator for &'a DynamicList {
    type Item = &'a dyn PartialReflect;
    type IntoIter = ListIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
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
    type Item = &'a dyn PartialReflect;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let value = self.list.get(self.index);
        self.index += value.is_some() as usize;
        value
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.list.len();
        (size, Some(size))
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

/// Applies the elements of `b` to the corresponding elements of `a`.
///
/// If the length of `b` is greater than that of `a`, the excess elements of `b`
/// are cloned and appended to `a`.
///
/// # Panics
///
/// This function panics if `b` is not a list.
#[inline]
pub fn list_apply<L: List>(a: &mut L, b: &dyn PartialReflect) {
    if let Err(err) = list_try_apply(a, b) {
        panic!("{err}");
    }
}

/// Tries to apply the elements of `b` to the corresponding elements of `a` and
/// returns a Result.
///
/// If the length of `b` is greater than that of `a`, the excess elements of `b`
/// are cloned and appended to `a`.
///
/// # Errors
///
/// This function returns an [`ApplyError::MismatchedKinds`] if `b` is not a list or if
/// applying elements to each other fails.
#[inline]
pub fn list_try_apply<L: List>(a: &mut L, b: &dyn PartialReflect) -> Result<(), ApplyError> {
    let list_value = b.reflect_ref().as_list()?;

    for (i, value) in list_value.iter().enumerate() {
        if i < a.len() {
            if let Some(v) = a.get_mut(i) {
                v.try_apply(value)?;
            }
        } else {
            List::push(a, value.to_dynamic());
        }
    }

    Ok(())
}

/// Compares a [`List`] with a [`Reflect`] value.
///
/// Returns true if and only if all of the following are true:
/// - `b` is a list;
/// - `b` is the same length as `a`;
/// - [`PartialReflect::reflect_partial_eq`] returns `Some(true)` for pairwise elements of `a` and `b`.
///
/// Returns [`None`] if the comparison couldn't even be performed.
#[inline]
pub fn list_partial_eq<L: List + ?Sized>(a: &L, b: &dyn PartialReflect) -> Option<bool> {
    let ReflectRef::List(list) = b.reflect_ref() else {
        return Some(false);
    };

    if a.len() != list.len() {
        return Some(false);
    }

    for (a_value, b_value) in a.iter().zip(list.iter()) {
        let eq_result = a_value.reflect_partial_eq(b_value);
        if let failed @ (Some(false) | None) = eq_result {
            return failed;
        }
    }

    Some(true)
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
    use super::DynamicList;
    use crate::Reflect;
    use alloc::{boxed::Box, vec};
    use core::assert_eq;

    #[test]
    fn test_into_iter() {
        let mut list = DynamicList::default();
        list.push(0usize);
        list.push(1usize);
        list.push(2usize);
        let items = list.into_iter();
        for (index, item) in items.into_iter().enumerate() {
            let value = item
                .try_take::<usize>()
                .expect("couldn't downcast to usize");
            assert_eq!(index, value);
        }
    }

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
