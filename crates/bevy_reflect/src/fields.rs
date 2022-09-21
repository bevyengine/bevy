use crate::{type_path, Reflect, TypePath};
use std::any::{Any, TypeId};

/// The named field of a reflected struct.
#[derive(Clone, Debug)]
pub struct NamedField {
    name: &'static str,
    type_path: &'static str,
    type_id: TypeId,
}

impl NamedField {
    /// Create a new [`NamedField`].
    pub fn new<T: Reflect + TypePath>(name: &'static str) -> Self {
        Self {
            name,
            type_path: type_path::<T>(),
            type_id: TypeId::of::<T>(),
        }
    }

    /// The name of the field.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The [type path] of the field.
    ///
    /// [type path]: TypePath
    pub fn type_path(&self) -> &'static str {
        self.type_path
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
    type_path: &'static str,
    type_id: TypeId,
}

impl UnnamedField {
    pub fn new<T: Reflect + TypePath>(index: usize) -> Self {
        Self {
            index,
            type_path: type_path::<T>(),
            type_id: TypeId::of::<T>(),
        }
    }

    /// Returns the index of the field.
    pub fn index(&self) -> usize {
        self.index
    }

    /// The [type path] of the field.
    ///
    /// [type path]: TypePath
    pub fn type_path(&self) -> &'static str {
        self.type_path
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
