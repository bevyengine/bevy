use bevy_utils::HashMap;
use std::borrow::Cow;
use std::collections::HashSet;

/// Contains data relevant to the automatic reflect powered serialization of a type.
#[derive(Debug, Clone)]
pub enum SerializationData {
    Struct(StructSerializationData),
    Enum(EnumSerializationData),
}

/// Contains data relevant to the automatic reflect powered serialization of a struct or tuple struct.
#[derive(Debug, Clone)]
pub struct StructSerializationData {
    ignored_field_indices: HashSet<usize>,
}

impl StructSerializationData {
    /// Creates a new `StructSerializationData` instance given:
    ///
    /// - `ignored_fields`: the iterator of member indices to be ignored during serialization.
    /// Indices are assigned only to reflected members, those which are not reflected are skipped.
    pub fn new<I: Iterator<Item = usize>>(ignored_fields: I) -> Self {
        Self {
            ignored_field_indices: ignored_fields.collect(),
        }
    }

    /// Returns true if the given index corresponds to a field meant to be ignored in serialization.
    ///
    /// Indices start from 0 and ignored fields are skipped.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// for (idx, field) in my_struct.iter_fields().enumerate() {
    ///     if serialization_data.is_ignored_field(idx){
    ///        // serialize ...
    ///     }
    /// }
    /// ```
    pub fn is_ignored_field(&self, index: usize) -> bool {
        self.ignored_field_indices.contains(&index)
    }

    /// Returns the number of ignored fields.
    pub fn len(&self) -> usize {
        self.ignored_field_indices.len()
    }

    /// Returns true if there are no ignored fields.
    pub fn is_empty(&self) -> bool {
        self.ignored_field_indices.is_empty()
    }
}

/// Contains data relevant to the automatic reflect powered serialization of an enum.
#[derive(Debug, Clone)]
pub struct EnumSerializationData {
    variant_serialization_data: HashMap<Cow<'static, str>, HashSet<usize>>,
}

impl EnumSerializationData {
    /// Creates a new `EnumSerializationData` instance given:
    ///
    /// - `ignored_variants`: the iterator of member variant-indices pairs to be ignored during serialization.
    /// Indices are assigned only to reflected members, those which are not reflected are skipped.
    pub fn new<TName, TFields, TVariant>(ignored_variants: TVariant) -> Self
    where
        TName: Into<Cow<'static, str>>,
        TFields: Iterator<Item = usize>,
        TVariant: Iterator<Item = (TName, TFields)>,
    {
        let mut data = HashMap::new();
        for (name, fields) in ignored_variants {
            data.insert(name.into(), fields.collect());
        }
        Self {
            variant_serialization_data: data,
        }
    }
    /// Returns true if the given index corresponds to a field meant to be ignored in serialization.
    ///
    /// Indices start from 0 and ignored fields are skipped.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// for (idx, field) in my_variant.iter_fields().enumerate() {
    ///     if serialization_data.is_ignored_field(field.name(), idx){
    ///        // serialize ...
    ///     }
    /// }
    /// ```
    ///
    /// This will return `None` if the variant does not exist.
    pub fn is_ignored_field(&self, variant: &str, index: usize) -> Option<bool> {
        Some(
            self.variant_serialization_data
                .get(variant)?
                .contains(&index),
        )
    }

    /// Returns the number of ignored fields for the given variant.
    ///
    /// This will return `None` if the variant does not exist.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self, variant: &str) -> Option<usize> {
        Some(self.variant_serialization_data.get(variant)?.len())
    }

    /// Returns true if there are no ignored fields for the given variant.
    ///
    /// This will return `None` if the variant does not exist.
    pub fn is_empty(&self, variant: &str) -> Option<bool> {
        Some(self.variant_serialization_data.get(variant)?.is_empty())
    }
}
