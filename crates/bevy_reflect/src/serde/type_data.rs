use crate::Reflect;
use bevy_utils::hashbrown::hash_map::Iter;
use bevy_utils::HashMap;

/// Contains data relevant to the automatic reflect powered (de)serialization of a type.
#[derive(Debug, Clone)]
pub struct SerializationData {
    skipped_fields: HashMap<usize, SkippedField>,
}

impl SerializationData {
    /// Creates a new `SerializationData` instance with the given skipped fields.
    ///
    /// # Arguments
    ///
    /// * `skipped_iter`: The iterator of field indices to be skipped during (de)serialization.
    ///                   Indices are assigned only to reflected fields.
    ///                   Ignored fields (i.e. those marked `#[reflect(ignore)]`) are implicitly skipped
    ///                   and do not need to be included in this iterator.
    pub fn new<I: Iterator<Item = (usize, SkippedField)>>(skipped_iter: I) -> Self {
        Self {
            skipped_fields: skipped_iter.collect(),
        }
    }
    /// Returns true if the given index corresponds to a field meant to be skipped during (de)serialization.
    ///
    /// # Example
    ///
    /// ```
    /// # use std::any::TypeId;
    /// # use bevy_reflect::{Reflect, Struct, TypeRegistry, serde::SerializationData};
    /// #[derive(Reflect)]
    /// struct MyStruct {
    ///   serialize_me: i32,
    ///   #[reflect(skip_serializing)]
    ///   skip_me: i32
    /// }
    ///
    /// let mut registry = TypeRegistry::new();
    /// registry.register::<MyStruct>();
    ///
    /// let my_struct = MyStruct {
    ///   serialize_me: 123,
    ///   skip_me: 321,
    /// };
    ///
    /// let serialization_data = registry.get_type_data::<SerializationData>(TypeId::of::<MyStruct>()).unwrap();
    ///
    /// for (idx, field) in my_struct.iter_fields().enumerate(){
    ///   if serialization_data.is_field_skipped(idx) {
    ///     // Skipped!
    ///     assert_eq!(1, idx);
    ///   } else {
    ///     // Not Skipped!
    ///     assert_eq!(0, idx);
    ///   }
    /// }
    /// ```
    pub fn is_field_skipped(&self, index: usize) -> bool {
        self.skipped_fields.contains_key(&index)
    }

    /// Generates a default instance of the skipped field at the given index.
    ///
    /// Returns `None` if the field is not skipped.
    ///
    /// # Example
    ///
    /// ```
    /// # use std::any::TypeId;
    /// # use bevy_reflect::{Reflect, Struct, TypeRegistry, serde::SerializationData};
    /// #[derive(Reflect)]
    /// struct MyStruct {
    ///   serialize_me: i32,
    ///   #[reflect(skip_serializing)]
    ///   #[reflect(default = "skip_me_default")]
    ///   skip_me: i32
    /// }
    ///
    /// fn skip_me_default() -> i32 {
    ///   789
    /// }
    ///
    /// let mut registry = TypeRegistry::new();
    /// registry.register::<MyStruct>();
    ///
    /// let serialization_data = registry.get_type_data::<SerializationData>(TypeId::of::<MyStruct>()).unwrap();
    /// assert_eq!(789, serialization_data.generate_default(1).unwrap().take::<i32>().unwrap());
    /// ```
    pub fn generate_default(&self, index: usize) -> Option<Box<dyn Reflect>> {
        self.skipped_fields
            .get(&index)
            .map(|field| field.generate_default())
    }

    /// Returns the number of skipped fields.
    pub fn len(&self) -> usize {
        self.skipped_fields.len()
    }

    /// Returns true if there are no skipped fields.
    pub fn is_empty(&self) -> bool {
        self.skipped_fields.is_empty()
    }

    /// Returns an iterator over the skipped fields.
    ///
    /// Each item in the iterator is a tuple containing:
    /// 1. The reflected index of the field
    /// 2. The (de)serialization metadata of the field  
    pub fn iter_skipped(&self) -> Iter<'_, usize, SkippedField> {
        self.skipped_fields.iter()
    }
}

/// Data needed for (de)serialization of a skipped field.
#[derive(Debug, Clone)]
pub struct SkippedField {
    default_fn: fn() -> Box<dyn Reflect>,
}

impl SkippedField {
    /// Create a new `SkippedField`.
    ///
    /// # Arguments
    ///
    /// * `default_fn`: A function pointer used to generate a default instance of the field.
    pub fn new(default_fn: fn() -> Box<dyn Reflect>) -> Self {
        Self { default_fn }
    }

    /// Generates a default instance of the field.
    pub fn generate_default(&self) -> Box<dyn Reflect> {
        (self.default_fn)()
    }
}
