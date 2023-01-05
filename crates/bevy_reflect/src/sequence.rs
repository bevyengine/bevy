use crate::{
    utility::NonGenericTypeInfoCell, DynamicInfo, Reflect, ReflectMut, ReflectOwned, ReflectRef,
    TypeInfo, Typed,
};
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    hash::{Hash, Hasher},
};

/// A static-sized sequence of [`Reflect`] items.
///
/// This corresponds to types like `[T; N]` (sequences).
///
/// Currently, this only supports sequences of up to 32 items. It can technically
/// contain more than 32, but the blanket [`GetTypeRegistration`] is only
/// implemented up to the 32 item limit due to a [limitation] on `Deserialize`.
///
/// [`GetTypeRegistration`]: crate::GetTypeRegistration
/// [limitation]: https://github.com/serde-rs/serde/issues/1937
pub trait Sequence: Reflect {
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
    fn iter(&self) -> SequenceIter;
    /// Drain the elements of this sequence to get a vector of owned values.
    fn drain(self: Box<Self>) -> Vec<Box<dyn Reflect>>;

    fn clone_dynamic(&self) -> DynamicSequence {
        DynamicSequence {
            name: self.type_name().to_string(),
            values: self.iter().map(|value| value.clone_value()).collect(),
        }
    }
}

/// A container for compile-time sequence info.
#[derive(Clone, Debug)]
pub struct SequenceInfo {
    type_name: &'static str,
    type_id: TypeId,
    item_type_name: &'static str,
    item_type_id: TypeId,
    capacity: usize,
    #[cfg(feature = "documentation")]
    docs: Option<&'static str>,
}

impl SequenceInfo {
    /// Create a new [`SequenceInfo`].
    ///
    /// # Arguments
    ///
    /// * `capacity`: The maximum capacity of the underlying sequence.
    ///
    pub fn new<TSequence: Sequence, TItem: Reflect>(capacity: usize) -> Self {
        Self {
            type_name: std::any::type_name::<TSequence>(),
            type_id: TypeId::of::<TSequence>(),
            item_type_name: std::any::type_name::<TItem>(),
            item_type_id: TypeId::of::<TItem>(),
            capacity,
            #[cfg(feature = "documentation")]
            docs: None,
        }
    }

    /// Sets the docstring for this sequence.
    #[cfg(feature = "documentation")]
    pub fn with_docs(self, docs: Option<&'static str>) -> Self {
        Self { docs, ..self }
    }

    /// The compile-time capacity of the sequence.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// The [type name] of the sequence.
    ///
    /// [type name]: std::any::type_name
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// The [`TypeId`] of the sequence.
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Check if the given type matches the sequence type.
    pub fn is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }

    /// The [type name] of the sequence item.
    ///
    /// [type name]: std::any::type_name
    pub fn item_type_name(&self) -> &'static str {
        self.item_type_name
    }

    /// The [`TypeId`] of the sequence item.
    pub fn item_type_id(&self) -> TypeId {
        self.item_type_id
    }

    /// Check if the given type matches the sequence item type.
    pub fn item_is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.item_type_id
    }

    /// The docstring of this sequence, if any.
    #[cfg(feature = "documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }
}

/// A fixed-size list of reflected values.
///
/// This differs from [`DynamicList`] in that the size of the [`DynamicSequence`]
/// is constant, whereas a [`DynamicList`] can have items added and removed.
///
/// This isn't to say that a [`DynamicSequence`] is immutable— its items
/// can be mutated— just that the _number_ of items cannot change.
///
/// [`DynamicList`]: crate::DynamicList
#[derive(Debug)]
pub struct DynamicSequence {
    pub(crate) name: String,
    pub(crate) values: Box<[Box<dyn Reflect>]>,
}

impl DynamicSequence {
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

impl Reflect for DynamicSequence {
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
        sequence_apply(self, value);
    }

    #[inline]
    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
    }

    #[inline]
    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::Sequence(self)
    }

    #[inline]
    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::Sequence(self)
    }

    #[inline]
    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Sequence(self)
    }

    #[inline]
    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(self.clone_dynamic())
    }

    #[inline]
    fn reflect_hash(&self) -> Option<u64> {
        sequence_hash(self)
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        sequence_partial_eq(self, value)
    }
}

impl Sequence for DynamicSequence {
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
    fn iter(&self) -> SequenceIter {
        SequenceIter {
            sequence: self,
            index: 0,
        }
    }

    #[inline]
    fn drain(self: Box<Self>) -> Vec<Box<dyn Reflect>> {
        self.values.into_vec()
    }

    #[inline]
    fn clone_dynamic(&self) -> DynamicSequence {
        DynamicSequence {
            name: self.name.clone(),
            values: self
                .values
                .iter()
                .map(|value| value.clone_value())
                .collect(),
        }
    }
}

impl Typed for DynamicSequence {
    fn type_info() -> &'static TypeInfo {
        static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
        CELL.get_or_set(|| TypeInfo::Dynamic(DynamicInfo::new::<Self>()))
    }
}

/// An iterator over a [`Sequence`].
pub struct SequenceIter<'a> {
    pub(crate) sequence: &'a dyn Sequence,
    pub(crate) index: usize,
}

impl<'a> Iterator for SequenceIter<'a> {
    type Item = &'a dyn Reflect;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let value = self.sequence.get(self.index);
        self.index += 1;
        value
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.sequence.len();
        (size, Some(size))
    }
}

impl<'a> ExactSizeIterator for SequenceIter<'a> {}

/// Returns the `u64` hash of the given [sequence](Sequence).
#[inline]
pub fn sequence_hash<S: Sequence>(sequence: &S) -> Option<u64> {
    let mut hasher = crate::ReflectHasher::default();
    std::any::Any::type_id(sequence).hash(&mut hasher);
    sequence.len().hash(&mut hasher);
    for value in sequence.iter() {
        hasher.write_u64(value.reflect_hash()?);
    }
    Some(hasher.finish())
}

/// Applies the reflected [sequence](Sequence) data to the given [sequence](Sequence).
///
/// # Panics
///
/// * Panics if the two sequences have differing lengths.
/// * Panics if the reflected value is not a [valid sequence](ReflectRef::Sequence).
///
#[inline]
pub fn sequence_apply<S: Sequence>(sequence: &mut S, reflect: &dyn Reflect) {
    if let ReflectRef::Sequence(reflect_sequence) = reflect.reflect_ref() {
        if sequence.len() != reflect_sequence.len() {
            panic!("Attempted to apply different sized `Sequence` types.");
        }
        for (i, value) in reflect_sequence.iter().enumerate() {
            let v = sequence.get_mut(i).unwrap();
            v.apply(value);
        }
    } else {
        panic!("Attempted to apply a non-`Sequence` type to an `Sequence` type.");
    }
}

/// Compares two [sequences](Sequence) (one concrete and one reflected) to see if they
/// are equal.
///
/// Returns [`None`] if the comparison couldn't even be performed.
#[inline]
pub fn sequence_partial_eq<S: Sequence>(sequence: &S, reflect: &dyn Reflect) -> Option<bool> {
    match reflect.reflect_ref() {
        ReflectRef::Sequence(reflect_sequence) if reflect_sequence.len() == sequence.len() => {
            for (a, b) in sequence.iter().zip(reflect_sequence.iter()) {
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

/// The default debug formatter for [`Sequence`] types.
///
/// # Example
/// ```
/// use bevy_reflect::Reflect;
///
/// let my_sequence: &dyn Reflect = &[1, 2, 3];
/// println!("{:#?}", my_sequence);
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
pub fn sequence_debug(
    dyn_sequence: &dyn Sequence,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    let mut debug = f.debug_list();
    for item in dyn_sequence.iter() {
        debug.entry(&item as &dyn Debug);
    }
    debug.finish()
}
