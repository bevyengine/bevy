use crate::{Reflect, TypeIdentity};
use std::borrow::Cow;

/// The named field of a reflected struct
#[derive(Clone, Debug)]
pub struct NamedField {
    name: Cow<'static, str>,
    id: TypeIdentity,
}

impl NamedField {
    /// Create a new [`NamedField`]
    pub fn new<T: Reflect>(name: &str) -> Self {
        Self {
            name: Cow::Owned(name.into()),
            id: TypeIdentity::of::<T>(),
        }
    }

    /// Create a new [`NamedField`] using a static string
    ///
    /// This helps save an allocation when the string has a static lifetime, such
    /// as when using [`std::any::type_name`].
    pub fn static_new<T: Reflect>(name: &'static str) -> Self {
        Self {
            name: Cow::Borrowed(name),
            id: TypeIdentity::of::<T>(),
        }
    }

    /// The name of the field
    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
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
