use crate::generics::impl_generic_info_methods;
use crate::{
    type_info::impl_type_methods, utility::reflect_hasher, ApplyError, Generics, MaybeTyped,
    PartialReflect, Reflect, ReflectKind, ReflectMut, ReflectOwned, ReflectRef, Type, TypeInfo,
    TypePath,
};
use alloc::{boxed::Box, vec::Vec};
use bevy_reflect_derive::impl_type_path;
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
/// use bevy_reflect::{PartialReflect, Array};
///
/// let foo: &dyn Array = &[123_u32, 456_u32, 789_u32];
/// assert_eq!(foo.len(), 3);
///
/// let field: &dyn PartialReflect = foo.get(0).unwrap();
/// assert_eq!(field.try_downcast_ref::<u32>(), Some(&123));
/// ```
///
/// [array-like]: https://doc.rust-lang.org/book/ch03-02-data-types.html#the-array-type
/// [reflection]: crate
/// [`List`]: crate::List
/// [type-erasing]: https://doc.rust-lang.org/book/ch17-02-trait-objects.html
/// [`GetTypeRegistration`]: crate::GetTypeRegistration
/// [limitation]: https://github.com/serde-rs/serde/issues/1937
/// [`Deserialize`]: ::serde::Deserialize
pub trait Array: PartialReflect {
    /// Returns a reference to the element at `index`, or `None` if out of bounds.
    fn get(&self, index: usize) -> Option<&dyn PartialReflect>;

    /// Returns a mutable reference to the element at `index`, or `None` if out of bounds.
    fn get_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect>;

    /// Returns the number of elements in the array.
    fn len(&self) -> usize;

    /// Returns `true` if the collection contains no elements.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over the array.
    fn iter(&self) -> ArrayIter<'_>;

    /// Drain the elements of this array to get a vector of owned values.
    fn drain(self: Box<Self>) -> Vec<Box<dyn PartialReflect>>;

    /// Creates a new [`DynamicArray`] from this array.
    fn to_dynamic_array(&self) -> DynamicArray {
        DynamicArray {
            represented_type: self.get_represented_type_info(),
            values: self.iter().map(PartialReflect::to_dynamic).collect(),
        }
    }

    /// Will return `None` if [`TypeInfo`] is not available.
    fn get_represented_array_info(&self) -> Option<&'static ArrayInfo> {
        self.get_represented_type_info()?.as_array().ok()
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
    #[cfg(feature = "documentation")]
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
            #[cfg(feature = "documentation")]
            docs: None,
        }
    }

    /// Sets the docstring for this array.
    #[cfg(feature = "documentation")]
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
    #[cfg(feature = "documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }

    impl_generic_info_methods!(generics);
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
#[derive(Debug)]
pub struct DynamicArray {
    pub(crate) represented_type: Option<&'static TypeInfo>,
    pub(crate) values: Box<[Box<dyn PartialReflect>]>,
}

impl DynamicArray {
    /// Creates a new [`DynamicArray`].
    #[inline]
    pub fn new(values: Box<[Box<dyn PartialReflect>]>) -> Self {
        Self {
            represented_type: None,
            values,
        }
    }

    /// Sets the [type] to be represented by this `DynamicArray`.
    ///
    /// # Panics
    ///
    /// Panics if the given [type] is not a [`TypeInfo::Array`].
    ///
    /// [type]: TypeInfo
    pub fn set_represented_type(&mut self, represented_type: Option<&'static TypeInfo>) {
        if let Some(represented_type) = represented_type {
            assert!(
                matches!(represented_type, TypeInfo::Array(_)),
                "expected TypeInfo::Array but received: {represented_type:?}"
            );
        }

        self.represented_type = represented_type;
    }
}

impl PartialReflect for DynamicArray {
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
        array_apply(self, value);
    }

    fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
        array_try_apply(self, value)
    }

    #[inline]
    fn reflect_kind(&self) -> ReflectKind {
        ReflectKind::Array
    }

    #[inline]
    fn reflect_ref(&self) -> ReflectRef<'_> {
        ReflectRef::Array(self)
    }

    #[inline]
    fn reflect_mut(&mut self) -> ReflectMut<'_> {
        ReflectMut::Array(self)
    }

    #[inline]
    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Array(self)
    }

    #[inline]
    fn reflect_hash(&self) -> Option<u64> {
        array_hash(self)
    }

    fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
        array_partial_eq(self, value)
    }

    fn debug(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "DynamicArray(")?;
        array_debug(self, f)?;
        write!(f, ")")
    }

    #[inline]
    fn is_dynamic(&self) -> bool {
        true
    }
}

impl Array for DynamicArray {
    #[inline]
    fn get(&self, index: usize) -> Option<&dyn PartialReflect> {
        self.values.get(index).map(|value| &**value)
    }

    #[inline]
    fn get_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        self.values.get_mut(index).map(|value| &mut **value)
    }

    #[inline]
    fn len(&self) -> usize {
        self.values.len()
    }

    #[inline]
    fn iter(&self) -> ArrayIter<'_> {
        ArrayIter::new(self)
    }

    #[inline]
    fn drain(self: Box<Self>) -> Vec<Box<dyn PartialReflect>> {
        self.values.into_vec()
    }
}

impl FromIterator<Box<dyn PartialReflect>> for DynamicArray {
    fn from_iter<I: IntoIterator<Item = Box<dyn PartialReflect>>>(values: I) -> Self {
        Self {
            represented_type: None,
            values: values.into_iter().collect::<Vec<_>>().into_boxed_slice(),
        }
    }
}

impl<T: PartialReflect> FromIterator<T> for DynamicArray {
    fn from_iter<I: IntoIterator<Item = T>>(values: I) -> Self {
        values
            .into_iter()
            .map(|value| Box::new(value).into_partial_reflect())
            .collect()
    }
}

impl IntoIterator for DynamicArray {
    type Item = Box<dyn PartialReflect>;
    type IntoIter = alloc::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_vec().into_iter()
    }
}

impl<'a> IntoIterator for &'a DynamicArray {
    type Item = &'a dyn PartialReflect;
    type IntoIter = ArrayIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl_type_path!((in bevy_reflect) DynamicArray);

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
    type Item = &'a dyn PartialReflect;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let value = self.array.get(self.index);
        self.index += value.is_some() as usize;
        value
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.array.len();
        (size, Some(size))
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

/// Applies the reflected [array](Array) data to the given [array](Array).
///
/// # Panics
///
/// * Panics if the two arrays have differing lengths.
/// * Panics if the reflected value is not a [valid array](ReflectRef::Array).
#[inline]
pub fn array_apply<A: Array + ?Sized>(array: &mut A, reflect: &dyn PartialReflect) {
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

/// Tries to apply the reflected [array](Array) data to the given [array](Array) and
/// returns a Result.
///
/// # Errors
///
/// * Returns an [`ApplyError::DifferentSize`] if the two arrays have differing lengths.
/// * Returns an [`ApplyError::MismatchedKinds`] if the reflected value is not a
///   [valid array](ReflectRef::Array).
/// * Returns any error that is generated while applying elements to each other.
#[inline]
pub fn array_try_apply<A: Array>(
    array: &mut A,
    reflect: &dyn PartialReflect,
) -> Result<(), ApplyError> {
    let reflect_array = reflect.reflect_ref().as_array()?;

    if array.len() != reflect_array.len() {
        return Err(ApplyError::DifferentSize {
            from_size: reflect_array.len(),
            to_size: array.len(),
        });
    }

    for (i, value) in reflect_array.iter().enumerate() {
        let v = array.get_mut(i).unwrap();
        v.try_apply(value)?;
    }

    Ok(())
}

/// Compares two [arrays](Array) (one concrete and one reflected) to see if they
/// are equal.
///
/// Returns [`None`] if the comparison couldn't even be performed.
#[inline]
pub fn array_partial_eq<A: Array + ?Sized>(
    array: &A,
    reflect: &dyn PartialReflect,
) -> Option<bool> {
    match reflect.reflect_ref() {
        ReflectRef::Array(reflect_array) if reflect_array.len() == array.len() => {
            for (a, b) in array.iter().zip(reflect_array.iter()) {
                let eq_result = a.reflect_partial_eq(b);
                if let failed @ (Some(false) | None) = eq_result {
                    return failed;
                }
            }
        }
        _ => return Some(false),
    }

    Some(true)
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
}
