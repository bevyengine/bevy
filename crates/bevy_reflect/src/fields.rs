use crate::Reflect;
use std::any::TypeId;
use std::borrow::{Borrow, Cow};

/// The named field of a reflected struct
#[derive(Clone, Debug)]
pub struct NamedField {
    name: Cow<'static, str>,
    type_name: Cow<'static, str>,
    type_id: TypeId,
}

impl NamedField {
    pub fn new<T: Reflect>(name: &str) -> Self {
        Self {
            name: Cow::Owned(name.into()),
            type_name: Cow::Owned(std::any::type_name::<T>().to_string()),
            type_id: TypeId::of::<T>(),
        }
    }

    /// The name of the field
    pub fn name(&self) -> &str {
        self.name.borrow()
    }

    /// The type name of the field
    pub fn type_name(&self) -> &str {
        self.type_name.borrow()
    }

    /// The `TypeId` of the field
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Check if the given type matches this field's type
    pub fn is<T: Reflect>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }
}

/// The unnamed field of a reflected tuple or tuple struct
#[derive(Clone, Debug)]
pub struct UnnamedField {
    index: usize,
    type_name: Cow<'static, str>,
    type_id: TypeId,
}

impl UnnamedField {
    pub fn new<T: Reflect>(index: usize) -> Self {
        Self {
            index,
            type_name: Cow::Owned(std::any::type_name::<T>().to_string()),
            type_id: TypeId::of::<T>(),
        }
    }

    /// Returns the index of the field
    pub fn index(&self) -> usize {
        self.index
    }

    /// The type name of the field
    pub fn type_name(&self) -> &str {
        self.type_name.borrow()
    }

    /// The `TypeId` of the field
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Check if the given type matches this field's type
    pub fn is<T: Reflect>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }
}
