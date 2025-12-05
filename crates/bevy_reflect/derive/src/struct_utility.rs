use crate::ReflectStruct;

/// A helper struct for creating remote-aware field accessors.
///
/// These are "remote-aware" because when a field is a remote field, it uses a [`transmute`] internally
/// to access the field.
///
/// [`transmute`]: std::mem::transmute
pub(crate) struct FieldAccessors {
    /// The referenced field accessors, such as `&self.foo`.
    pub fields_ref: Vec<proc_macro2::TokenStream>,
    /// The mutably referenced field accessors, such as `&mut self.foo`.
    pub fields_mut: Vec<proc_macro2::TokenStream>,
    /// The ordered set of field indices (basically just the range of [0, `field_count`).
    pub field_indices: Vec<usize>,
    /// The number of fields in the reflected struct.
    pub field_count: usize,
}

impl FieldAccessors {
    pub fn new(reflect_struct: &ReflectStruct) -> Self {
        let (fields_ref, fields_mut): (Vec<_>, Vec<_>) = reflect_struct
            .active_fields()
            .map(|field| {
                (
                    reflect_struct.access_for_field(field, false),
                    reflect_struct.access_for_field(field, true),
                )
            })
            .unzip();

        let field_count = fields_ref.len();
        let field_indices = (0..field_count).collect();

        Self {
            fields_ref,
            fields_mut,
            field_indices,
            field_count,
        }
    }
}
