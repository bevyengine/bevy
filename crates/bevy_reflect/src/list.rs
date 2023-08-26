use std::any::{Any, TypeId};
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};

use bevy_reflect_derive::impl_type_path;

use crate::utility::reflect_hasher;
use crate::{
    self as bevy_reflect, FromReflect, Reflect, ReflectMut, ReflectOwned, ReflectRef, TypeInfo,
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
/// use bevy_reflect::{Reflect, List};
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
/// [`Vec`]: std::vec::Vec
/// [type-erasing]: https://doc.rust-lang.org/book/ch17-02-trait-objects.html
pub trait List: Reflect {
    /// Returns a reference to the element at `index`, or `None` if out of bounds.
    fn get(&self, index: usize) -> Option<&dyn Reflect>;

    /// Returns a mutable reference to the element at `index`, or `None` if out of bounds.
    fn get_mut(&mut self, index: usize) -> Option<&mut dyn Reflect>;

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
    fn iter(&self) -> ListIter;

    /// Drain the elements of this list to get a vector of owned values.
    fn drain(self: Box<Self>) -> Vec<Box<dyn Reflect>>;

    /// Clones the list, producing a [`DynamicList`].
    fn clone_dynamic(&self) -> DynamicList {
        DynamicList {
            represented_type: self.get_represented_type_info(),
            values: self.iter().map(|value| value.clone_value()).collect(),
        }
    }
}

/// A container for compile-time list info.
#[derive(Clone, Debug)]
pub struct ListInfo {
    type_name: &'static str,
    type_id: TypeId,
    item_type_name: &'static str,
    item_type_id: TypeId,
    #[cfg(feature = "documentation")]
    docs: Option<&'static str>,
}

impl ListInfo {
    /// Create a new [`ListInfo`].
    pub fn new<TList: List, TItem: FromReflect>() -> Self {
        Self {
            type_name: std::any::type_name::<TList>(),
            type_id: TypeId::of::<TList>(),
            item_type_name: std::any::type_name::<TItem>(),
            item_type_id: TypeId::of::<TItem>(),
            #[cfg(feature = "documentation")]
            docs: None,
        }
    }

    /// Sets the docstring for this list.
    #[cfg(feature = "documentation")]
    pub fn with_docs(self, docs: Option<&'static str>) -> Self {
        Self { docs, ..self }
    }

    /// The [type name] of the list.
    ///
    /// [type name]: std::any::type_name
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// The [`TypeId`] of the list.
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Check if the given type matches the list type.
    pub fn is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }

    /// The [type name] of the list item.
    ///
    /// [type name]: std::any::type_name
    pub fn item_type_name(&self) -> &'static str {
        self.item_type_name
    }

    /// The [`TypeId`] of the list item.
    pub fn item_type_id(&self) -> TypeId {
        self.item_type_id
    }

    /// Check if the given type matches the list item type.
    pub fn item_is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.item_type_id
    }

    /// The docstring of this list, if any.
    #[cfg(feature = "documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }
}

/// A list of reflected values.
#[derive(Default)]
pub struct DynamicList {
    represented_type: Option<&'static TypeInfo>,
    values: Vec<Box<dyn Reflect>>,
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
                "expected TypeInfo::List but received: {:?}",
                represented_type
            );
        }

        self.represented_type = represented_type;
    }

    /// Appends a typed value to the list.
    pub fn push<T: Reflect>(&mut self, value: T) {
        self.values.push(Box::new(value));
    }

    /// Appends a [`Reflect`] trait object to the list.
    pub fn push_box(&mut self, value: Box<dyn Reflect>) {
        self.values.push(value);
    }
}

impl List for DynamicList {
    fn get(&self, index: usize) -> Option<&dyn Reflect> {
        self.values.get(index).map(|value| &**value)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        self.values.get_mut(index).map(|value| &mut **value)
    }

    fn insert(&mut self, index: usize, element: Box<dyn Reflect>) {
        self.values.insert(index, element);
    }

    fn remove(&mut self, index: usize) -> Box<dyn Reflect> {
        self.values.remove(index)
    }

    fn push(&mut self, value: Box<dyn Reflect>) {
        DynamicList::push_box(self, value);
    }

    fn pop(&mut self) -> Option<Box<dyn Reflect>> {
        self.values.pop()
    }

    fn len(&self) -> usize {
        self.values.len()
    }

    fn iter(&self) -> ListIter {
        ListIter::new(self)
    }

    fn drain(self: Box<Self>) -> Vec<Box<dyn Reflect>> {
        self.values
    }

    fn clone_dynamic(&self) -> DynamicList {
        DynamicList {
            represented_type: self.represented_type,
            values: self
                .values
                .iter()
                .map(|value| value.clone_value())
                .collect(),
        }
    }
}

impl Reflect for DynamicList {
    #[inline]
    fn type_name(&self) -> &str {
        self.represented_type
            .map(|info| info.type_name())
            .unwrap_or_else(|| std::any::type_name::<Self>())
    }

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

    fn apply(&mut self, value: &dyn Reflect) {
        list_apply(self, value);
    }

    #[inline]
    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
    }

    #[inline]
    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::List(self)
    }

    #[inline]
    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::List(self)
    }

    #[inline]
    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::List(self)
    }

    #[inline]
    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(self.clone_dynamic())
    }

    #[inline]
    fn reflect_hash(&self) -> Option<u64> {
        list_hash(self)
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        list_partial_eq(self, value)
    }

    fn debug(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.debug(f)
    }
}

impl IntoIterator for DynamicList {
    type Item = Box<dyn Reflect>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

/// An iterator over an [`List`].
pub struct ListIter<'a> {
    list: &'a dyn List,
    index: usize,
}

impl<'a> ListIter<'a> {
    /// Creates a new [`ListIter`].
    #[inline]
    pub const fn new(list: &'a dyn List) -> ListIter {
        ListIter { list, index: 0 }
    }
}

impl<'a> Iterator for ListIter<'a> {
    type Item = &'a dyn Reflect;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let value = self.list.get(self.index);
        self.index += 1;
        value
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.list.len();
        (size, Some(size))
    }
}

impl<'a> ExactSizeIterator for ListIter<'a> {}

/// Returns the `u64` hash of the given [list](List).
#[inline]
pub fn list_hash<L: List>(list: &L) -> Option<u64> {
    let mut hasher = reflect_hasher();
    std::any::Any::type_id(list).hash(&mut hasher);
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
pub fn list_apply<L: List>(a: &mut L, b: &dyn Reflect) {
    if let ReflectRef::List(list_value) = b.reflect_ref() {
        for (i, value) in list_value.iter().enumerate() {
            if i < a.len() {
                if let Some(v) = a.get_mut(i) {
                    v.apply(value);
                }
            } else {
                a.push(value.clone_value());
            }
        }
    } else {
        panic!("Attempted to apply a non-list type to a list type.");
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
#[inline]
pub fn list_partial_eq<L: List>(a: &L, b: &dyn Reflect) -> Option<bool> {
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
pub fn list_debug(dyn_list: &dyn List, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let mut debug = f.debug_list();
    for item in dyn_list.iter() {
        debug.entry(&item as &dyn Debug);
    }
    debug.finish()
}

#[cfg(test)]
mod tests {
    use super::DynamicList;
    use std::assert_eq;

    #[test]
    fn test_into_iter() {
        let mut list = DynamicList::default();
        list.push(0usize);
        list.push(1usize);
        list.push(2usize);
        let items = list.into_iter();
        for (index, item) in items.into_iter().enumerate() {
            let value = item.take::<usize>().expect("couldn't downcast to usize");
            assert_eq!(index, value);
        }
    }
}
