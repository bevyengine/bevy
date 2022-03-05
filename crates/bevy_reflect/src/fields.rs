use crate::{Reflect, TypeIdentity};
use std::borrow::{Borrow, Cow};

/// The named field of a reflected struct
#[derive(Clone, Debug)]
pub struct NamedField {
    name: Cow<'static, str>,
    id: TypeIdentity,
}

impl NamedField {
    pub fn new<T: Reflect>(name: &str) -> Self {
        Self {
            name: Cow::Owned(name.into()),
            id: TypeIdentity::of::<T>(),
        }
    }

    /// The name of the field
    pub fn name(&self) -> &str {
        self.name.borrow()
    }

    /// The [`TypeIdentity`] of the field
    pub fn id(&self) -> &TypeIdentity {
        &self.id
    }
}

/// The unnamed field of a reflected tuple or tuple struct
#[derive(Clone, Debug)]
pub struct UnnamedField {
    index: usize,
    id: TypeIdentity,
}

impl UnnamedField {
    pub fn new<T: Reflect>(index: usize) -> Self {
        Self {
            index,
            id: TypeIdentity::of::<T>(),
        }
    }

    /// Returns the index of the field
    pub fn index(&self) -> usize {
        self.index
    }

    /// The [`TypeIdentity`] of the field
    pub fn id(&self) -> &TypeIdentity {
        &self.id
    }
}
