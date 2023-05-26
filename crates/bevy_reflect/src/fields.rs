use crate::Reflect;
use std::any::{Any, TypeId};

/// The named field of a reflected struct.
#[derive(Clone, Debug)]
pub struct NamedField {
    name: &'static str,
    type_name: &'static str,
    type_id: TypeId,
    meta: FieldMeta,
}

impl NamedField {
    /// Create a new [`NamedField`].
    pub fn new<T: Reflect>(name: &'static str) -> Self {
        Self {
            name,
            type_name: std::any::type_name::<T>(),
            type_id: TypeId::of::<T>(),
            meta: FieldMeta::new(),
        }
    }

    /// Add metadata for this field.
    pub fn with_meta(self, meta: FieldMeta) -> Self {
        Self { meta, ..self }
    }

    /// The name of the field.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The [type name] of the field.
    ///
    /// [type name]: std::any::type_name
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// The [`TypeId`] of the field.
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// The metadata of the field.
    pub fn meta(&self) -> &FieldMeta {
        &self.meta
    }

    /// Check if the given type matches the field type.
    pub fn is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }
}

/// The unnamed field of a reflected tuple or tuple struct.
#[derive(Clone, Debug)]
pub struct UnnamedField {
    index: usize,
    type_name: &'static str,
    type_id: TypeId,
    meta: FieldMeta,
}

impl UnnamedField {
    pub fn new<T: Reflect>(index: usize) -> Self {
        Self {
            index,
            type_name: std::any::type_name::<T>(),
            type_id: TypeId::of::<T>(),
            meta: FieldMeta::new(),
        }
    }

    /// Add metadata for this field.
    pub fn with_meta(self, meta: FieldMeta) -> Self {
        Self { meta, ..self }
    }

    /// Returns the index of the field.
    pub fn index(&self) -> usize {
        self.index
    }

    /// The [type name] of the field.
    ///
    /// [type name]: std::any::type_name
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// The [`TypeId`] of the field.
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// The metadata of the field.
    pub fn meta(&self) -> &FieldMeta {
        &self.meta
    }

    /// Check if the given type matches the field type.
    pub fn is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }
}

/// Metadata for both [named] and [unnamed] fields.
///
/// [named]: NamedField
/// [unnamed]: UnnamedField
#[derive(Clone, Debug)]
pub struct FieldMeta {
    /// This field should not be used in its container's [`Reflect::reflect_hash`] implementation.
    ///
    /// This may be configured when [deriving `Reflect`] by adding `#[reflect(skip_hash)]` to the field.
    ///
    /// [deriving `Reflect`]: bevy_reflect_derive::Reflect
    pub skip_hash: bool,
    /// This field should not be used in its container's [`Reflect::reflect_partial_eq`] implementation.
    ///
    /// This may be configured when [deriving `Reflect`] by adding `#[reflect(skip_partial_eq)]` to the field.
    ///
    /// [deriving `Reflect`]: bevy_reflect_derive::Reflect
    pub skip_partial_eq: bool,
    /// The docstring of this field, if any.
    #[cfg(feature = "documentation")]
    pub docs: Option<&'static str>,
}

impl FieldMeta {
    pub const fn new() -> Self {
        Self {
            skip_hash: false,
            skip_partial_eq: false,
            #[cfg(feature = "documentation")]
            docs: None,
        }
    }
}

impl Default for FieldMeta {
    fn default() -> Self {
        Self::new()
    }
}
