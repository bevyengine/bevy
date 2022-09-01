use crate::Reflect;
use std::any::{Any, TypeId};

/// The named field of a reflected struct.
#[derive(Clone, Debug)]
pub struct NamedField {
    name: &'static str,
    type_name: &'static str,
    type_id: TypeId,
}

impl NamedField {
    /// Create a new [`NamedField`].
    pub fn new<T: Reflect>(name: &'static str) -> Self {
        Self {
            name,
            type_name: std::any::type_name::<T>(),
            type_id: TypeId::of::<T>(),
        }
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
}

impl UnnamedField {
    pub fn new<T: Reflect>(index: usize) -> Self {
        Self {
            index,
            type_name: std::any::type_name::<T>(),
            type_id: TypeId::of::<T>(),
        }
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

    /// Check if the given type matches the field type.
    pub fn is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }
}
