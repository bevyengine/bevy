use std::collections::HashSet;

/// Contains data relevant to the automatic reflect powered serialization of a type
#[derive(Debug, Clone)]
pub struct SerializationData {
    ignored_field_indices: HashSet<usize>,
}

impl SerializationData {
    /// Creates a new `SerializationData` instance given:
    ///
    /// - `ignored_iter`: the iterator of member indices to be ignored during serialization. Indices are assigned only to reflected members, those which are not reflected are skipped.
    pub fn new<I: Iterator<Item = usize>>(ignored_iter: I) -> Self {
        Self {
            ignored_field_indices: ignored_iter.collect(),
        }
    }
    /// Returns true if the given index corresponds to a field meant to be ignored in serialization.
    ///
    /// Indices start from 0 and ignored fields are skipped.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// for (idx, field) in my_struct.iter_fields().enumerate(){
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
