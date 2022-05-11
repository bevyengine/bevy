use crate::{serde::Serializable, Reflect, ReflectMut, ReflectRef};
use serde::ser::SerializeSeq;
use std::{
    any::Any,
    hash::{Hash, Hasher},
};

/// A static-sized array of [`Reflect`] items.
///
/// This corresponds to types like `[T; N]` (arrays).
///
/// Currently, this only supports arrays of up to 32 items. It can technically
/// contain more than 32, but the blanket [`GetTypeRegistration`] is only
/// implemented up to the 32 item limit due to a [limitation] on `Deserialize`.
///
/// [`GetTypeRegistration`]: crate::GetTypeRegistration
/// [limitation]: https://github.com/serde-rs/serde/issues/1937
pub trait Array: Reflect {
    /// Returns a reference to the element at `index`, or `None` if out of bounds.
    fn get(&self, index: usize) -> Option<&dyn Reflect>;
    /// Returns a mutable reference to the element at `index`, or `None` if out of bounds.
    fn get_mut(&mut self, index: usize) -> Option<&mut dyn Reflect>;
    /// Returns the number of elements in the collection.
    fn len(&self) -> usize;
    /// Returns `true` if the collection contains no elements.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Returns an iterator over the collection.
    fn iter(&self) -> ArrayIter;

    fn clone_dynamic(&self) -> DynamicArray {
        DynamicArray {
            name: self.type_name().to_string(),
            values: self.iter().map(|value| value.clone_value()).collect(),
        }
    }
}

/// A fixed-size list of reflected values.
///
/// This differs from [`DynamicList`] in that the size of the [`DynamicArray`]
/// is constant, whereas a [`DynamicList`] can have items added and removed.
///
/// This isn't to say that a [`DynamicArray`] is immutable— its items
/// can be mutated— just that the _number_ of items cannot change.
///
/// [`DynamicList`]: crate::DynamicList
pub struct DynamicArray {
    pub(crate) name: String,
    pub(crate) values: Box<[Box<dyn Reflect>]>,
}

impl DynamicArray {
    #[inline]
    pub fn new(values: Box<[Box<dyn Reflect>]>) -> Self {
        Self {
            name: String::default(),
            values,
        }
    }

    pub fn from_vec<T: Reflect>(values: Vec<T>) -> Self {
        Self {
            name: String::default(),
            values: values
                .into_iter()
                .map(|field| Box::new(field) as Box<dyn Reflect>)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        }
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

// SAFE: any and any_mut both return self
unsafe impl Reflect for DynamicArray {
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

    #[inline]
    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    #[inline]
    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }

    fn apply(&mut self, value: &dyn Reflect) {
        array_apply(self, value);
    }

    #[inline]
    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
    }

    #[inline]
    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::Array(self)
    }

    #[inline]
    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::Array(self)
    }

    #[inline]
    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(self.clone_dynamic())
    }

    #[inline]
    fn reflect_hash(&self) -> Option<u64> {
        array_hash(self)
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        array_partial_eq(self, value)
    }

    fn serializable(&self) -> Option<Serializable> {
        Some(Serializable::Borrowed(self))
    }
}

impl Array for DynamicArray {
    #[inline]
    fn get(&self, index: usize) -> Option<&dyn Reflect> {
        self.values.get(index).map(|value| &**value)
    }

    #[inline]
    fn get_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        self.values.get_mut(index).map(|value| &mut **value)
    }

    #[inline]
    fn len(&self) -> usize {
        self.values.len()
    }

    #[inline]
    fn iter(&self) -> ArrayIter {
        ArrayIter {
            array: self,
            index: 0,
        }
    }

    #[inline]
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

/// An iterator over an [`Array`].
pub struct ArrayIter<'a> {
    pub(crate) array: &'a dyn Array,
    pub(crate) index: usize,
}

impl<'a> Iterator for ArrayIter<'a> {
    type Item = &'a dyn Reflect;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let value = self.array.get(self.index);
        self.index += 1;
        value
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.array.len();
        (size, Some(size))
    }
}

impl<'a> ExactSizeIterator for ArrayIter<'a> {}

impl<'a> serde::Serialize for dyn Array {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        array_serialize(self, serializer)
    }
}

impl serde::Serialize for DynamicArray {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        array_serialize(self, serializer)
    }
}

/// Serializes the given [array](Array).
#[inline]
pub fn array_serialize<A: Array + ?Sized, S>(array: &A, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut seq = serializer.serialize_seq(Some(array.len()))?;
    for element in array.iter() {
        let serializable = element.serializable().ok_or_else(|| {
            serde::ser::Error::custom(format!(
                "Type '{}' does not support `Reflect` serialization",
                element.type_name()
            ))
        })?;
        seq.serialize_element(serializable.borrow())?;
    }
    seq.end()
}

/// Returns the `u64` hash of the given [array](Array).
#[inline]
pub fn array_hash<A: Array>(array: &A) -> Option<u64> {
    let mut hasher = crate::ReflectHasher::default();
    std::any::Any::type_id(array).hash(&mut hasher);
    array.len().hash(&mut hasher);
    for value in array.iter() {
        hasher.write_u64(value.reflect_hash()?)
    }
    Some(hasher.finish())
}

/// Applies the reflected [array](Array) data to the given [array](Array).
///
/// # Panics
///
/// * Panics if the two arrays have differing lengths.
/// * Panics if the reflected value is not a [valid array](ReflectRef::Array).
///
#[inline]
pub fn array_apply<A: Array>(array: &mut A, reflect: &dyn Reflect) {
    if let ReflectRef::Array(reflect_array) = reflect.reflect_ref() {
        if array.len() != reflect_array.len() {
            panic!("Attempted to apply different sized `Array` types.");
        }
        for (i, value) in reflect_array.iter().enumerate() {
            let v = array.get_mut(i).unwrap();
            v.apply(value);
        }
    } else {
        panic!("Attempted to apply a non-`Array` type to an `Array` type.");
    }
}

/// Compares two [arrays](Array) (one concrete and one reflected) to see if they
/// are equal.
#[inline]
pub fn array_partial_eq<A: Array>(array: &A, reflect: &dyn Reflect) -> Option<bool> {
    match reflect.reflect_ref() {
        ReflectRef::Array(reflect_array) if reflect_array.len() == array.len() => {
            for (a, b) in array.iter().zip(reflect_array.iter()) {
                if let Some(false) | None = a.reflect_partial_eq(b) {
                    return Some(false);
                }
            }
        }
        _ => return Some(false),
    }

    Some(true)
}
