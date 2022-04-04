use std::any::Any;

use crate::{serde::Serializable, Reflect, ReflectMut, ReflectRef};

/// An ordered, mutable list of [Reflect] items. This corresponds to types like [`std::vec::Vec`].
pub trait List: Reflect {
    /// Returns a reference to the element at `index`, or `None` if out of bounds.
    fn get(&self, index: usize) -> Option<&dyn Reflect>;

    /// Returns a mutable reference to the element at `index`, or `None` if out of bounds.
    fn get_mut(&mut self, index: usize) -> Option<&mut dyn Reflect>;

    /// Appends an element to the list.
    fn push(&mut self, value: Box<dyn Reflect>);

    /// Returns the number of elements in the list.
    fn len(&self) -> usize;

    /// Returns `true` if the list contains no elements.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over the list.
    fn iter(&self) -> ListIter;

    /// Clones the list, producing a [`DynamicList`].
    fn clone_dynamic(&self) -> DynamicList {
        DynamicList {
            name: self.type_name().to_string(),
            values: self.iter().map(|value| value.clone_value()).collect(),
        }
    }
}

/// A list of reflected values.
#[derive(Default)]
pub struct DynamicList {
    name: String,
    values: Vec<Box<dyn Reflect>>,
}

impl DynamicList {
    /// Returns the type name of the list.
    ///
    /// The value returned by this method is the same value returned by
    /// [`Reflect::type_name`].
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the type name of the list.
    ///
    /// The value set by this method is the value returned by
    /// [`Reflect::type_name`].
    pub fn set_name(&mut self, name: String) {
        self.name = name;
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

    fn len(&self) -> usize {
        self.values.len()
    }

    fn clone_dynamic(&self) -> DynamicList {
        DynamicList {
            name: self.name.clone(),
            values: self
                .values
                .iter()
                .map(|value| value.clone_value())
                .collect(),
        }
    }

    fn iter(&self) -> ListIter {
        ListIter {
            list: self,
            index: 0,
        }
    }

    fn push(&mut self, value: Box<dyn Reflect>) {
        DynamicList::push_box(self, value);
    }
}

// SAFE: any and any_mut both return self
unsafe impl Reflect for DynamicList {
    #[inline]
    fn type_name(&self) -> &str {
        self.name.as_str()
    }

    #[inline]
    fn any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn any_mut(&mut self) -> &mut dyn Any {
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
    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(self.clone_dynamic())
    }

    #[inline]
    fn reflect_hash(&self) -> Option<u64> {
        None
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        list_partial_eq(self, value)
    }

    fn serializable(&self) -> Option<Serializable> {
        None
    }
}

/// An iterator over the elements of a [`List`].
pub struct ListIter<'a> {
    pub(crate) list: &'a dyn List,
    pub(crate) index: usize,
}

impl<'a> Iterator for ListIter<'a> {
    type Item = &'a dyn Reflect;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.list.get(self.index);
        self.index += 1;
        value
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.list.len();
        (size, Some(size))
    }
}

impl<'a> ExactSizeIterator for ListIter<'a> {}

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
                List::push(a, value.clone_value());
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
#[inline]
pub fn list_partial_eq<L: List>(a: &L, b: &dyn Reflect) -> Option<bool> {
    let list = if let ReflectRef::List(list) = b.reflect_ref() {
        list
    } else {
        return Some(false);
    };

    if a.len() != list.len() {
        return Some(false);
    }

    for (a_value, b_value) in a.iter().zip(list.iter()) {
        if let Some(false) | None = a_value.reflect_partial_eq(b_value) {
            return Some(false);
        }
    }

    Some(true)
}
