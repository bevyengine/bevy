use std::any::{Any, TypeId};
use std::fmt::{Debug, Formatter};

use crate::utility::NonGenericTypeInfoCell;
use crate::{
    Array, ArrayIter, DynamicArray, DynamicInfo, FromReflect, Reflect, ReflectMut, ReflectOwned,
    ReflectRef, TypeInfo, Typed,
};

/// An ordered, mutable list of [Reflect] items. This corresponds to types like [`std::vec::Vec`].
///
/// This is a sub-trait of [`Array`] as it implements a [`push`](List::push) function, allowing
/// it's internal size to grow.
pub trait List: Reflect + Array {
    /// Appends an element to the list.
    fn push(&mut self, value: Box<dyn Reflect>);

    /// Removes the last element from the list (highest index in the array) and returns it, or [`None`] if it is empty.
    fn pop(&mut self) -> Option<Box<dyn Reflect>>;

    /// Clones the list, producing a [`DynamicList`].
    fn clone_dynamic(&self) -> DynamicList {
        DynamicList {
            name: self.type_name().to_string(),
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

impl Array for DynamicList {
    fn get(&self, index: usize) -> Option<&dyn Reflect> {
        self.values.get(index).map(|value| &**value)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        self.values.get_mut(index).map(|value| &mut **value)
    }

    fn len(&self) -> usize {
        self.values.len()
    }

    fn iter(&self) -> ArrayIter {
        ArrayIter {
            array: self,
            index: 0,
        }
    }

    fn drain(self: Box<Self>) -> Vec<Box<dyn Reflect>> {
        self.values
    }

    fn clone_dynamic(&self) -> DynamicArray {
        DynamicArray {
            name: self.name.clone(),
            values: self
                .values
                .iter()
                .map(|value| value.clone_value())
                .collect(),
        }
    }
}

impl List for DynamicList {
    fn push(&mut self, value: Box<dyn Reflect>) {
        DynamicList::push_box(self, value);
    }

    fn pop(&mut self) -> Option<Box<dyn Reflect>> {
        self.values.pop()
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
}

impl Reflect for DynamicList {
    #[inline]
    fn type_name(&self) -> &str {
        self.name.as_str()
    }

    #[inline]
    fn get_type_info(&self) -> &'static TypeInfo {
        <Self as Typed>::type_info()
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
        Box::new(List::clone_dynamic(self))
    }

    #[inline]
    fn reflect_hash(&self) -> Option<u64> {
        crate::array_hash(self)
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        list_partial_eq(self, value)
    }

    fn debug(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "DynamicList(")?;
        list_debug(self, f)?;
        write!(f, ")")
    }
}

impl Debug for DynamicList {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.debug(f)
    }
}

impl Typed for DynamicList {
    fn type_info() -> &'static TypeInfo {
        static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
        CELL.get_or_set(|| TypeInfo::Dynamic(DynamicInfo::new::<Self>()))
    }
}

impl IntoIterator for DynamicList {
    type Item = Box<dyn Reflect>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
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
